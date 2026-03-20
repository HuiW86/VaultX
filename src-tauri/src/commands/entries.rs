use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use serde::{Deserialize, Serialize};

use crate::crypto::encryption;
use crate::db::queries::{self, EntrySummary, EntryWithFields, Field, FieldInput};
use crate::state::AppState;

// Field types that get AES-256-GCM encryption on top of SQLCipher
const SENSITIVE_TYPES: &[&str] = &["password", "hidden", "card_number"];

#[derive(Debug, Deserialize)]
pub struct CreateEntryInput {
    pub vault_id: String,
    pub category: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub fields: Vec<FieldInputDto>,
}

#[derive(Debug, Deserialize)]
pub struct FieldInputDto {
    pub field_type: String,
    pub label: String,
    pub value: String, // plaintext from frontend
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEntryInput {
    pub entry_id: String,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub fields: Option<Vec<FieldInputDto>>,
}

/// Decrypted field for frontend display
#[derive(Debug, Serialize)]
pub struct DecryptedField {
    pub id: String,
    pub entry_id: String,
    pub field_type: String,
    pub label: String,
    pub value: String, // plaintext
    pub sort_order: i32,
    pub sensitive: bool,
}

#[derive(Debug, Serialize)]
pub struct EntryDetailResponse {
    pub entry: queries::Entry,
    pub fields: Vec<DecryptedField>,
}

fn require_unlocked(app: &AppState) -> Result<(), String> {
    if !app.is_unlocked() {
        return Err("Vault is locked".to_string());
    }
    Ok(())
}

fn encrypt_field_value(key: &[u8; 32], field_type: &str, plaintext: &str) -> Result<Vec<u8>, String> {
    if SENSITIVE_TYPES.contains(&field_type) {
        encryption::encrypt(key, plaintext.as_bytes())
    } else {
        Ok(plaintext.as_bytes().to_vec())
    }
}

fn decrypt_field(key: &[u8; 32], field: &Field) -> Result<DecryptedField, String> {
    let sensitive = SENSITIVE_TYPES.contains(&field.field_type.as_str());
    let plaintext = if sensitive {
        let bytes = encryption::decrypt(key, &field.value)?;
        String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8: {e}"))?
    } else {
        String::from_utf8(field.value.clone()).map_err(|e| format!("Invalid UTF-8: {e}"))?
    };

    Ok(DecryptedField {
        id: field.id.clone(),
        entry_id: field.entry_id.clone(),
        field_type: field.field_type.clone(),
        label: field.label.clone(),
        value: plaintext,
        sort_order: field.sort_order,
        sensitive,
    })
}

#[tauri::command]
pub fn create_entry(input: CreateEntryInput, state: State<'_, Mutex<AppState>>) -> Result<EntrySummary, String> {
    let app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    require_unlocked(&app)?;
    let db = app.db.as_ref().unwrap();
    let key = app.master_key.as_ref().unwrap();

    let fields: Result<Vec<FieldInput>, String> = input
        .fields
        .iter()
        .map(|f| {
            let encrypted = encrypt_field_value(key, &f.field_type, &f.value)?;
            Ok(FieldInput {
                field_type: f.field_type.clone(),
                label: f.label.clone(),
                value: encrypted,
                sort_order: f.sort_order,
            })
        })
        .collect();

    let result = queries::create_entry(
        db,
        &input.vault_id,
        &input.category,
        &input.title,
        input.subtitle.as_deref(),
        &fields?,
    )?;

    Ok(EntrySummary {
        id: result.entry.id,
        vault_id: result.entry.vault_id,
        category: result.entry.category,
        title: result.entry.title,
        subtitle: result.entry.subtitle,
        icon_url: result.entry.icon_url,
        favorite: result.entry.favorite,
        trashed: result.entry.trashed,
        updated_at: result.entry.updated_at,
    })
}

#[tauri::command]
pub fn get_entry(entry_id: String, state: State<'_, Mutex<AppState>>) -> Result<EntryDetailResponse, String> {
    let app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    require_unlocked(&app)?;
    let db = app.db.as_ref().unwrap();
    let key = app.master_key.as_ref().unwrap();

    let entry_with_fields = queries::get_entry(db, &entry_id)?;
    let decrypted_fields: Result<Vec<DecryptedField>, String> = entry_with_fields
        .fields
        .iter()
        .map(|f| decrypt_field(key, f))
        .collect();

    Ok(EntryDetailResponse {
        entry: entry_with_fields.entry,
        fields: decrypted_fields?,
    })
}

#[tauri::command]
pub fn list_entries(
    vault_id: Option<String>,
    category: Option<String>,
    trashed: Option<bool>,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<EntrySummary>, String> {
    let app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    require_unlocked(&app)?;
    let db = app.db.as_ref().unwrap();

    queries::list_entries(
        db,
        vault_id.as_deref(),
        category.as_deref(),
        trashed.unwrap_or(false),
    )
}

#[tauri::command]
pub fn update_entry(input: UpdateEntryInput, state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    require_unlocked(&app)?;
    let db = app.db.as_ref().unwrap();
    let key = app.master_key.as_ref().unwrap();

    // If fields are being updated, save old password values to history
    if let Some(ref new_fields) = input.fields {
        let old_entry = queries::get_entry(db, &input.entry_id)?;
        for old_field in &old_entry.fields {
            if old_field.field_type == "password" {
                queries::save_password_history(db, &input.entry_id, &old_field.value)?;
            }
        }

        let encrypted_fields: Result<Vec<FieldInput>, String> = new_fields
            .iter()
            .map(|f| {
                let encrypted = encrypt_field_value(key, &f.field_type, &f.value)?;
                Ok(FieldInput {
                    field_type: f.field_type.clone(),
                    label: f.label.clone(),
                    value: encrypted,
                    sort_order: f.sort_order,
                })
            })
            .collect();

        queries::update_entry(
            db,
            &input.entry_id,
            input.title.as_deref(),
            input.subtitle.as_deref(),
            Some(&encrypted_fields?),
        )?;
    } else {
        queries::update_entry(
            db,
            &input.entry_id,
            input.title.as_deref(),
            input.subtitle.as_deref(),
            None,
        )?;
    }

    Ok(())
}

#[tauri::command]
pub fn trash_entry(entry_id: String, state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    require_unlocked(&app)?;
    let db = app.db.as_ref().unwrap();
    queries::trash_entry(db, &entry_id)
}

#[tauri::command]
pub fn get_category_counts(
    vault_id: Option<String>,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<(String, i64)>, String> {
    let app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    require_unlocked(&app)?;
    let db = app.db.as_ref().unwrap();
    queries::get_category_counts(db, vault_id.as_deref())
}

#[tauri::command]
pub fn generate_password() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+";
    (0..20)
        .map(|_| charset[rng.gen_range(0..charset.len())] as char)
        .collect()
}

#[tauri::command]
pub fn copy_to_clipboard(
    value: String,
    clear_after_ms: Option<u64>,
    app_handle: AppHandle,
) -> Result<(), String> {
    use std::thread;
    use std::time::Duration;

    // Write to system clipboard via arboard
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| format!("Cannot access clipboard: {e}"))?;
    clipboard
        .set_text(&value)
        .map_err(|e| format!("Cannot write to clipboard: {e}"))?;

    let timeout = clear_after_ms.unwrap_or(30000);

    // Emit event to frontend for Toast display
    let _ = app_handle.emit("clipboard:copied", timeout);

    // Spawn background thread to clear clipboard after timeout
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(timeout));
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text("");
        }
        let _ = app_handle.emit("clipboard:cleared", ());
    });

    Ok(())
}

#[tauri::command]
pub fn list_vaults(state: State<'_, Mutex<AppState>>) -> Result<Vec<queries::Vault>, String> {
    let app = state.lock().map_err(|_| "State lock poisoned".to_string())?;
    require_unlocked(&app)?;
    let db = app.db.as_ref().unwrap();
    queries::list_vaults(db)
}
