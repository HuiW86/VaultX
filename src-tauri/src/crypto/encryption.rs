use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, AeadCore, Nonce,
};

// Encrypted field format:
//   version(1) || nonce(12) || ciphertext(variable) || tag(16)
//
// Only used for sensitive field_types: password, hidden, card_number.
// Non-sensitive fields (title, username, url, text) rely on SQLCipher only.
const ENCRYPTION_VERSION: u8 = 1;
const NONCE_LEN: usize = 12;

/// Encrypt plaintext with AES-256-GCM.
/// Returns: version(1) || nonce(12) || ciphertext || tag(16)
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {e}"))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {e}"))?;

    let mut result = Vec::with_capacity(1 + NONCE_LEN + ciphertext.len());
    result.push(ENCRYPTION_VERSION);
    result.extend_from_slice(nonce.as_ref());
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt data produced by encrypt().
/// Expects: version(1) || nonce(12) || ciphertext || tag(16)
pub fn decrypt(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, String> {
    let min_len = 1 + NONCE_LEN + 16; // version + nonce + tag (minimum, empty plaintext)
    if data.len() < min_len {
        return Err("Data too short to be valid ciphertext".to_string());
    }

    let version = data[0];
    if version != ENCRYPTION_VERSION {
        return Err(format!("Unsupported encryption version: {version}"));
    }

    let nonce = Nonce::from_slice(&data[1..1 + NONCE_LEN]);
    let ciphertext = &data[1 + NONCE_LEN..];

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {e}"))?;
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Decryption failed: wrong key or corrupted data".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    fn random_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = random_key();
        let plaintext = b"my-secret-password";
        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let key1 = random_key();
        let key2 = random_key();
        let encrypted = encrypt(&key1, b"secret").unwrap();
        assert!(decrypt(&key2, &encrypted).is_err());
    }

    #[test]
    fn corrupt_data_fails() {
        let key = random_key();
        let mut encrypted = encrypt(&key, b"secret").unwrap();
        // Flip a byte in ciphertext
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF;
        assert!(decrypt(&key, &encrypted).is_err());
    }

    #[test]
    fn empty_plaintext() {
        let key = random_key();
        let encrypted = encrypt(&key, b"").unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, b"");
    }

    #[test]
    fn nonce_uniqueness() {
        let key = random_key();
        let enc1 = encrypt(&key, b"same").unwrap();
        let enc2 = encrypt(&key, b"same").unwrap();
        // Same plaintext + key should produce different ciphertext (different nonce)
        assert_ne!(enc1, enc2);
    }

    #[test]
    fn data_too_short() {
        let key = random_key();
        assert!(decrypt(&key, &[1, 2, 3]).is_err());
    }

    #[test]
    fn wrong_version() {
        let key = random_key();
        let mut encrypted = encrypt(&key, b"test").unwrap();
        encrypted[0] = 99; // Invalid version
        assert!(decrypt(&key, &encrypted).is_err());
    }
}
