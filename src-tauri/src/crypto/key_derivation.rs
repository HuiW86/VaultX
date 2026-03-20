use argon2::{Argon2, Params, Version};
use rand::RngCore;
use zeroize::Zeroizing;

// Argon2id parameters — OWASP 2025 minimum secure
// m=19MiB, t=2 iterations, p=1 parallelism
const M_COST: u32 = 19456; // 19 MiB in KiB
const T_COST: u32 = 2;
const P_COST: u32 = 1;
const KEY_LEN: usize = 32; // 256-bit
const SALT_LEN: usize = 32;

/// Derive a 256-bit key from a password using Argon2id.
/// Returns a zeroizing wrapper that clears the key on drop.
pub fn derive_key(password: &[u8], salt: &[u8]) -> Result<Zeroizing<[u8; KEY_LEN]>, String> {
    let params = Params::new(M_COST, T_COST, P_COST, Some(KEY_LEN))
        .map_err(|e| format!("Invalid Argon2 params: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);

    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon2
        .hash_password_into(password, salt, key.as_mut())
        .map_err(|e| format!("Key derivation failed: {e}"))?;

    Ok(key)
}

/// Generate a cryptographically secure random salt.
pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

/// Verify a password against a known salt by deriving the key and comparing.
pub fn verify_password(
    password: &[u8],
    salt: &[u8],
    expected_key: &[u8; KEY_LEN],
) -> Result<bool, String> {
    let derived = derive_key(password, salt)?;
    // Constant-time comparison to prevent timing attacks
    Ok(constant_time_eq(derived.as_ref(), expected_key))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_roundtrip() {
        let password = b"correct-horse-battery-staple";
        let salt = generate_salt();
        let key1 = derive_key(password, &salt).unwrap();
        let key2 = derive_key(password, &salt).unwrap();
        assert_eq!(key1.as_ref(), key2.as_ref());
    }

    #[test]
    fn wrong_password_different_key() {
        let salt = generate_salt();
        let key1 = derive_key(b"password1", &salt).unwrap();
        let key2 = derive_key(b"password2", &salt).unwrap();
        assert_ne!(key1.as_ref(), key2.as_ref());
    }

    #[test]
    fn salt_uniqueness() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn verify_correct_password() {
        let password = b"test-password";
        let salt = generate_salt();
        let key = derive_key(password, &salt).unwrap();
        assert!(verify_password(password, &salt, &*key).unwrap());
    }

    #[test]
    fn verify_wrong_password() {
        let salt = generate_salt();
        let key = derive_key(b"correct", &salt).unwrap();
        assert!(!verify_password(b"wrong", &salt, &*key).unwrap());
    }
}
