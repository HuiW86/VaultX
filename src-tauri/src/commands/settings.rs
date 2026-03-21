use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use crate::state::AppState;

const SETTINGS_FILENAME: &str = ".vaultx-settings";

/// User-configurable settings persisted as JSON.
/// Readable before unlock (not in encrypted DB).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultxSettings {
    pub auto_lock_timeout_minutes: i64, // -1 = never
    pub lock_on_sleep: bool,
    pub clipboard_clear_seconds: i64, // -1 = never
    pub touch_id_enabled: bool,
    pub theme: String, // "dark" | "light" | "system"
    pub start_at_login: bool,
    pub show_in_menu_bar: bool,
    #[serde(default = "default_language")]
    pub language: String, // "en" | "zh-CN"
}

fn default_language() -> String {
    "en".to_string()
}

impl Default for VaultxSettings {
    fn default() -> Self {
        Self {
            auto_lock_timeout_minutes: 480, // 8 hours
            lock_on_sleep: true,
            clipboard_clear_seconds: 30,
            touch_id_enabled: false,
            theme: "dark".to_string(),
            start_at_login: false,
            show_in_menu_bar: false,
            language: "en".to_string(),
        }
    }
}

pub fn read_settings(base_dir: &Path) -> VaultxSettings {
    let path = base_dir.join(SETTINGS_FILENAME);
    match fs::read_to_string(&path) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => VaultxSettings::default(),
    }
}

pub fn write_settings(base_dir: &Path, settings: &VaultxSettings) -> Result<(), String> {
    let path = base_dir.join(SETTINGS_FILENAME);
    let tmp_path = base_dir.join(format!("{SETTINGS_FILENAME}.tmp"));
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {e}"))?;
    fs::write(&tmp_path, &json).map_err(|e| format!("Failed to write settings: {e}"))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to rename settings: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, Mutex<AppState>>) -> Result<VaultxSettings, String> {
    let state = state.lock().map_err(|e| format!("Lock error: {e}"))?;
    Ok(read_settings(&state.data_dir))
}

#[tauri::command]
pub fn save_settings(
    settings: VaultxSettings,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| format!("Lock error: {e}"))?;
    write_settings(&state.data_dir, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_settings_when_missing() {
        let dir = TempDir::new().unwrap();
        let settings = read_settings(dir.path());
        assert_eq!(settings.auto_lock_timeout_minutes, 480);
        assert!(settings.lock_on_sleep);
        assert_eq!(settings.clipboard_clear_seconds, 30);
        assert_eq!(settings.theme, "dark");
    }

    #[test]
    fn write_and_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let mut settings = VaultxSettings::default();
        settings.theme = "light".to_string();
        settings.touch_id_enabled = true;

        write_settings(dir.path(), &settings).unwrap();
        let loaded = read_settings(dir.path());
        assert_eq!(loaded.theme, "light");
        assert!(loaded.touch_id_enabled);
    }

    #[test]
    fn corrupted_json_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(SETTINGS_FILENAME);
        fs::write(&path, "not valid json").unwrap();
        let settings = read_settings(dir.path());
        assert_eq!(settings.auto_lock_timeout_minutes, 480);
    }
}
