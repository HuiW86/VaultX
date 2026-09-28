//! v1 → v2 migration (contract C8).
//!
//! States, all restart-safe:
//! 1. `vault.db` + v1 meta, possibly a partial `vault.v2.db` shadow. The
//!    shadow is garbage; `reconcile` deletes it. Old password (and v1
//!    recovery key) keep working.
//! 2. Shadow complete, re-encrypted and verified; v1 meta still current.
//!    Same as 1 on restart (the shadow is rebuilt next time).
//! 3. Commit: one atomic rename of `.vaultx-meta` to a v2 meta that points to
//!    `vault.v2.db` and wraps the new DEK with the password KEK. The shadow's
//!    directory entry is fsynced before, and the directory after, the rename;
//!    any sync error aborts the flow with `vault.db` kept.
//! 4. Post-commit: `vault.db` is deleted only once the v2 meta is confirmed
//!    durable and on disk and the new DB reopens with the DEK, has the full
//!    schema and the migrated row counts (`verify_migrated_db`,
//!    `remove_superseded_db`). If anything fails, `vault.db` is kept and the
//!    next keyed open (password, Touch ID, recovery) retries the removal.
//!    `reconcile` never deletes it, since it has no key to verify the new DB.
//!
//! The original DB is never modified. The whole flow runs under the
//! cross-process `VaultLock` (C10).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};
use zeroize::Zeroizing;

use super::lock::VaultLock;
use super::FaultHook;
use crate::crypto::{encryption, key_derivation, key_wrap};
use crate::db::connection::{
    self, VaultMeta, DB_FILENAME, META_VERSION_V1, META_VERSION_V2, MIGRATED_DB_FILENAME,
};

/// Row counts of every table that holds user data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCounts {
    pub vaults: i64,
    pub entries: i64,
    pub trashed_entries: i64,
    pub fields: i64,
    pub password_history: i64,
}

/// Every field value and history value, decrypted. Sensitive field values
/// and history values are decrypted with `key`; other field values are raw.
pub struct VaultPlaintexts {
    pub fields: BTreeMap<String, Zeroizing<Vec<u8>>>,
    pub history: BTreeMap<String, Zeroizing<Vec<u8>>>,
    pub counts: TableCounts,
}

impl PartialEq for VaultPlaintexts {
    fn eq(&self, other: &Self) -> bool {
        self.counts == other.counts
            && self.fields.len() == other.fields.len()
            && self.history.len() == other.history.len()
            && self.fields.iter().zip(other.fields.iter()).all(|(a, b)| a.0 == b.0 && *a.1 == *b.1)
            && self.history.iter().zip(other.history.iter()).all(|(a, b)| a.0 == b.0 && *a.1 == *b.1)
    }
}

impl std::fmt::Debug for VaultPlaintexts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print plaintexts.
        f.debug_struct("VaultPlaintexts").field("counts", &self.counts).finish_non_exhaustive()
    }
}

fn count(conn: &Connection, sql: &str) -> Result<i64, String> {
    conn.query_row(sql, [], |r| r.get(0)).map_err(|e| format!("Count failed: {e}"))
}

/// Decrypt every field (all entries, trashed included) and every
/// password_history row. Fails if any single value does not decrypt (C7).
pub fn collect_plaintexts(conn: &Connection, key: &[u8; 32]) -> Result<VaultPlaintexts, String> {
    let mut fields = BTreeMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, field_type, value FROM fields")
            .map_err(|e| format!("Query failed: {e}"))?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Vec<u8>>(2)?)))
            .map_err(|e| format!("Query failed: {e}"))?;
        for row in rows {
            let (id, field_type, value) = row.map_err(|e| format!("Query failed: {e}"))?;
            let plain = if encryption::is_sensitive_field_type(&field_type) {
                encryption::decrypt(key, &value)
                    .map_err(|_| "A sensitive field could not be decrypted".to_string())?
            } else {
                value
            };
            fields.insert(id, Zeroizing::new(plain));
        }
    }

    let mut history = BTreeMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, value FROM password_history")
            .map_err(|e| format!("Query failed: {e}"))?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?)))
            .map_err(|e| format!("Query failed: {e}"))?;
        for row in rows {
            let (id, value) = row.map_err(|e| format!("Query failed: {e}"))?;
            let plain = encryption::decrypt(key, &value)
                .map_err(|_| "A password history value could not be decrypted".to_string())?;
            history.insert(id, Zeroizing::new(plain));
        }
    }

    let counts = TableCounts {
        vaults: count(conn, "SELECT count(*) FROM vaults")?,
        entries: count(conn, "SELECT count(*) FROM entries")?,
        trashed_entries: count(conn, "SELECT count(*) FROM entries WHERE trashed = 1")?,
        fields: count(conn, "SELECT count(*) FROM fields")?,
        password_history: count(conn, "SELECT count(*) FROM password_history")?,
    };

    Ok(VaultPlaintexts { fields, history, counts })
}

/// Re-encrypt every sensitive field and history value from `old_key` to `new_key`.
fn reencrypt(conn: &mut Connection, old_key: &[u8; 32], new_key: &[u8; 32]) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| format!("Transaction failed: {e}"))?;
    {
        let rows: Vec<(String, String, Vec<u8>)> = {
            let mut stmt = tx
                .prepare("SELECT id, field_type, value FROM fields")
                .map_err(|e| format!("Query failed: {e}"))?;
            let mapped = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map_err(|e| format!("Query failed: {e}"))?;
            mapped.collect::<Result<_, _>>().map_err(|e| format!("Query failed: {e}"))?
        };
        for (id, field_type, value) in rows {
            if !encryption::is_sensitive_field_type(&field_type) {
                continue;
            }
            let plain = Zeroizing::new(
                encryption::decrypt(old_key, &value)
                    .map_err(|_| "A sensitive field could not be decrypted".to_string())?,
            );
            let fresh = encryption::encrypt(new_key, &plain)?;
            tx.execute("UPDATE fields SET value = ?1 WHERE id = ?2", params![fresh, id])
                .map_err(|e| format!("Update failed: {e}"))?;
        }

        let rows: Vec<(String, Vec<u8>)> = {
            let mut stmt = tx
                .prepare("SELECT id, value FROM password_history")
                .map_err(|e| format!("Query failed: {e}"))?;
            let mapped = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(|e| format!("Query failed: {e}"))?;
            mapped.collect::<Result<_, _>>().map_err(|e| format!("Query failed: {e}"))?
        };
        for (id, value) in rows {
            let plain = Zeroizing::new(
                encryption::decrypt(old_key, &value)
                    .map_err(|_| "A password history value could not be decrypted".to_string())?,
            );
            let fresh = encryption::encrypt(new_key, &plain)?;
            tx.execute("UPDATE password_history SET value = ?1 WHERE id = ?2", params![fresh, id])
                .map_err(|e| format!("Update failed: {e}"))?;
        }
    }
    tx.commit().map_err(|e| format!("Commit failed: {e}"))
}

/// Migrate a v1 vault to v2. `legacy_key` must open the v1 DB and decrypt
/// its fields; `password` becomes the password wrapping the new DEK.
/// Returns the new DEK once the migration has committed and the legacy DB
/// has been removed. Takes the vault lock.
pub fn migrate_v1(
    dir: &Path,
    legacy_key: &[u8; 32],
    password: &[u8],
    hook: FaultHook,
) -> Result<Zeroizing<[u8; 32]>, String> {
    let lock = VaultLock::acquire(dir)?;
    migrate_v1_locked(&lock, dir, legacy_key, password, hook)
}

/// `migrate_v1` for callers already holding the vault lock.
pub(crate) fn migrate_v1_locked(
    lock: &VaultLock,
    dir: &Path,
    legacy_key: &[u8; 32],
    password: &[u8],
    hook: FaultHook,
) -> Result<Zeroizing<[u8; 32]>, String> {
    let meta = connection::read_meta(dir)?;
    if meta.version != META_VERSION_V1 || meta.db_filename()? != DB_FILENAME {
        return Err("Vault is not a v1 vault".to_string());
    }
    let src_path = dir.join(DB_FILENAME);
    let shadow_path = dir.join(MIGRATED_DB_FILENAME);
    connection::remove_db_files(&shadow_path);

    // State 1: build the shadow DB keyed with a new DEK.
    let src = connection::open_db_file(&src_path, legacy_key)?;
    let expected = collect_plaintexts(&src, legacy_key)?;
    let dek = key_wrap::generate_dek();

    hook("migrate:before_export")?;
    {
        let shadow_str = shadow_path
            .to_str()
            .ok_or_else(|| "Data directory path is not valid UTF-8".to_string())?;
        let key_literal = Zeroizing::new(format!("x'{}'", connection::hex_encode(&*dek).as_str()));
        // Bound parameters keep the key out of SQL text and error messages (C4).
        src.execute("ATTACH DATABASE ?1 AS shadow KEY ?2", params![shadow_str, key_literal.as_str()])
            .map_err(|_| "Failed to create migration database".to_string())?;
        src.query_row("SELECT sqlcipher_export('shadow')", [], |_| Ok(()))
            .map_err(|_| "Failed to export vault for migration".to_string())?;
        src.execute("DETACH DATABASE shadow", [])
            .map_err(|_| "Failed to finish migration export".to_string())?;
    }
    drop(src);
    hook("migrate:exported")?;

    let mut shadow = connection::open_db_file(&shadow_path, &dek)?;
    reencrypt(&mut shadow, legacy_key, &dek)?;
    hook("migrate:reencrypted")?;
    shadow
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| format!("Checkpoint failed: {e}"))?;
    drop(shadow);
    fs::File::open(&shadow_path)
        .and_then(|f| f.sync_all())
        .map_err(|e| format!("Failed to sync migration database: {e}"))?;
    // The shadow's directory entry must be durable before meta points to it.
    connection::sync_dir(dir)?;

    // State 2: verify the shadow reopens and every value decrypts to the original.
    {
        let verify = connection::open_db_file(&shadow_path, &dek)?;
        let check: String = verify
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .map_err(|e| format!("Integrity check failed: {e}"))?;
        if check != "ok" {
            return Err("Migrated database failed integrity check".to_string());
        }
        let actual = collect_plaintexts(&verify, &dek)?;
        if actual != expected {
            return Err("Migrated data does not match the original".to_string());
        }
    }
    hook("migrate:verified")?;

    // State 3: commit by atomically switching meta to the v2 DB.
    let salt = key_derivation::generate_salt();
    let kek = key_derivation::derive_key(password, &salt)?;
    let wrapped = key_wrap::wrap_dek(&kek, &dek, key_wrap::PURPOSE_PASSWORD)?;
    let mut new_meta = VaultMeta::new_v2(&salt, MIGRATED_DB_FILENAME, meta.created_at.clone(), wrapped);
    // The Keychain may hold the legacy key; it must be removed (C6).
    new_meta.keychain_invalidation_pending = true;
    // Errors here (including a failed directory fsync after the rename)
    // abort with `vault.db` untouched.
    connection::write_meta_with_hook(dir, &new_meta, hook)?;
    hook("migrate:committed")?;

    // State 4: drop the legacy DB, but only after re-verifying the commit.
    let on_disk = connection::read_meta(dir)?;
    if on_disk.version != META_VERSION_V2
        || on_disk.db_path != MIGRATED_DB_FILENAME
        || on_disk.dek_wrapped_by_password != new_meta.dek_wrapped_by_password
    {
        return Err("Migration commit could not be confirmed; the original database was kept".to_string());
    }
    // "Opens with the DEK" is not enough (a 0-byte file opens with any key):
    // the committed DB must hold the schema and exactly the migrated rows.
    connection::open_db(dir, &dek)
        .and_then(|conn| verify_migrated_db(&conn, &dek, Some(&expected.counts)))
        .map_err(|_| "Migrated database could not be verified; the original database was kept".to_string())?;
    hook("migrate:before_legacy_delete")?;
    if !remove_superseded_db(lock, dir, &dek, Some(&expected.counts)) {
        log::warn!("Legacy database kept after migration; it will be removed on a later unlock");
    }
    Ok(dek)
}

/// Tables that must exist in a migrated DB before the legacy DB may go.
const REQUIRED_TABLES: [&str; 4] = ["vaults", "entries", "fields", "password_history"];

/// Check that `conn` (the committed `vault.v2.db`) really holds the vault:
/// every required table exists, the integrity check passes and every
/// sensitive field and history value decrypts with `dek`. With `expected`
/// (right after the migration) all row counts must equal the source DB's;
/// without it (a later retry, when the user may have edited entries) the DB
/// must at least contain a vault row. A 0-byte or freshly created file fails.
pub(crate) fn verify_migrated_db(
    conn: &Connection,
    dek: &[u8; 32],
    expected: Option<&TableCounts>,
) -> Result<(), String> {
    for table in REQUIRED_TABLES {
        let present: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table],
                |r| r.get(0),
            )
            .map_err(|e| format!("Schema check failed: {e}"))?;
        if present != 1 {
            return Err(format!("Migrated database is missing table {table}"));
        }
    }
    let check: String = conn
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(|e| format!("Integrity check failed: {e}"))?;
    if check != "ok" {
        return Err("Migrated database failed integrity check".to_string());
    }
    let actual = collect_plaintexts(conn, dek)?.counts;
    match expected {
        Some(expected) if actual != *expected => {
            Err("Migrated database row counts do not match the original".to_string())
        }
        None if actual.vaults < 1 => Err("Migrated database contains no vault".to_string()),
        _ => Ok(()),
    }
}

/// Remove the legacy `vault.db` superseded by a committed migration. Runs
/// only when it is provably safe (F1, G2): the directory syncs (so the
/// committed meta is durable), the on-disk meta is v2 and references
/// `vault.v2.db`, that DB opens with `dek` and passes `verify_migrated_db`
/// (with `expected` counts when called by the migration itself). Returns
/// true when no legacy DB remains. Never fails the caller: on any doubt the
/// legacy DB is simply kept.
pub(crate) fn remove_superseded_db(
    _lock: &VaultLock,
    dir: &Path,
    dek: &[u8; 32],
    expected: Option<&TableCounts>,
) -> bool {
    let legacy_path = dir.join(DB_FILENAME);
    if !legacy_path.exists() {
        return true;
    }
    let safe = (|| -> Result<bool, String> {
        connection::sync_dir(dir)?;
        let meta = connection::read_meta(dir)?;
        if meta.version != META_VERSION_V2 || meta.db_filename()? != MIGRATED_DB_FILENAME {
            return Ok(false);
        }
        let conn = connection::open_db(dir, dek)?;
        verify_migrated_db(&conn, dek, expected)?;
        Ok(true)
    })();
    match safe {
        Ok(true) => {
            connection::remove_db_files(&legacy_path);
            if let Err(e) = connection::sync_dir(dir) {
                log::warn!("Legacy database removed but directory sync failed: {e}");
            }
            !legacy_path.exists()
        }
        Ok(false) => false,
        Err(e) => {
            log::warn!("Keeping legacy database: {e}");
            false
        }
    }
}
