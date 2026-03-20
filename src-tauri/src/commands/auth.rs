use std::sync::Mutex;
use tauri::State;
use serde::Serialize;

use crate::crypto::key_derivation;
use crate::db::connection::{self, AppFileStatus, KdfConfig, KdfParams, VaultMeta};
use crate::db::queries;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppStatus {
    FirstRun,
    Locked,
    Unlocked,
    Corrupted { reason: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct UnlockResult {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnlockError {
    pub kind: String, // "wrong_password" | "db_corrupted" | "rate_limited"
    pub message: String,
    pub retry_after_ms: Option<u64>,
}

#[tauri::command]
pub fn get_app_status(state: State<'_, Mutex<AppState>>) -> Result<AppStatus, String> {
    let app = state.lock().map_err(|_| "State lock poisoned".to_string())?;

    if app.is_unlocked() {
        return Ok(AppStatus::Unlocked);
    }

    match connection::check_file_status(&app.data_dir) {
        AppFileStatus::FirstRun => Ok(AppStatus::FirstRun),
        AppFileStatus::Ready => Ok(AppStatus::Locked),
        AppFileStatus::Corrupted { reason } => Ok(AppStatus::Corrupted { reason }),
    }
}

#[tauri::command]
pub fn setup_vault(password: String, state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut app = state.lock().map_err(|_| "State lock poisoned".to_string())?;

    // Ensure we're in first_run state
    match connection::check_file_status(&app.data_dir) {
        AppFileStatus::FirstRun => {}
        _ => return Err("Vault already exists".to_string()),
    }

    // Generate salt and derive key
    let salt = key_derivation::generate_salt();
    let master_key = key_derivation::derive_key(password.as_bytes(), &salt)?;
    // password String is dropped at end of this function (Rust ownership)

    // Initialize encrypted database
    let conn = match connection::init_db(&app.data_dir, &master_key) {
        Ok(c) => c,
        Err(e) => {
            // Cleanup on failure — no half-state
            connection::cleanup_files(&app.data_dir);
            return Err(e);
        }
    };

    // Create default "Personal" vault
    if let Err(e) = queries::create_vault(&conn, "Personal", None) {
        connection::cleanup_files(&app.data_dir);
        return Err(format!("Failed to create default vault: {e}"));
    }

    // Write meta file atomically
    let meta = VaultMeta {
        version: 1,
        kdf: KdfConfig {
            algorithm: "argon2id".to_string(),
            params: KdfParams {
                m_cost: 19456,
                t_cost: 2,
                p_cost: 1,
            },
            salt: salt.to_vec(),
        },
        created_at: chrono::Utc::now().to_rfc3339(),
        db_path: "vault.db".to_string(),
    };

    if let Err(e) = connection::write_meta(&app.data_dir, &meta) {
        connection::cleanup_files(&app.data_dir);
        return Err(e);
    }

    // Store in memory
    app.db = Some(conn);
    app.master_key = Some(master_key);

    Ok(())
}

#[tauri::command]
pub fn unlock(password: String, state: State<'_, Mutex<AppState>>) -> Result<UnlockResult, UnlockError> {
    let mut app = state.lock().map_err(|_| UnlockError {
        kind: "db_corrupted".to_string(),
        message: "State lock poisoned".to_string(),
        retry_after_ms: None,
    })?;

    // Read meta to get salt and KDF params
    let meta = connection::read_meta(&app.data_dir).map_err(|e| UnlockError {
        kind: "db_corrupted".to_string(),
        message: format!("Cannot read vault metadata: {e}"),
        retry_after_ms: None,
    })?;

    // Derive key from password
    let master_key = key_derivation::derive_key(password.as_bytes(), &meta.kdf.salt)
        .map_err(|e| UnlockError {
            kind: "db_corrupted".to_string(),
            message: format!("Key derivation failed: {e}"),
            retry_after_ms: None,
        })?;

    // Try to open database with derived key
    let conn = connection::open_db(&app.data_dir, &master_key).map_err(|_| UnlockError {
        kind: "wrong_password".to_string(),
        message: "Incorrect master password".to_string(),
        retry_after_ms: None,
    })?;

    // Store in memory
    app.db = Some(conn);
    app.master_key = Some(master_key);

    Ok(UnlockResult { success: true })
}

#[tauri::command]
pub fn lock(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    app.clear(); // Zeroizes master_key, closes DB
    Ok(())
}
