//! Credential flows over the DEK/KEK hierarchy.
//!
//! Invariants (contract C1/C2/C5/C6):
//! - The DB file and field ciphertexts are only ever keyed by the DEK. Password
//!   change and recovery rewrap the DEK and atomically rewrite `.vaultx-meta`;
//!   they never touch the DB.
//! - Every flow has exactly one commit point (an atomic meta rename). Before it
//!   the previous credentials work; after it the new ones do. Flows perform no
//!   destructive cleanup on error; stale artifacts are removed by `reconcile`.
//! - Every public flow holds the cross-process `VaultLock` for its whole
//!   duration (C10). Internals that expect the lock take `&VaultLock`.

use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine};
use rusqlite::Connection;
use zeroize::Zeroizing;

use super::keystore::KeyStore;
use super::lock::VaultLock;
use super::{migration, rate_limit, recovery_key, FaultHook};
use crate::commands::settings::{read_settings, write_settings};
use crate::crypto::{encryption, key_derivation, key_wrap};
use crate::db::connection::{
    self, AppFileStatus, KdfConfig, VaultMeta, DB_FILENAME, KNOWN_DB_FILENAMES, META_FILENAME,
    META_VERSION_V1, META_VERSION_V2, MIGRATED_DB_FILENAME,
};
use crate::db::queries;

/// An opened vault: DB connection plus the key used for fields.
pub struct Unlocked {
    pub conn: Connection,
    /// The DEK (v2). In legacy mode (v1 vault whose migration failed) this is
    /// the v1 password-derived key, which keys both DB and fields.
    pub key: Zeroizing<[u8; 32]>,
    /// True when the vault is still v1 because migration failed this time.
    pub legacy: bool,
}

impl std::fmt::Debug for Unlocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Unlocked").field("legacy", &self.legacy).finish_non_exhaustive()
    }
}

/// Stable error code returned to the UI when the migrated DB fails
/// verification while the legacy DB still exists (G2, E4): the vault is not
/// opened and the user is told a manual restore is needed. The frontend maps
/// it to a translated explanation.
pub const MANUAL_RESTORE_REQUIRED: &str = "manual_restore_required";

#[derive(Debug, Clone, PartialEq)]
pub enum UnlockFailure {
    WrongCredential,
    /// The credential is valid, but `vault.v2.db` failed verification while
    /// the legacy `vault.db` still exists. Nothing was opened or deleted.
    ManualRestoreRequired,
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecoverFailure {
    InvalidKey,
    NoRecoveryKit,
    /// Too many failed attempts (password or recovery); the key was not evaluated (C12).
    RateLimited { retry_after_ms: u64 },
    /// See `UnlockFailure::ManualRestoreRequired`. Nothing was committed.
    ManualRestoreRequired,
    Other(String),
}

impl RecoverFailure {
    /// User-facing message. Fixed strings only (C4).
    pub fn message(&self) -> String {
        match self {
            RecoverFailure::InvalidKey => "Invalid recovery key".to_string(),
            RecoverFailure::NoRecoveryKit => "No recovery kit has been set up".to_string(),
            RecoverFailure::RateLimited { retry_after_ms } => rate_limit::message(*retry_after_ms),
            RecoverFailure::ManualRestoreRequired => MANUAL_RESTORE_REQUIRED.to_string(),
            RecoverFailure::Other(e) => e.clone(),
        }
    }
}

#[derive(Debug)]
pub struct RecoverOutcome {
    pub unlocked: Unlocked,
    /// Set when deleting the Keychain copy failed after the recovery committed (C6).
    pub keychain_error: Option<String>,
}

/// Remove artifacts left by an interrupted flow. Safe to call at any time:
/// it never removes the DB referenced by a readable meta, and removes other
/// DB files only when that referenced DB exists. The legacy `vault.db`
/// superseded by a committed migration is *not* removed here (there is no key
/// to prove the new DB opens); `migration::remove_superseded_db` does that
/// after a keyed open (F1/C8).
pub fn reconcile(dir: &Path) -> Result<(), String> {
    let lock = VaultLock::acquire(dir)?;
    reconcile_locked(&lock, dir)
}

fn reconcile_locked(_lock: &VaultLock, dir: &Path) -> Result<(), String> {
    let _ = std::fs::remove_file(dir.join(format!("{META_FILENAME}.tmp")));
    if !dir.join(META_FILENAME).exists() {
        return Ok(());
    }
    let Ok(meta) = connection::read_meta(dir) else {
        return Ok(()); // Corrupted: leave everything for inspection.
    };
    let Ok(current) = meta.db_filename() else {
        return Ok(());
    };
    if !dir.join(current).exists() {
        return Ok(());
    }
    for name in KNOWN_DB_FILENAMES.iter().filter(|n| **n != current) {
        if current == MIGRATED_DB_FILENAME && *name == DB_FILENAME {
            continue; // Superseded legacy DB: removed only after a keyed open.
        }
        connection::remove_db_files(&dir.join(name));
    }
    Ok(())
}

/// Create a new v2 vault protected by `password`.
pub fn create_vault(dir: &Path, password: &[u8]) -> Result<Unlocked, String> {
    let _lock = VaultLock::acquire(dir)?;
    match connection::check_file_status(dir) {
        AppFileStatus::FirstRun => {}
        _ => return Err("Vault already exists".to_string()),
    }

    let dek = key_wrap::generate_dek();
    let salt = key_derivation::generate_salt();
    let kek = key_derivation::derive_key(password, &salt)?;
    let wrapped = key_wrap::wrap_dek(&kek, &dek, key_wrap::PURPOSE_PASSWORD)?;

    let result = (|| {
        let conn = connection::init_db(dir, &dek)?;
        queries::create_vault(&conn, "Personal", None)
            .map_err(|e| format!("Failed to create default vault: {e}"))?;
        let meta = VaultMeta::new_v2(&salt, DB_FILENAME, chrono::Utc::now().to_rfc3339(), wrapped);
        connection::write_meta(dir, &meta)?;
        Ok::<_, String>(conn)
    })();

    match result {
        Ok(conn) => Ok(Unlocked { conn, key: dek, legacy: false }),
        Err(e) => {
            // Nothing valuable exists yet: remove the half-created vault.
            connection::cleanup_files(dir);
            Err(e)
        }
    }
}

/// Unlock with the master password. A v1 vault is migrated to v2 (C8); if the
/// migration fails the vault opens in legacy mode and migration is retried on
/// the next password unlock.
pub fn unlock_with_password(
    dir: &Path,
    password: &[u8],
    keystore: &dyn KeyStore,
    hook: FaultHook,
) -> Result<Unlocked, UnlockFailure> {
    let lock = VaultLock::acquire(dir).map_err(UnlockFailure::Other)?;
    reconcile_locked(&lock, dir).map_err(UnlockFailure::Other)?;
    let meta = connection::read_meta(dir).map_err(UnlockFailure::Other)?;

    match meta.version {
        META_VERSION_V2 => unlock_v2(&lock, dir, &meta, password, keystore),
        META_VERSION_V1 => {
            let legacy_key = key_derivation::derive_key(password, &meta.kdf.salt)
                .map_err(UnlockFailure::Other)?;
            drop(connection::open_db(dir, &legacy_key).map_err(|_| UnlockFailure::WrongCredential)?);

            match migration::migrate_v1_locked(&lock, dir, &legacy_key, password, hook) {
                Ok(dek) => {
                    if let Err(e) = finish_keychain_invalidation_locked(&lock, dir, keystore) {
                        log::warn!("Keychain invalidation after migration pending: {e}");
                    }
                    let conn = connection::open_db(dir, &dek).map_err(UnlockFailure::Other)?;
                    Ok(Unlocked { conn, key: dek, legacy: false })
                }
                Err(e) => {
                    // The migration may have failed after its commit point.
                    let meta = connection::read_meta(dir).map_err(UnlockFailure::Other)?;
                    if meta.version == META_VERSION_V2 {
                        return unlock_v2(&lock, dir, &meta, password, keystore);
                    }
                    log::error!("v1 to v2 migration failed, vault stays v1: {e}");
                    let conn = connection::open_db(dir, &legacy_key).map_err(UnlockFailure::Other)?;
                    Ok(Unlocked { conn, key: legacy_key, legacy: true })
                }
            }
        }
        _ => Err(UnlockFailure::Other("Unsupported vault version".to_string())),
    }
}

fn unlock_v2(
    lock: &VaultLock,
    dir: &Path,
    meta: &VaultMeta,
    password: &[u8],
    keystore: &dyn KeyStore,
) -> Result<Unlocked, UnlockFailure> {
    let dek = unwrap_with_password(meta, password)?;
    // The DEK is authenticated by the unwrap, so a failing check here means
    // the new DB itself is bad; never open it silently while the legacy DB
    // exists (G2).
    finish_superseded_db(lock, dir, &dek).map_err(|_| UnlockFailure::ManualRestoreRequired)?;
    let conn = connection::open_db(dir, &dek).map_err(UnlockFailure::Other)?;
    if meta.keychain_invalidation_pending {
        if let Err(e) = finish_keychain_invalidation_locked(lock, dir, keystore) {
            log::warn!("Keychain invalidation still pending: {e}");
        }
    }
    Ok(Unlocked { conn, key: dek, legacy: false })
}

fn unwrap_with_password(meta: &VaultMeta, password: &[u8]) -> Result<Zeroizing<[u8; 32]>, UnlockFailure> {
    let wrapped = meta
        .dek_wrapped_by_password
        .as_deref()
        .ok_or_else(|| UnlockFailure::Other("Vault metadata is missing the wrapped key".to_string()))?;
    let kek = key_derivation::derive_key(password, &meta.kdf.salt).map_err(UnlockFailure::Other)?;
    key_wrap::unwrap_dek(&kek, wrapped, key_wrap::PURPOSE_PASSWORD)
        .map_err(|_| UnlockFailure::WrongCredential)
}

/// Unlock with the DEK copy stored in the Keychain (Touch ID).
pub fn unlock_with_keystore(dir: &Path, keystore: &dyn KeyStore) -> Result<Unlocked, String> {
    let lock = VaultLock::acquire(dir)?;
    reconcile_locked(&lock, dir)?;
    let meta = connection::read_meta(dir)?;
    if meta.version != META_VERSION_V2 {
        return Err("Unlock with your master password once to upgrade this vault".to_string());
    }
    if meta.keychain_invalidation_pending {
        let _ = finish_keychain_invalidation_locked(&lock, dir, keystore);
        return Err("Touch ID was reset. Unlock with your master password".to_string());
    }
    let bytes = keystore.read()?;
    if bytes.len() != 32 {
        return Err("Invalid key in Keychain".to_string());
    }
    let mut dek = Zeroizing::new([0u8; 32]);
    dek.copy_from_slice(&bytes);
    let conn = connection::open_db(dir, &dek)
        .map_err(|_| "Touch ID key does not match this vault".to_string())?;
    finish_superseded_db(&lock, dir, &dek).map_err(|_| MANUAL_RESTORE_REQUIRED.to_string())?;
    Ok(Unlocked { conn, key: dek, legacy: false })
}

/// Retry removing the legacy DB after a keyed open (`remove_superseded_db`).
/// `Err` when the legacy DB exists and the new DB fails verification; the
/// caller must refuse to open the vault. A kept legacy DB is only logged.
fn finish_superseded_db(lock: &VaultLock, dir: &Path, dek: &[u8; 32]) -> Result<(), String> {
    match migration::remove_superseded_db(lock, dir, dek) {
        Err(e) => {
            log::error!("Migrated database failed verification; legacy database kept, manual restore needed: {e}");
            Err(e)
        }
        Ok(migration::SupersededOutcome::Kept(reason)) => {
            log::warn!("Legacy database kept: {reason}");
            Ok(())
        }
        Ok(_) => Ok(()),
    }
}

/// Store the DEK in the Keychain for Touch ID (C6: never a password KEK).
pub fn enable_touch_id(
    dir: &Path,
    dek: &[u8; 32],
    legacy: bool,
    keystore: &dyn KeyStore,
) -> Result<(), String> {
    if legacy {
        return Err("Vault upgrade pending. Unlock with your master password first".to_string());
    }
    let lock = VaultLock::acquire(dir)?;
    let meta = connection::read_meta(dir)?;
    if meta.version != META_VERSION_V2 {
        return Err("Vault upgrade pending. Unlock with your master password first".to_string());
    }
    if meta.keychain_invalidation_pending {
        finish_keychain_invalidation_locked(&lock, dir, keystore)?;
    }
    let _ = keystore.delete(); // Remove stale item if any
    keystore.store(dek)
}

/// Whether the vault has a usable recovery kit (C11). Read-only; no lock.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RecoveryKitStatus {
    /// v2: `dek_wrapped_by_recovery` present; v1: `recovery_blob` present.
    pub present: bool,
    /// False in legacy mode or for a v1 vault, where kit generation is refused.
    pub can_generate: bool,
}

pub fn recovery_kit_status(dir: &Path, legacy: bool) -> Result<RecoveryKitStatus, String> {
    let meta = connection::read_meta(dir)?;
    Ok(match meta.version {
        META_VERSION_V2 => RecoveryKitStatus {
            present: meta.dek_wrapped_by_recovery.is_some(),
            can_generate: !legacy,
        },
        _ => RecoveryKitStatus { present: meta.recovery_blob.is_some(), can_generate: false },
    })
}

/// Create a new recovery kit: wrap the DEK with a fresh recovery KEK and
/// commit it to meta (replacing any previous kit). Returns the grouped key.
pub fn generate_recovery_kit(
    dir: &Path,
    dek: &[u8; 32],
    legacy: bool,
    hook: FaultHook,
) -> Result<String, String> {
    if legacy {
        return Err("Vault upgrade pending. Unlock with your master password first".to_string());
    }
    let _lock = VaultLock::acquire(dir)?;
    let mut meta = connection::read_meta(dir)?;
    if meta.version != META_VERSION_V2 {
        return Err("Vault upgrade pending. Unlock with your master password first".to_string());
    }
    // Sanity check: never wrap a key that does not open this vault.
    drop(connection::open_db(dir, dek).map_err(|_| "Vault key mismatch".to_string())?);

    let raw = recovery_key::generate_raw();
    let rkek = recovery_key::derive_kek(&*raw)?;
    meta.dek_wrapped_by_recovery = Some(key_wrap::wrap_dek(&rkek, dek, key_wrap::PURPOSE_RECOVERY)?);

    hook("recovery_kit:before_commit")?;
    connection::write_meta_with_hook(dir, &meta, hook)?;
    hook("recovery_kit:committed")?;
    Ok(recovery_key::encode_grouped(&*raw))
}

/// Recover with the recovery key and set a new master password.
/// v2: rewrap only (C2). v1: decrypt the legacy blob and migrate (C8).
/// On success the used kit is invalidated (C5) and the Keychain copy is
/// deleted with `touch_id_enabled` cleared (C6).
pub fn recover(
    dir: &Path,
    recovery_key_text: &str,
    new_password: &[u8],
    keystore: &dyn KeyStore,
    hook: FaultHook,
) -> Result<RecoverOutcome, RecoverFailure> {
    // Cheap input checks first: never parse unbounded input (F5).
    let raw = recovery_key::decode(recovery_key_text).map_err(|_| RecoverFailure::InvalidKey)?;
    let lock = VaultLock::acquire(dir).map_err(RecoverFailure::Other)?;
    reconcile_locked(&lock, dir).map_err(RecoverFailure::Other)?;
    let meta = connection::read_meta(dir).map_err(RecoverFailure::Other)?;

    if raw.len() != recovery_key::RECOVERY_KEY_LEN {
        return Err(RecoverFailure::InvalidKey);
    }

    let dek = match meta.version {
        META_VERSION_V2 => {
            let wrapped = meta
                .dek_wrapped_by_recovery
                .as_deref()
                .ok_or(RecoverFailure::NoRecoveryKit)?;
            let rkek = recovery_key::derive_kek(&raw).map_err(RecoverFailure::Other)?;
            let dek = key_wrap::unwrap_dek(&rkek, wrapped, key_wrap::PURPOSE_RECOVERY)
                .map_err(|_| RecoverFailure::InvalidKey)?;
            // Never commit a recovery onto an empty or incomplete migrated DB
            // while the legacy DB exists (G2); the kit keeps working.
            finish_superseded_db(&lock, dir, &dek).map_err(|_| RecoverFailure::ManualRestoreRequired)?;
            drop(connection::open_db(dir, &dek).map_err(RecoverFailure::Other)?);

            let salt = key_derivation::generate_salt();
            let kek = key_derivation::derive_key(new_password, &salt).map_err(RecoverFailure::Other)?;
            let mut updated = meta.clone();
            updated.kdf = KdfConfig::argon2id(&salt);
            updated.dek_wrapped_by_password = Some(
                key_wrap::wrap_dek(&kek, &dek, key_wrap::PURPOSE_PASSWORD).map_err(RecoverFailure::Other)?,
            );
            updated.dek_wrapped_by_recovery = None; // C5: kit is single-use
            updated.keychain_invalidation_pending = true; // C6

            hook("recover:before_commit").map_err(RecoverFailure::Other)?;
            connection::write_meta_with_hook(dir, &updated, hook).map_err(RecoverFailure::Other)?;
            hook("recover:committed").map_err(RecoverFailure::Other)?;
            dek
        }
        META_VERSION_V1 => {
            let blob_b64 = meta.recovery_blob.as_deref().ok_or(RecoverFailure::NoRecoveryKit)?;
            let blob = STANDARD.decode(blob_b64).map_err(|_| RecoverFailure::InvalidKey)?;
            let rkek = recovery_key::derive_kek(&raw).map_err(RecoverFailure::Other)?;
            let legacy_bytes = Zeroizing::new(
                encryption::decrypt(&rkek, &blob).map_err(|_| RecoverFailure::InvalidKey)?,
            );
            if legacy_bytes.len() != 32 {
                return Err(RecoverFailure::InvalidKey);
            }
            let mut legacy_key = Zeroizing::new([0u8; 32]);
            legacy_key.copy_from_slice(&legacy_bytes);
            drop(connection::open_db(dir, &legacy_key).map_err(RecoverFailure::Other)?);
            // Migration writes a v2 meta wrapped by the new password, without a
            // recovery wrap (C5) and with Keychain invalidation pending (C6).
            match migration::migrate_v1_locked(&lock, dir, &legacy_key, new_password, hook) {
                Ok(dek) => dek,
                Err(e) => {
                    // The migration may have failed after its commit point
                    // because the new DB did not verify: report that instead
                    // of a generic error (read-only check, nothing deleted).
                    if let Ok(meta) = connection::read_meta(dir) {
                        if meta.version == META_VERSION_V2 {
                            if let Ok(dek) = unwrap_with_password(&meta, new_password) {
                                if migration::check_superseding_db(dir, &dek).is_err() {
                                    return Err(RecoverFailure::ManualRestoreRequired);
                                }
                            }
                        }
                    }
                    return Err(RecoverFailure::Other(e));
                }
            }
        }
        _ => return Err(RecoverFailure::Other("Unsupported vault version".to_string())),
    };

    let keychain_error = finish_keychain_invalidation_locked(&lock, dir, keystore).err();
    let conn = connection::open_db(dir, &dek).map_err(RecoverFailure::Other)?;
    Ok(RecoverOutcome {
        unlocked: Unlocked { conn, key: dek, legacy: false },
        keychain_error,
    })
}

/// `recover` behind the persistent failed-attempt limiter shared with
/// password unlock (C12). This is the entry point for the IPC command.
/// While limited, the key is not evaluated. An invalid key counts as a
/// failure; success clears the counter; other errors leave it unchanged.
pub fn recover_rate_limited(
    dir: &Path,
    recovery_key_text: &str,
    new_password: &[u8],
    keystore: &dyn KeyStore,
    hook: FaultHook,
    now_ms: u64,
) -> Result<RecoverOutcome, RecoverFailure> {
    if let Err(retry_after_ms) = rate_limit::check(dir, now_ms) {
        return Err(RecoverFailure::RateLimited { retry_after_ms });
    }
    match recover(dir, recovery_key_text, new_password, keystore, hook) {
        Ok(outcome) => {
            rate_limit::clear(dir);
            Ok(outcome)
        }
        Err(RecoverFailure::InvalidKey) => {
            rate_limit::record_failure(dir, now_ms);
            Err(RecoverFailure::InvalidKey)
        }
        Err(e) => Err(e),
    }
}

/// Change the master password: rewrap the DEK only (C2). The recovery kit
/// stays valid. There is no IPC command for this yet; it is the single
/// implementation any future password-change UI must use.
pub fn change_password(
    dir: &Path,
    current_password: &[u8],
    new_password: &[u8],
    hook: FaultHook,
) -> Result<(), UnlockFailure> {
    let lock = VaultLock::acquire(dir).map_err(UnlockFailure::Other)?;
    reconcile_locked(&lock, dir).map_err(UnlockFailure::Other)?;
    let meta = connection::read_meta(dir).map_err(UnlockFailure::Other)?;
    if meta.version != META_VERSION_V2 {
        return Err(UnlockFailure::Other(
            "Vault upgrade pending. Unlock with your master password first".to_string(),
        ));
    }
    let dek = unwrap_with_password(&meta, current_password)?;

    let salt = key_derivation::generate_salt();
    let kek = key_derivation::derive_key(new_password, &salt).map_err(UnlockFailure::Other)?;
    let mut updated = meta.clone();
    updated.kdf = KdfConfig::argon2id(&salt);
    updated.dek_wrapped_by_password = Some(
        key_wrap::wrap_dek(&kek, &dek, key_wrap::PURPOSE_PASSWORD).map_err(UnlockFailure::Other)?,
    );

    hook("change_password:before_commit").map_err(UnlockFailure::Other)?;
    connection::write_meta_with_hook(dir, &updated, hook).map_err(UnlockFailure::Other)?;
    hook("change_password:committed").map_err(UnlockFailure::Other)?;
    Ok(())
}

/// Complete a pending Keychain invalidation (C6): clear `touch_id_enabled`
/// and delete the Keychain item, then clear the meta flag. Both steps are
/// always attempted, independently: a settings write failure must not keep
/// the (possibly legacy) key in the Keychain. The flag is cleared only when
/// both succeeded; otherwise it stays set and Touch ID unlock is refused
/// until a later call succeeds. Idempotent.
pub fn finish_keychain_invalidation(dir: &Path, keystore: &dyn KeyStore) -> Result<(), String> {
    let lock = VaultLock::acquire(dir)?;
    finish_keychain_invalidation_locked(&lock, dir, keystore)
}

fn finish_keychain_invalidation_locked(
    _lock: &VaultLock,
    dir: &Path,
    keystore: &dyn KeyStore,
) -> Result<(), String> {
    let mut meta = connection::read_meta(dir)?;
    if !meta.keychain_invalidation_pending {
        return Ok(());
    }

    let mut errors: Vec<String> = Vec::new();
    let mut settings = read_settings(dir);
    if settings.touch_id_enabled {
        settings.touch_id_enabled = false;
        if let Err(e) = write_settings(dir, &settings) {
            log::warn!("Keychain invalidation: disabling Touch ID in settings failed: {e}");
            errors.push("Failed to turn off Touch ID in settings".to_string());
        }
    }
    if let Err(e) = keystore.delete() {
        log::warn!("Keychain invalidation: deleting the Keychain item failed: {e}");
        errors.push("Failed to remove the Touch ID key from the Keychain".to_string());
    }
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }

    meta.keychain_invalidation_pending = false;
    connection::write_meta(dir, &meta)
}
