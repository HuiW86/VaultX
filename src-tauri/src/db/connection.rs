use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

use super::schema;

pub const META_FILENAME: &str = ".vaultx-meta";
/// DB file of vaults created as v2 and of every v1 vault.
pub const DB_FILENAME: &str = "vault.db";
/// DB file produced by the v1→v2 migration (contract C8). The migration
/// commits by atomically switching `.vaultx-meta` to point at this file.
pub const MIGRATED_DB_FILENAME: &str = "vault.v2.db";
/// Every DB filename `.vaultx-meta` may reference.
pub const KNOWN_DB_FILENAMES: &[&str] = &[DB_FILENAME, MIGRATED_DB_FILENAME];
const BUSY_TIMEOUT_MS: u32 = 5000;

/// Legacy meta version: the password-derived key encrypted the DB directly.
pub const META_VERSION_V1: u32 = 1;
/// Current meta version: DEK/KEK hierarchy.
pub const META_VERSION_V2: u32 = 2;

/// Metadata file stored alongside the encrypted DB.
/// Contains KDF params and the wrapped DEK needed before opening the DB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMeta {
    pub version: u32,
    pub kdf: KdfConfig,
    pub created_at: String,
    /// DB filename relative to the data dir; must be one of KNOWN_DB_FILENAMES.
    pub db_path: String,
    /// v1 only: base64 AES-GCM blob of the legacy master key, encrypted by
    /// the recovery KEK. Never written by v2 code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_blob: Option<String>,
    /// v2: base64 DEK wrapped by the password KEK (purpose `vaultx:dek:password`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dek_wrapped_by_password: Option<String>,
    /// v2: base64 DEK wrapped by the recovery KEK (purpose `vaultx:dek:recovery`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dek_wrapped_by_recovery: Option<String>,
    /// v2: set in the same atomic meta write that commits a recovery or a
    /// v1→v2 migration. While set, the Keychain copy must be deleted and
    /// `touch_id_enabled` cleared before Touch ID may be used again (C6).
    #[serde(default, skip_serializing_if = "is_false")]
    pub keychain_invalidation_pending: bool,
}

fn is_false(v: &bool) -> bool {
    !*v
}

impl VaultMeta {
    /// Build a fresh v2 meta.
    pub fn new_v2(salt: &[u8], db_path: &str, created_at: String, dek_wrapped_by_password: String) -> Self {
        VaultMeta {
            version: META_VERSION_V2,
            kdf: KdfConfig::argon2id(salt),
            created_at,
            db_path: db_path.to_string(),
            recovery_blob: None,
            dek_wrapped_by_password: Some(dek_wrapped_by_password),
            dek_wrapped_by_recovery: None,
            keychain_invalidation_pending: false,
        }
    }

    /// Validated DB filename referenced by this meta.
    pub fn db_filename(&self) -> Result<&'static str, String> {
        KNOWN_DB_FILENAMES
            .iter()
            .copied()
            .find(|n| *n == self.db_path)
            .ok_or_else(|| "Meta references an unknown database file".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfConfig {
    pub algorithm: String,
    pub params: KdfParams,
    #[serde(with = "base64_serde")]
    pub salt: Vec<u8>,
}

impl KdfConfig {
    pub fn argon2id(salt: &[u8]) -> Self {
        KdfConfig {
            algorithm: "argon2id".to_string(),
            params: KdfParams { m_cost: 19456, t_cost: 2, p_cost: 1 },
            salt: salt.to_vec(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfParams {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

mod base64_serde {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

/// App status based on file presence and consistency.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppFileStatus {
    FirstRun,
    Ready,
    Corrupted { reason: String },
}

/// Resolve the data directory path.
/// macOS: ~/Library/Application Support/com.vaultx.app/
pub fn data_dir() -> Result<PathBuf, String> {
    let dir = dirs_next()
        .ok_or_else(|| "Cannot determine application support directory".to_string())?;
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create data directory: {e}"))?;
    Ok(dir)
}

fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::data_dir().map(|d| d.join("com.vaultx.app"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs::data_dir().map(|d| d.join("vaultx"))
    }
}

/// Check file status: meta and the DB it references present, neither, or inconsistent.
pub fn check_file_status(base_dir: &Path) -> AppFileStatus {
    let meta_exists = base_dir.join(META_FILENAME).exists();
    let any_db_exists = KNOWN_DB_FILENAMES.iter().any(|n| base_dir.join(n).exists());

    if !meta_exists {
        return if any_db_exists {
            AppFileStatus::Corrupted {
                reason: "Database exists but meta file is missing".to_string(),
            }
        } else {
            AppFileStatus::FirstRun
        };
    }

    let meta = match read_meta(base_dir) {
        Ok(m) => m,
        Err(e) => {
            return AppFileStatus::Corrupted {
                reason: format!("Meta file unreadable: {e}"),
            }
        }
    };
    match meta.db_filename() {
        Ok(name) if base_dir.join(name).exists() => AppFileStatus::Ready,
        Ok(_) => AppFileStatus::Corrupted {
            reason: "Meta file exists but database is missing".to_string(),
        },
        Err(e) => AppFileStatus::Corrupted { reason: e },
    }
}

/// Write meta file atomically (write tmp → fsync → rename → fsync dir). C3.
pub fn write_meta(base_dir: &Path, meta: &VaultMeta) -> Result<(), String> {
    write_meta_with_hook(base_dir, meta, &|_| Ok(()))
}

/// Same as `write_meta`, calling `hook("meta:tmp_written")` between the temp
/// write and the committing rename (used for fault injection).
pub fn write_meta_with_hook(
    base_dir: &Path,
    meta: &VaultMeta,
    hook: &dyn Fn(&'static str) -> Result<(), String>,
) -> Result<(), String> {
    let meta_path = base_dir.join(META_FILENAME);
    let tmp_path = base_dir.join(format!("{META_FILENAME}.tmp"));

    let json = serde_json::to_string_pretty(meta)
        .map_err(|e| format!("Failed to serialize meta: {e}"))?;
    {
        use std::io::Write;
        let mut f = fs::File::create(&tmp_path).map_err(|e| format!("Failed to write temp meta: {e}"))?;
        f.write_all(json.as_bytes()).map_err(|e| format!("Failed to write temp meta: {e}"))?;
        f.sync_all().map_err(|e| format!("Failed to sync temp meta: {e}"))?;
    }
    hook("meta:tmp_written")?;
    fs::rename(&tmp_path, &meta_path).map_err(|e| format!("Failed to rename meta: {e}"))?;
    sync_dir(base_dir);
    Ok(())
}

/// Best-effort directory fsync so a rename is durable before later steps.
pub fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    {
        if let Ok(d) = fs::File::open(dir) {
            let _ = d.sync_all();
        }
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
    }
}

/// Read and parse the meta file.
pub fn read_meta(base_dir: &Path) -> Result<VaultMeta, String> {
    let meta_path = base_dir.join(META_FILENAME);
    let json = fs::read_to_string(&meta_path)
        .map_err(|e| format!("Failed to read meta file: {e}"))?;
    serde_json::from_str(&json).map_err(|e| format!("Failed to parse meta file: {e}"))
}

/// Initialize a new encrypted database at `vault.db`.
/// Creates the DB file, sets SQLCipher key, runs migrations, returns connection.
pub fn init_db(base_dir: &Path, key: &[u8; 32]) -> Result<Connection, String> {
    let db_path = base_dir.join(DB_FILENAME);
    if db_path.exists() {
        return Err("Database already exists".to_string());
    }

    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to create database: {e}"))?;
    configure_connection(&conn, key)?;
    schema::run_migrations(&conn)?;
    Ok(conn)
}

/// Path of the vault DB: the file referenced by `.vaultx-meta`, or
/// `vault.db` when no meta exists yet (e.g. during setup).
pub fn db_file_path(base_dir: &Path) -> Result<PathBuf, String> {
    if base_dir.join(META_FILENAME).exists() {
        let meta = read_meta(base_dir)?;
        Ok(base_dir.join(meta.db_filename()?))
    } else {
        Ok(base_dir.join(DB_FILENAME))
    }
}

/// Open the existing encrypted vault database.
pub fn open_db(base_dir: &Path, key: &[u8; 32]) -> Result<Connection, String> {
    open_db_file(&db_file_path(base_dir)?, key)
}

/// Open an encrypted database file and verify the key.
pub fn open_db_file(db_path: &Path, key: &[u8; 32]) -> Result<Connection, String> {
    if !db_path.exists() {
        return Err("Database file not found".to_string());
    }

    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open database: {e}"))?;
    // With a wrong key the first statement touching the file fails.
    configure_connection(&conn, key)
        .map_err(|_| "Wrong encryption key or corrupted database".to_string())?;

    // Verify the key is correct by running a simple query
    conn.execute_batch("SELECT count(*) FROM sqlite_master;")
        .map_err(|_| "Wrong encryption key or corrupted database".to_string())?;

    Ok(conn)
}

/// Configure SQLCipher connection: set key, page size, busy timeout.
fn configure_connection(conn: &Connection, key: &[u8; 32]) -> Result<(), String> {
    let pragma = Zeroizing::new(format!("PRAGMA key = \"x'{}'\";", hex_encode(key).as_str()));
    // Never include the underlying error: it could echo the statement text (C4).
    conn.execute_batch(&pragma)
        .map_err(|_| "Failed to set encryption key".to_string())?;
    conn.execute_batch("PRAGMA cipher_page_size = 4096;")
        .map_err(|e| format!("Failed to set cipher page size: {e}"))?;
    conn.busy_timeout(std::time::Duration::from_millis(BUSY_TIMEOUT_MS as u64))
        .map_err(|e| format!("Failed to set busy timeout: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")
        .map_err(|e| format!("Failed to set WAL mode: {e}"))?;
    Ok(())
}

/// Hex-encode key material into a zeroizing string.
pub fn hex_encode(bytes: &[u8]) -> Zeroizing<String> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = Zeroizing::new(String::with_capacity(bytes.len() * 2));
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// Remove a DB file together with its SQLite side files.
pub fn remove_db_files(db_path: &Path) {
    let _ = fs::remove_file(db_path);
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut p = db_path.as_os_str().to_owned();
        p.push(suffix);
        let _ = fs::remove_file(PathBuf::from(p));
    }
}

/// Remove all vault files (used for cleanup after interrupted setup).
pub fn cleanup_files(base_dir: &Path) {
    for name in KNOWN_DB_FILENAMES {
        remove_db_files(&base_dir.join(name));
    }
    let _ = fs::remove_file(base_dir.join(META_FILENAME));
    let _ = fs::remove_file(base_dir.join(format!("{META_FILENAME}.tmp")));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_key() -> [u8; 32] {
        [0xAA; 32]
    }

    #[test]
    fn first_run_detection() {
        let dir = TempDir::new().unwrap();
        assert_eq!(check_file_status(dir.path()), AppFileStatus::FirstRun);
    }

    #[test]
    fn init_and_open_roundtrip() {
        let dir = TempDir::new().unwrap();
        let key = test_key();

        // Init
        let conn = init_db(dir.path(), &key).unwrap();
        drop(conn);

        // Write meta
        let meta = VaultMeta {
            version: 1,
            kdf: KdfConfig {
                algorithm: "argon2id".to_string(),
                params: KdfParams {
                    m_cost: 19456,
                    t_cost: 2,
                    p_cost: 1,
                },
                salt: vec![1, 2, 3],
            },
            created_at: "2026-01-01T00:00:00Z".to_string(),
            db_path: "vault.db".to_string(),
            recovery_blob: None,
            dek_wrapped_by_password: None,
            dek_wrapped_by_recovery: None,
            keychain_invalidation_pending: false,
        };
        write_meta(dir.path(), &meta).unwrap();

        // Check status
        assert_eq!(check_file_status(dir.path()), AppFileStatus::Ready);

        // Re-open
        let conn = open_db(dir.path(), &key).unwrap();
        drop(conn);
    }

    #[test]
    fn wrong_key_fails() {
        let dir = TempDir::new().unwrap();
        let key = test_key();
        let conn = init_db(dir.path(), &key).unwrap();
        drop(conn);

        let wrong_key = [0xBB; 32];
        assert!(open_db(dir.path(), &wrong_key).is_err());
    }

    #[test]
    fn init_existing_fails() {
        let dir = TempDir::new().unwrap();
        let key = test_key();
        let _ = init_db(dir.path(), &key).unwrap();
        assert!(init_db(dir.path(), &key).is_err());
    }

    #[test]
    fn meta_db_inconsistency_detected() {
        let dir = TempDir::new().unwrap();
        // Create only meta, no DB
        let meta = VaultMeta {
            version: 1,
            kdf: KdfConfig {
                algorithm: "argon2id".to_string(),
                params: KdfParams {
                    m_cost: 19456,
                    t_cost: 2,
                    p_cost: 1,
                },
                salt: vec![1, 2, 3],
            },
            created_at: "2026-01-01T00:00:00Z".to_string(),
            db_path: "vault.db".to_string(),
            recovery_blob: None,
            dek_wrapped_by_password: None,
            dek_wrapped_by_recovery: None,
            keychain_invalidation_pending: false,
        };
        write_meta(dir.path(), &meta).unwrap();

        match check_file_status(dir.path()) {
            AppFileStatus::Corrupted { reason } => {
                assert!(reason.contains("missing"));
            }
            other => panic!("Expected Corrupted, got {other:?}"),
        }
    }

    #[test]
    fn hex_encode_matches_expected() {
        assert_eq!(hex_encode(&[0x00, 0xab, 0xff]).as_str(), "00abff");
    }

    #[test]
    fn unknown_db_path_is_rejected() {
        let dir = TempDir::new().unwrap();
        let mut meta = VaultMeta::new_v2(&[1], "vault.db", "t".into(), "w".into());
        meta.db_path = "../elsewhere.db".into();
        write_meta(dir.path(), &meta).unwrap();
        assert!(matches!(check_file_status(dir.path()), AppFileStatus::Corrupted { .. }));
        assert!(open_db(dir.path(), &test_key()).is_err());
    }

    #[test]
    fn cleanup_removes_all_files() {
        let dir = TempDir::new().unwrap();
        let key = test_key();
        let _ = init_db(dir.path(), &key).unwrap();
        let meta = VaultMeta {
            version: 1,
            kdf: KdfConfig {
                algorithm: "argon2id".to_string(),
                params: KdfParams { m_cost: 19456, t_cost: 2, p_cost: 1 },
                salt: vec![1],
            },
            created_at: "2026-01-01T00:00:00Z".to_string(),
            db_path: "vault.db".to_string(),
            recovery_blob: None,
            dek_wrapped_by_password: None,
            dek_wrapped_by_recovery: None,
            keychain_invalidation_pending: false,
        };
        write_meta(dir.path(), &meta).unwrap();

        cleanup_files(dir.path());
        assert_eq!(check_file_status(dir.path()), AppFileStatus::FirstRun);
    }
}
