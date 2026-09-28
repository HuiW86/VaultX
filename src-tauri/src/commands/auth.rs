use std::sync::Mutex;
use tauri::State;
use serde::Serialize;

use zeroize::Zeroizing;

use crate::commands::security::SystemKeyStore;
use crate::db::connection::{self, AppFileStatus};
use crate::state::AppState;
use crate::vault::lifecycle::{self, UnlockFailure};
use crate::vault::{no_faults, rate_limit};

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
    let password = Zeroizing::new(password);
    let mut app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    let unlocked = lifecycle::create_vault(&app.data_dir, password.as_bytes())?;
    app.set_unlocked(unlocked);
    Ok(())
}

#[tauri::command]
pub fn unlock(password: String, state: State<'_, Mutex<AppState>>) -> Result<UnlockResult, UnlockError> {
    let password = Zeroizing::new(password);
    let mut app = state.lock().map_err(|_| UnlockError {
        kind: "db_corrupted".to_string(),
        message: "State lock poisoned".to_string(),
        retry_after_ms: None,
    })?;

    // Check brute-force rate limit (shared with recovery, C12)
    if let Err(remaining) = rate_limit::check(&app.data_dir, rate_limit::now_ms()) {
        return Err(UnlockError {
            kind: "rate_limited".to_string(),
            message: rate_limit::message(remaining),
            retry_after_ms: Some(remaining),
        });
    }

    let keystore = SystemKeyStore;
    match lifecycle::unlock_with_password(&app.data_dir, password.as_bytes(), &keystore, &no_faults) {
        Ok(unlocked) => {
            // Success: reset failed attempts, store state
            rate_limit::clear(&app.data_dir);
            app.set_unlocked(unlocked);
            Ok(UnlockResult { success: true })
        }
        Err(UnlockFailure::WrongCredential) => {
            // Failure: increment failed attempts with timestamp
            let next_delay = rate_limit::record_failure(&app.data_dir, rate_limit::now_ms());
            Err(UnlockError {
                kind: "wrong_password".to_string(),
                message: "Incorrect master password".to_string(),
                retry_after_ms: if next_delay > 0 { Some(next_delay) } else { None },
            })
        }
        Err(UnlockFailure::Other(e)) => Err(UnlockError {
            kind: "db_corrupted".to_string(),
            message: e,
            retry_after_ms: None,
        }),
    }
}

#[tauri::command]
pub fn lock(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    app.clear(); // Zeroizes DEK, closes DB
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
