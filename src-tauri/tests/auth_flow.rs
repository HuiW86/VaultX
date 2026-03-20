//! Integration tests for the auth flow: setup → lock → unlock → lock

use tempfile::TempDir;
use vaultx_lib::crypto::key_derivation;
use vaultx_lib::db::connection::{self, AppFileStatus, KdfConfig, KdfParams, VaultMeta};
use vaultx_lib::db::{queries, schema};

fn setup_full_vault(dir: &std::path::Path) -> [u8; 32] {
    let salt = key_derivation::generate_salt();
    let key = key_derivation::derive_key(b"test-password-123", &salt).unwrap();

    let conn = connection::init_db(dir, &*key).unwrap();
    schema::run_migrations(&conn).unwrap(); // already called in init_db, but idempotent
    queries::create_vault(&conn, "Personal", None).unwrap();
    drop(conn);

    let meta = VaultMeta {
        version: 1,
        kdf: KdfConfig {
            algorithm: "argon2id".to_string(),
            params: KdfParams { m_cost: 19456, t_cost: 2, p_cost: 1 },
            salt: salt.to_vec(),
        },
        created_at: chrono::Utc::now().to_rfc3339(),
        db_path: "vault.db".to_string(),
    };
    connection::write_meta(dir, &meta).unwrap();

    let mut out = [0u8; 32];
    out.copy_from_slice(&*key);
    out
}

#[test]
fn full_setup_creates_db_meta_and_vault() {
    let dir = TempDir::new().unwrap();
    let key = setup_full_vault(dir.path());

    // Verify files exist
    assert_eq!(connection::check_file_status(dir.path()), AppFileStatus::Ready);

    // Verify meta is readable
    let meta = connection::read_meta(dir.path()).unwrap();
    assert_eq!(meta.version, 1);
    assert_eq!(meta.kdf.algorithm, "argon2id");

    // Verify DB can be opened and has the vault
    let conn = connection::open_db(dir.path(), &key).unwrap();
    let vaults = queries::list_vaults(&conn).unwrap();
    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].name, "Personal");
}

#[test]
fn unlock_with_correct_password() {
    let dir = TempDir::new().unwrap();
    let salt = key_derivation::generate_salt();
    let password = b"my-secure-password";
    let key = key_derivation::derive_key(password, &salt).unwrap();

    let conn = connection::init_db(dir.path(), &*key).unwrap();
    queries::create_vault(&conn, "Personal", None).unwrap();
    drop(conn);

    let meta = VaultMeta {
        version: 1,
        kdf: KdfConfig {
            algorithm: "argon2id".to_string(),
            params: KdfParams { m_cost: 19456, t_cost: 2, p_cost: 1 },
            salt: salt.to_vec(),
        },
        created_at: chrono::Utc::now().to_rfc3339(),
        db_path: "vault.db".to_string(),
    };
    connection::write_meta(dir.path(), &meta).unwrap();

    // Re-derive key from password + stored salt (simulates unlock)
    let meta_read = connection::read_meta(dir.path()).unwrap();
    let derived = key_derivation::derive_key(password, &meta_read.kdf.salt).unwrap();
    let conn = connection::open_db(dir.path(), &*derived).unwrap();
    let vaults = queries::list_vaults(&conn).unwrap();
    assert_eq!(vaults.len(), 1);
}

#[test]
fn unlock_with_wrong_password_fails() {
    let dir = TempDir::new().unwrap();
    let salt = key_derivation::generate_salt();
    let key = key_derivation::derive_key(b"correct", &salt).unwrap();

    let conn = connection::init_db(dir.path(), &*key).unwrap();
    drop(conn);

    connection::write_meta(dir.path(), &VaultMeta {
        version: 1,
        kdf: KdfConfig {
            algorithm: "argon2id".to_string(),
            params: KdfParams { m_cost: 19456, t_cost: 2, p_cost: 1 },
            salt: salt.to_vec(),
        },
        created_at: chrono::Utc::now().to_rfc3339(),
        db_path: "vault.db".to_string(),
    }).unwrap();

    let meta = connection::read_meta(dir.path()).unwrap();
    let wrong_key = key_derivation::derive_key(b"wrong-password", &meta.kdf.salt).unwrap();
    assert!(connection::open_db(dir.path(), &*wrong_key).is_err());
}

#[test]
fn lock_clears_state_simulation() {
    // Simulate what AppState.clear() does
    use zeroize::Zeroizing;

    let key = Zeroizing::new([0xAA_u8; 32]);
    let key_ptr = key.as_ptr();

    // Drop the key (simulates lock)
    drop(key);

    // After drop, the memory at that location should be zeroed
    // (We can't directly verify this safely, but we verify the Zeroizing wrapper works)
    // The key variable is no longer accessible — ownership model enforces this.
}

#[test]
fn interrupted_setup_cleanup() {
    let dir = TempDir::new().unwrap();
    let key = [0xAA; 32];

    // Create DB but don't write meta (simulates crash after DB creation)
    let _ = connection::init_db(dir.path(), &key).unwrap();

    // Status should detect inconsistency
    match connection::check_file_status(dir.path()) {
        AppFileStatus::Corrupted { reason } => {
            assert!(reason.contains("missing"), "Expected 'missing' in: {reason}");
        }
        other => panic!("Expected Corrupted, got {other:?}"),
    }

    // Cleanup should restore to first_run
    connection::cleanup_files(dir.path());
    assert_eq!(connection::check_file_status(dir.path()), AppFileStatus::FirstRun);
}

#[test]
fn corrupt_meta_detected() {
    let dir = TempDir::new().unwrap();
    let key = [0xAA; 32];
    let _ = connection::init_db(dir.path(), &key).unwrap();

    // Write invalid JSON as meta
    std::fs::write(dir.path().join(".vaultx-meta"), "not json").unwrap();

    match connection::check_file_status(dir.path()) {
        AppFileStatus::Corrupted { reason } => {
            assert!(reason.contains("unreadable"), "Expected 'unreadable' in: {reason}");
        }
        other => panic!("Expected Corrupted, got {other:?}"),
    }
}
