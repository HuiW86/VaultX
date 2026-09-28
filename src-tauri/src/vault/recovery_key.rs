//! Recovery key encoding and recovery KEK derivation.

use rand::RngCore;
use zeroize::Zeroizing;

use crate::crypto::key_derivation;

pub const RECOVERY_KEY_LEN: usize = 16; // 128 bits
// Salt specifically for recovery key derivation (constant, embedded). The
// recovery key itself carries 128 bits of entropy.
const RECOVERY_SALT: &[u8] = b"vaultx-recovery-key-derivation00";

/// Generate a new random raw recovery key.
pub fn generate_raw() -> Zeroizing<[u8; RECOVERY_KEY_LEN]> {
    let mut raw = Zeroizing::new([0u8; RECOVERY_KEY_LEN]);
    rand::thread_rng().fill_bytes(raw.as_mut());
    raw
}

/// Derive the recovery KEK from the raw recovery key (Argon2id).
pub fn derive_kek(raw: &[u8]) -> Result<Zeroizing<[u8; 32]>, String> {
    key_derivation::derive_key(raw, RECOVERY_SALT)
}

/// Encode bytes as base32 with groups of 4, separated by hyphens for readability.
pub fn encode_grouped(bytes: &[u8]) -> String {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits = 0u16;
    let mut bits_left = 0u8;
    let mut chars = Zeroizing::new(Vec::new());

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

    chars
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

/// Decode a grouped base32 recovery key. Errors never echo input characters (C4).
pub fn decode(input: &str) -> Result<Zeroizing<Vec<u8>>, String> {
    let mut bits = 0u32;
    let mut bits_left = 0u8;
    let mut result = Zeroizing::new(Vec::new());

    for ch in input.chars() {
        if ch == '-' || ch.is_whitespace() {
            continue;
        }
        let ch = ch.to_ascii_uppercase();
        let val = match ch {
            'A'..='Z' => ch as u32 - 'A' as u32,
            '2'..='7' => ch as u32 - '2' as u32 + 26,
            _ => return Err("Invalid recovery key format".to_string()),
        };
        bits = ((bits << 5) | val) & 0xFFFF;
        bits_left += 5;
        if bits_left >= 8 {
            bits_left -= 8;
            result.push(((bits >> bits_left) & 0xFF) as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base32_roundtrip() {
        let bytes = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x23, 0x45, 0x67,
                     0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98];
        let encoded = encode_grouped(&bytes);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(&decoded[..], &bytes);
    }

    #[test]
    fn base32_decode_ignores_hyphens_and_case() {
        let decoded1 = decode("abcd-EFGH-IJKL").unwrap();
        let decoded2 = decode("ABCDEFGHIJKL").unwrap();
        assert_eq!(decoded1, decoded2);
    }

    #[test]
    fn invalid_char_error_does_not_echo_input() {
        let err = decode("ABCD-EF1H").unwrap_err();
        assert_eq!(err, "Invalid recovery key format");
        assert!(!err.contains('1'));
    }

    #[test]
    fn recovery_kek_is_deterministic() {
        let raw = generate_raw();
        assert_eq!(*derive_kek(&*raw).unwrap(), *derive_kek(&*raw).unwrap());
    }
}
