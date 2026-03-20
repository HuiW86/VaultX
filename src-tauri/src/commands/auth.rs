use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tauri::State;
use serde::{Deserialize, Serialize};

use crate::crypto::key_derivation;
use crate::db::connection::{self, AppFileStatus, KdfConfig, KdfParams, VaultMeta};
use crate::db::queries;
use crate::state::AppState;

const FAILED_ATTEMPTS_FILENAME: &str = ".vaultx-failed-attempts";

#[derive(Debug, Serialize, Deserialize, Default)]
struct FailedAttempts {
    count: u32,
    last_failed_at: Option<u64>, // unix timestamp ms
}

fn read_failed_attempts(base_dir: &Path) -> FailedAttempts {
    let path = base_dir.join(FAILED_ATTEMPTS_FILENAME);
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_failed_attempts(base_dir: &Path, data: &FailedAttempts) {
    let path = base_dir.join(FAILED_ATTEMPTS_FILENAME);
    if let Ok(json) = serde_json::to_string(data) {
        let _ = fs::write(&path, json);
    }
}

fn clear_failed_attempts(base_dir: &Path) {
    let path = base_dir.join(FAILED_ATTEMPTS_FILENAME);
    let _ = fs::remove_file(&path);
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Compute rate-limit delay in ms based on failed attempt count.
/// 1-4: 0, 5: 5s, 6: 15s, 7: 30s, 8+: 60s
fn rate_limit_delay_ms(failed: u32) -> u64 {
    match failed {
        0..=4 => 0,
        5 => 5_000,
        6 => 15_000,
        7 => 30_000,
        _ => 60_000,
    }
}

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
        recovery_blob: None,
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

    // Check brute-force rate limit
    let attempts = read_failed_attempts(&app.data_dir);
    let delay = rate_limit_delay_ms(attempts.count);
    if delay > 0 {
        if let Some(last) = attempts.last_failed_at {
            let elapsed = now_ms().saturating_sub(last);
            if elapsed < delay {
                let remaining = delay - elapsed;
                return Err(UnlockError {
                    kind: "rate_limited".to_string(),
                    message: format!("Too many failed attempts. Wait {} seconds.", remaining / 1000 + 1),
                    retry_after_ms: Some(remaining),
                });
            }
        }
    }

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
    match connection::open_db(&app.data_dir, &master_key) {
        Ok(conn) => {
            // Success: reset failed attempts, store state
            clear_failed_attempts(&app.data_dir);
            app.db = Some(conn);
            app.master_key = Some(master_key);
            app.touch_activity();
            Ok(UnlockResult { success: true })
        }
        Err(_) => {
            // Failure: increment failed attempts with timestamp
            let new_count = attempts.count + 1;
            write_failed_attempts(&app.data_dir, &FailedAttempts {
                count: new_count,
                last_failed_at: Some(now_ms()),
            });
            let next_delay = rate_limit_delay_ms(new_count);
            Err(UnlockError {
                kind: "wrong_password".to_string(),
                message: "Incorrect master password".to_string(),
                retry_after_ms: if next_delay > 0 { Some(next_delay) } else { None },
            })
        }
    }
}

#[tauri::command]
pub fn lock(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    app.clear(); // Zeroizes master_key, closes DB
    Ok(())
}

/// Called by frontend on user interaction to reset auto-lock timer
#[tauri::command]
pub fn heartbeat(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    if app.is_unlocked() {
        app.touch_activity();
    }
    Ok(())
}
