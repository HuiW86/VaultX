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
//! 4. Post-commit: the committed meta records the source row counts in
//!    `superseded_db`. `vault.db` is deleted only once the v2 meta is
//!    confirmed durable and on disk and the new DB reopens with the DEK, has
//!    the full schema and exactly the recorded row counts
//!    (`verify_migrated_db`, `remove_superseded_db`); the marker is cleared
//!    afterwards. If anything fails, `vault.db` is kept and the next keyed open
//!    (password, Touch ID, recovery) retries; if the new DB fails verification
//!    while `vault.db` exists, that open is refused with a manual-restore
//!    error instead of opening an empty or incomplete vault (G2, 2.3).
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
    self, SupersededDb, VaultMeta, DB_FILENAME, META_VERSION_V1, META_VERSION_V2,
    MIGRATED_DB_FILENAME,
};

pub use crate::db::connection::TableCounts;

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
    // Every later deletion of the legacy DB must match these counts (G2).
    new_meta.superseded_db = Some(SupersededDb {
        counts: expected.counts.clone(),
        legacy_kdf: meta.kdf.clone(),
        legacy_recovery_blob: meta.recovery_blob.clone(),
    });
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
    hook("migrate:before_legacy_delete")?;
    // "Opens with the DEK" is not enough (a 0-byte file opens with any key):
    // the committed DB must hold the schema and exactly the migrated rows,
    // which `remove_superseded_db` checks against the counts in the meta.
    match remove_superseded_db(lock, dir, &dek) {
        Err(_) => {
            return Err("Migrated database could not be verified; the original database was kept".to_string())
        }
        Ok(SupersededOutcome::Kept(reason)) => {
            log::warn!("Legacy database kept after migration ({reason}); it will be removed on a later unlock");
        }
        Ok(_) => {}
    }
    Ok(dek)
}

/// Tables that must exist in a migrated DB before the legacy DB may go.
const REQUIRED_TABLES: [&str; 4] = ["vaults", "entries", "fields", "password_history"];

/// Check that `conn` (the committed `vault.v2.db`) really holds the vault:
/// every required table exists, the integrity check passes and every
/// sensitive field and history value decrypts with `dek`. With `expected`
/// (the source counts recorded in meta at the migration commit) all row
/// counts must equal them; without it (meta written before 2.3, which
/// recorded no counts) the DB must at least contain a vault row. A 0-byte or
/// freshly created file fails.
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

/// Result of `remove_superseded_db` when the new DB is not known to be bad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupersededOutcome {
    /// No legacy DB is superseded (not migrated, or already removed).
    Absent,
    /// The legacy DB was verified to be superseded and deleted.
    Removed,
    /// The legacy DB was kept (reason given); the new DB passed verification
    /// but deletion was not provably safe or did not complete.
    Kept(String),
}

/// Whether the legacy `vault.db` is superseded by a committed migration:
/// meta is v2, references `vault.v2.db`, and `vault.db` still exists.
fn legacy_db_pending(dir: &Path, meta: &VaultMeta) -> bool {
    meta.version == META_VERSION_V2
        && meta.db_filename().ok() == Some(MIGRATED_DB_FILENAME)
        && dir.join(DB_FILENAME).exists()
}

/// Read-only check used before trusting `vault.v2.db` while the legacy DB
/// still exists: the new DB must open with `dek` and pass
/// `verify_migrated_db` against the row counts recorded in meta at the
/// migration commit (or, for meta written before 2.3 without them, contain
/// at least one vault). `Ok(())` when no legacy DB is pending. `Err` means
/// the new DB is empty or incomplete and must not be opened silently.
pub(crate) fn check_superseding_db(dir: &Path, dek: &[u8; 32]) -> Result<(), String> {
    let meta = connection::read_meta(dir)?;
    if !legacy_db_pending(dir, &meta) {
        return Ok(());
    }
    let conn = connection::open_db(dir, dek)?;
    verify_migrated_db(&conn, dek, meta.superseded_db.as_ref().map(|s| &s.counts))
}

/// Remove the legacy `vault.db` superseded by a committed migration. Runs
/// only when it is provably safe (F1, G2): the on-disk meta is v2 and
/// references `vault.v2.db`, that DB opens with `dek` and passes
/// `verify_migrated_db` with **exactly** the row counts recorded in
/// `meta.superseded_db` at the migration commit, and the directory syncs.
/// Every caller (migration, password unlock, Touch ID unlock, recovery)
/// goes through here, so no path deletes the legacy DB without that
/// comparison; without recorded counts the legacy DB is kept.
///
/// Returns `Err` when the legacy DB exists and the new DB fails
/// verification: the caller must not open the vault silently (it reports
/// that a manual restore is needed). Otherwise returns the outcome; the
/// legacy DB is never deleted on any doubt. Clears `superseded_db` once the
/// legacy DB is gone.
pub(crate) fn remove_superseded_db(
    _lock: &VaultLock,
    dir: &Path,
    dek: &[u8; 32],
) -> Result<SupersededOutcome, String> {
    let legacy_path = dir.join(DB_FILENAME);
    let mut meta = match connection::read_meta(dir) {
        Ok(m) => m,
        Err(e) => return Ok(SupersededOutcome::Kept(e)),
    };
    let migrated = meta.version == META_VERSION_V2 && meta.db_filename().ok() == Some(MIGRATED_DB_FILENAME);
    if !migrated {
        return Ok(SupersededOutcome::Absent);
    }
    if !legacy_path.exists() {
        clear_superseded_marker(dir, &mut meta);
        return Ok(SupersededOutcome::Absent);
    }

    check_superseding_db(dir, dek)?;
    if meta.superseded_db.is_none() {
        let reason = "no recorded row counts for the legacy database".to_string();
        log::warn!("Keeping legacy database: {reason}");
        return Ok(SupersededOutcome::Kept(reason));
    }
    // The committed meta must be durable before the destructive step.
    if let Err(e) = connection::sync_dir(dir) {
        log::warn!("Keeping legacy database: {e}");
        return Ok(SupersededOutcome::Kept(e));
    }
    connection::remove_db_files(&legacy_path);
    if let Err(e) = connection::sync_dir(dir) {
        log::warn!("Legacy database removed but directory sync failed: {e}");
    }
    if legacy_path.exists() {
        let reason = "the legacy database file could not be deleted".to_string();
        log::warn!("Keeping legacy database: {reason}");
        return Ok(SupersededOutcome::Kept(reason));
    }
    clear_superseded_marker(dir, &mut meta);
    Ok(SupersededOutcome::Removed)
}

/// Drop `superseded_db` from meta once no legacy DB remains. Best effort: a
/// stale marker without a legacy DB is harmless and retried next time.
fn clear_superseded_marker(dir: &Path, meta: &mut VaultMeta) {
    if meta.superseded_db.take().is_some() {
        if let Err(e) = connection::write_meta(dir, meta) {
            log::warn!("Could not clear the superseded database marker: {e}");
        }
    }
}
