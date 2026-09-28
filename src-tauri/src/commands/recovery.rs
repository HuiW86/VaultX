use std::sync::Mutex;
use serde::Serialize;
use tauri::State;
use zeroize::Zeroizing;

use crate::commands::security::SystemKeyStore;
use crate::state::AppState;
use crate::vault::{lifecycle, no_faults, rate_limit};

#[derive(Debug, Serialize)]
pub struct RecoveryKitResult {
    /// Base32-encoded recovery key for the user to save
    pub recovery_key: String,
    /// Full .txt content ready for download
    pub file_content: String,
}

#[derive(Debug, Serialize)]
pub struct RecoverResult {
    /// True when the recovery committed but deleting the Keychain (Touch ID)
    /// copy failed; the UI must tell the user (contract C6).
    pub touch_id_cleanup_failed: bool,
}

fn generate_file_content(recovery_key: &str) -> String {
    format!(
        "╔══════════════════════════════════════════╗\n\
         ║         VAULTX RECOVERY KIT              ║\n\
         ╚══════════════════════════════════════════╝\n\
         \n\
         Recovery Key:\n\
         {recovery_key}\n\
         \n\
         ─────────────────────────────────────────\n\
         \n\
         INSTRUCTIONS:\n\
         1. Store this file in a safe place (printed copy recommended)\n\
         2. Do NOT store it on the same computer as VaultX\n\
         3. If you forget your master password, use this key to reset it\n\
         4. This key works once: after a recovery, generate a new kit\n\
         5. Anyone with this key can access your vault — keep it secret\n\
         \n\
         Generated: {date}\n",
        date = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"),
    )
}

/// Generate a recovery kit: wrap the DEK with a fresh recovery KEK, commit it
/// to .vaultx-meta, return key + file content.
#[tauri::command]
pub fn generate_recovery_kit(
    state: State<'_, Mutex<AppState>>,
) -> Result<RecoveryKitResult, String> {
    let app = state.lock().map_err(|_| "Lock poisoned".to_string())?;
    let dek = app.dek.as_ref().ok_or("Vault is locked")?;
    let recovery_key = lifecycle::generate_recovery_kit(&app.data_dir, dek, app.legacy, &no_faults)?;
    let file_content = generate_file_content(&recovery_key);
    Ok(RecoveryKitResult { recovery_key, file_content })
}

/// Recover the vault with the recovery key and set a new master password.
/// Only the DEK is rewrapped; the DB and fields are untouched (contract C2).
#[tauri::command]
pub fn recover_with_key(
    recovery_key: String,
    new_password: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<RecoverResult, String> {
    let recovery_key = Zeroizing::new(recovery_key);
    let new_password = Zeroizing::new(new_password);
    let mut app = state.lock().map_err(|_| "Lock poisoned".to_string())?;
    app.clear();

    let keystore = SystemKeyStore;
    // Bounded input and the shared persistent rate limit (C12).
    let outcome = lifecycle::recover_rate_limited(
        &app.data_dir,
        &recovery_key,
        new_password.as_bytes(),
        &keystore,
        &no_faults,
        rate_limit::now_ms(),
    )
    .map_err(|e| e.message())?;

    if let Some(e) = &outcome.keychain_error {
        log::error!("Recovery succeeded but Touch ID cleanup failed: {e}");
    }
    let touch_id_cleanup_failed = outcome.keychain_error.is_some();
    app.set_unlocked(outcome.unlocked);
    Ok(RecoverResult { touch_id_cleanup_failed })
}
