use std::sync::Mutex;
use tauri::State;

use crate::db::{queries::EntrySummary, search};
use crate::state::AppState;

#[tauri::command]
pub fn search_entries(
    query: String,
    limit: Option<usize>,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<EntrySummary>, String> {
    let app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    if !app.is_unlocked() {
        return Err("Vault is locked".to_string());
    }
    let db = app.db.as_ref().unwrap();
    search::search_entries(db, &query, limit.unwrap_or(20))
}

#[tauri::command]
pub fn recent_entries(
    limit: Option<usize>,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<EntrySummary>, String> {
    let app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    if !app.is_unlocked() {
        return Err("Vault is locked".to_string());
    }
    let db = app.db.as_ref().unwrap();
    search::recent_entries(db, limit.unwrap_or(5))
}
