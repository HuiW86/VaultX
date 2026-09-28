//! DEK generation and wrapping (contract: docs/contracts/vault-key-lifecycle.md).
//!
//! The DEK is the SQLCipher key and the field key. KEKs (password- or
//! recovery-derived) only wrap the DEK. Each wrap binds a purpose label as
//! AES-GCM associated data so a recovery wrap can never be used as a password
//! wrap (or vice versa).

use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use zeroize::Zeroizing;

use super::encryption;

pub const DEK_LEN: usize = 32;

/// Purpose label for the DEK wrapped by the password KEK.
pub const PURPOSE_PASSWORD: &[u8] = b"vaultx:dek:password";
/// Purpose label for the DEK wrapped by the recovery KEK.
pub const PURPOSE_RECOVERY: &[u8] = b"vaultx:dek:recovery";

/// Generate a fresh random 256-bit DEK.
pub fn generate_dek() -> Zeroizing<[u8; DEK_LEN]> {
    let mut dek = Zeroizing::new([0u8; DEK_LEN]);
    rand::thread_rng().fill_bytes(dek.as_mut());
    dek
}

/// Wrap the DEK with a KEK; returns base64 text for `.vaultx-meta`.
pub fn wrap_dek(kek: &[u8; 32], dek: &[u8; DEK_LEN], purpose: &[u8]) -> Result<String, String> {
    let wrapped = encryption::encrypt_with_aad(kek, dek, purpose)?;
    Ok(STANDARD.encode(wrapped))
}

/// Unwrap a DEK. Any failure (bad base64, wrong KEK, wrong purpose, bad
/// length) yields the same fixed error so callers can map it to
/// "wrong credential" without leaking details.
pub fn unwrap_dek(
    kek: &[u8; 32],
    wrapped_b64: &str,
    purpose: &[u8],
) -> Result<Zeroizing<[u8; DEK_LEN]>, String> {
    const ERR: &str = "Unable to unwrap vault key";
    let wrapped = STANDARD.decode(wrapped_b64).map_err(|_| ERR.to_string())?;
    let plain = Zeroizing::new(
        encryption::decrypt_with_aad(kek, &wrapped, purpose).map_err(|_| ERR.to_string())?,
    );
    if plain.len() != DEK_LEN {
        return Err(ERR.to_string());
    }
    let mut dek = Zeroizing::new([0u8; DEK_LEN]);
    dek.copy_from_slice(&plain);
    Ok(dek)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_unwrap_roundtrip() {
        let kek = [7u8; 32];
        let dek = generate_dek();
        let w = wrap_dek(&kek, &dek, PURPOSE_PASSWORD).unwrap();
        let back = unwrap_dek(&kek, &w, PURPOSE_PASSWORD).unwrap();
        assert_eq!(*back, *dek);
    }

    #[test]
    fn purpose_is_bound() {
        let kek = [7u8; 32];
        let dek = generate_dek();
        let w = wrap_dek(&kek, &dek, PURPOSE_RECOVERY).unwrap();
        assert!(unwrap_dek(&kek, &w, PURPOSE_PASSWORD).is_err());
    }

    #[test]
    fn wrong_kek_fails_with_fixed_message() {
        let dek = generate_dek();
        let w = wrap_dek(&[1u8; 32], &dek, PURPOSE_PASSWORD).unwrap();
        let err = unwrap_dek(&[2u8; 32], &w, PURPOSE_PASSWORD).unwrap_err();
        assert_eq!(err, "Unable to unwrap vault key");
    }

    #[test]
    fn deks_are_random() {
        assert_ne!(*generate_dek(), *generate_dek());
    }
}
