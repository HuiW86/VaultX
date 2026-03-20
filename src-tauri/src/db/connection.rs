use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use super::schema;

const META_FILENAME: &str = ".vaultx-meta";
const DB_FILENAME: &str = "vault.db";
const BUSY_TIMEOUT_MS: u32 = 5000;

/// Metadata file stored alongside the encrypted DB.
/// Contains KDF params needed to derive the key before opening the DB.
#[derive(Debug, Serialize, Deserialize)]
pub struct VaultMeta {
    pub version: u32,
    pub kdf: KdfConfig,
    pub created_at: String,
    pub db_path: String,
    /// Base64-encoded AES-GCM encrypted master_key (encrypted by recovery key)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_blob: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KdfConfig {
    pub algorithm: String,
    pub params: KdfParams,
    #[serde(with = "base64_serde")]
    pub salt: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
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

/// Check file status: both files present, neither, or inconsistent.
pub fn check_file_status(base_dir: &Path) -> AppFileStatus {
    let meta_path = base_dir.join(META_FILENAME);
    let db_path = base_dir.join(DB_FILENAME);
    let meta_exists = meta_path.exists();
    let db_exists = db_path.exists();

    match (meta_exists, db_exists) {
        (false, false) => AppFileStatus::FirstRun,
        (true, true) => {
            // Validate meta file is readable
            match read_meta(base_dir) {
                Ok(_) => AppFileStatus::Ready,
                Err(e) => AppFileStatus::Corrupted {
                    reason: format!("Meta file unreadable: {e}"),
                },
            }
        }
        (true, false) => AppFileStatus::Corrupted {
            reason: "Meta file exists but database is missing".to_string(),
        },
        (false, true) => AppFileStatus::Corrupted {
            reason: "Database exists but meta file is missing".to_string(),
        },
    }
}

/// Write meta file atomically (write tmp → rename).
pub fn write_meta(base_dir: &Path, meta: &VaultMeta) -> Result<(), String> {
    let meta_path = base_dir.join(META_FILENAME);
    let tmp_path = base_dir.join(format!("{META_FILENAME}.tmp"));

    let json = serde_json::to_string_pretty(meta)
        .map_err(|e| format!("Failed to serialize meta: {e}"))?;
    fs::write(&tmp_path, &json).map_err(|e| format!("Failed to write temp meta: {e}"))?;
    fs::rename(&tmp_path, &meta_path).map_err(|e| format!("Failed to rename meta: {e}"))?;
    Ok(())
}

/// Read and parse the meta file.
pub fn read_meta(base_dir: &Path) -> Result<VaultMeta, String> {
    let meta_path = base_dir.join(META_FILENAME);
    let json = fs::read_to_string(&meta_path)
        .map_err(|e| format!("Failed to read meta file: {e}"))?;
    serde_json::from_str(&json).map_err(|e| format!("Failed to parse meta file: {e}"))
}

/// Initialize a new encrypted database.
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

/// Open an existing encrypted database.
pub fn open_db(base_dir: &Path, key: &[u8; 32]) -> Result<Connection, String> {
    let db_path = base_dir.join(DB_FILENAME);
    if !db_path.exists() {
        return Err("Database file not found".to_string());
    }

    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {e}"))?;
    configure_connection(&conn, key)?;

    // Verify the key is correct by running a simple query
    conn.execute_batch("SELECT count(*) FROM sqlite_master;")
        .map_err(|_| "Wrong encryption key or corrupted database".to_string())?;

    Ok(conn)
}

/// Configure SQLCipher connection: set key, page size, busy timeout.
fn configure_connection(conn: &Connection, key: &[u8; 32]) -> Result<(), String> {
    let hex_key = hex_encode(key);
    conn.execute_batch(&format!("PRAGMA key = \"x'{hex_key}'\";"))
        .map_err(|e| format!("Failed to set encryption key: {e}"))?;
    conn.execute_batch("PRAGMA cipher_page_size = 4096;")
        .map_err(|e| format!("Failed to set cipher page size: {e}"))?;
    conn.busy_timeout(std::time::Duration::from_millis(BUSY_TIMEOUT_MS as u64))
        .map_err(|e| format!("Failed to set busy timeout: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")
        .map_err(|e| format!("Failed to set WAL mode: {e}"))?;
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Remove all vault files (used for cleanup after interrupted setup).
pub fn cleanup_files(base_dir: &Path) {
    let _ = fs::remove_file(base_dir.join(DB_FILENAME));
    let _ = fs::remove_file(base_dir.join(META_FILENAME));
    let _ = fs::remove_file(base_dir.join(format!("{META_FILENAME}.tmp")));
    // WAL and SHM files from SQLite
    let _ = fs::remove_file(base_dir.join(format!("{DB_FILENAME}-wal")));
    let _ = fs::remove_file(base_dir.join(format!("{DB_FILENAME}-shm")));
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
        };
        write_meta(dir.path(), &meta).unwrap();

        cleanup_files(dir.path());
        assert_eq!(check_file_status(dir.path()), AppFileStatus::FirstRun);
    }
}
