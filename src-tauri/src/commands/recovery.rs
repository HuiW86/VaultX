use std::sync::Mutex;
use base64::Engine;
use tauri::State;
use serde::Serialize;
use rand::RngCore;
use zeroize::Zeroizing;

use crate::crypto::{encryption, key_derivation};
use crate::db::connection;
use crate::state::AppState;

const RECOVERY_KEY_LEN: usize = 16; // 128 bits
// Salt specifically for recovery key derivation (constant, embedded)
const RECOVERY_SALT: &[u8] = b"vaultx-recovery-key-derivation00";

#[derive(Debug, Serialize)]
pub struct RecoveryKitResult {
    /// Base32-encoded recovery key for the user to save
    pub recovery_key: String,
    /// Full .txt content ready for download
    pub file_content: String,
}

/// Derive a 256-bit encryption key from the recovery key using Argon2id
fn derive_recovery_encryption_key(
    recovery_key: &[u8],
) -> Result<Zeroizing<[u8; 32]>, String> {
    key_derivation::derive_key(recovery_key, RECOVERY_SALT)
}

/// Encode bytes as base32 with groups of 4, separated by hyphens for readability
fn encode_base32_grouped(bytes: &[u8]) -> String {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits = 0u16;
    let mut bits_left = 0u8;
    let mut chars = Vec::new();

    for &byte in bytes {
        bits = (bits << 8) | byte as u16;
        bits_left += 8;
        while bits_left >= 5 {
            bits_left -= 5;
            chars.push(alphabet[((bits >> bits_left) & 0x1F) as usize] as char);
        }
    }
    if bits_left > 0 {
        chars.push(alphabet[((bits << (5 - bits_left)) & 0x1F) as usize] as char);
    }

    // Group into chunks of 4 separated by hyphens
    chars
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

/// Decode base32 grouped string back to bytes
fn decode_base32(input: &str) -> Result<Vec<u8>, String> {
    let clean: String = input.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let clean = clean.to_uppercase();

    let mut bits = 0u32;
    let mut bits_left = 0u8;
    let mut result = Vec::new();

    for ch in clean.chars() {
        let val = match ch {
            'A'..='Z' => ch as u32 - 'A' as u32,
            '2'..='7' => ch as u32 - '2' as u32 + 26,
            _ => return Err(format!("Invalid base32 character: {ch}")),
        };
        bits = (bits << 5) | val;
        bits_left += 5;
        if bits_left >= 8 {
            bits_left -= 8;
            result.push(((bits >> bits_left) & 0xFF) as u8);
        }
    }

    Ok(result)
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
         4. Anyone with this key can access your vault — keep it secret\n\
         \n\
         Generated: {date}\n",
        date = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"),
    )
}

/// Generate a recovery kit: create recovery key, encrypt master_key with it,
/// store the encrypted blob in .vaultx-meta, return key + file content.
#[tauri::command]
pub fn generate_recovery_kit(
    state: State<'_, Mutex<AppState>>,
) -> Result<RecoveryKitResult, String> {
    let app = state.lock().map_err(|_| "Lock poisoned".to_string())?;
    let master_key = app.master_key.as_ref().ok_or("Vault is locked")?;

    // Generate 128-bit random recovery key
    let mut raw_key = [0u8; RECOVERY_KEY_LEN];
    rand::thread_rng().fill_bytes(&mut raw_key);

    let recovery_key_str = encode_base32_grouped(&raw_key);

    // Derive encryption key from recovery key
    let enc_key = derive_recovery_encryption_key(&raw_key)?;

    // Encrypt master_key with the derived key
    let encrypted_master = encryption::encrypt(&enc_key, master_key.as_ref())?;

    // Store encrypted recovery blob in meta file
    let mut meta = connection::read_meta(&app.data_dir)?;
    meta.recovery_blob = Some(base64::engine::general_purpose::STANDARD.encode(&encrypted_master));
    connection::write_meta(&app.data_dir, &meta)?;

    // Zero raw key
    drop(enc_key);

    let file_content = generate_file_content(&recovery_key_str);

    Ok(RecoveryKitResult {
        recovery_key: recovery_key_str,
        file_content,
    })
}

/// Recover vault: decode recovery key, decrypt master_key, re-encrypt DB with new password.
#[tauri::command]
pub fn recover_with_key(
    recovery_key: String,
    new_password: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let mut app = state.lock().map_err(|_| "Lock poisoned".to_string())?;

    // Read meta to get recovery blob
    let meta = connection::read_meta(&app.data_dir)?;
    let blob_b64 = meta
        .recovery_blob
        .as_ref()
        .ok_or("No recovery kit has been set up")?;

    let encrypted_master = base64::engine::general_purpose::STANDARD
        .decode(blob_b64)
        .map_err(|e| format!("Invalid recovery blob: {e}"))?;

    // Decode recovery key from base32
    let raw_key = decode_base32(&recovery_key)?;
    if raw_key.len() != RECOVERY_KEY_LEN {
        return Err("Invalid recovery key length".into());
    }

    // Derive decryption key from recovery key
    let dec_key = derive_recovery_encryption_key(&raw_key)?;

    // Decrypt master_key
    let master_bytes = encryption::decrypt(&dec_key, &encrypted_master)
        .map_err(|_| "Invalid recovery key")?;

    if master_bytes.len() != 32 {
        return Err("Recovered key has invalid length".into());
    }

    let mut old_master = Zeroizing::new([0u8; 32]);
    old_master.copy_from_slice(&master_bytes);

    // Open DB with old master key
    let conn = connection::open_db(&app.data_dir, &old_master)?;

    // Derive new key from new password
    let new_salt = key_derivation::generate_salt();
    let new_master = key_derivation::derive_key(new_password.as_bytes(), &new_salt)?;

    // Re-key the database with new master key
    let new_hex = hex_encode(new_master.as_ref());
    conn.execute_batch(&format!("PRAGMA rekey = \"x'{new_hex}'\";"))
        .map_err(|e| format!("Failed to re-key database: {e}"))?;

    // Update meta with new salt, clear recovery blob (user should generate new one)
    let mut updated_meta = connection::read_meta(&app.data_dir)?;
    updated_meta.kdf.salt = new_salt.to_vec();
    updated_meta.recovery_blob = None;
    connection::write_meta(&app.data_dir, &updated_meta)?;

    // Store new state
    app.db = Some(conn);
    app.master_key = Some(new_master);
    app.touch_activity();

    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base32_roundtrip() {
        let bytes = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x23, 0x45, 0x67,
                     0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98];
        let encoded = encode_base32_grouped(&bytes);
        let decoded = decode_base32(&encoded).unwrap();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn base32_decode_ignores_hyphens() {
        let encoded = "ABCD-EFGH-IJKL";
        let decoded1 = decode_base32(encoded).unwrap();
        let decoded2 = decode_base32("ABCDEFGHIJKL").unwrap();
        assert_eq!(decoded1, decoded2);
    }

    #[test]
    fn recovery_key_encrypt_decrypt() {
        let mut raw_key = [0u8; RECOVERY_KEY_LEN];
        rand::thread_rng().fill_bytes(&mut raw_key);

        let enc_key = derive_recovery_encryption_key(&raw_key).unwrap();
        let secret = b"this is the master key bytes!!!!"; // 32 bytes
        let encrypted = encryption::encrypt(&enc_key, secret).unwrap();

        // Derive again from same raw key
        let dec_key = derive_recovery_encryption_key(&raw_key).unwrap();
        let decrypted = encryption::decrypt(&dec_key, &encrypted).unwrap();
        assert_eq!(&decrypted, secret);
    }
}
