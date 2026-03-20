//! Integration tests for entry CRUD with encryption

use tempfile::TempDir;
use vaultx_lib::crypto::{encryption, key_derivation};
use vaultx_lib::db::connection;
use vaultx_lib::db::queries::{self, FieldInput};

fn setup_db_with_vault(dir: &std::path::Path) -> (rusqlite::Connection, [u8; 32]) {
    let salt = key_derivation::generate_salt();
    let key = key_derivation::derive_key(b"test-pass", &salt).unwrap();
    let conn = connection::init_db(dir.path_join_workaround(), &*key).unwrap();
    queries::create_vault(&conn, "Personal", None).unwrap();
    let mut out = [0u8; 32];
    out.copy_from_slice(&*key);
    (conn, out)
}

// Helper to avoid path issues
trait PathJoin {
    fn path_join_workaround(&self) -> &std::path::Path;
}
impl PathJoin for std::path::Path {
    fn path_join_workaround(&self) -> &std::path::Path {
        self
    }
}

fn setup() -> (TempDir, rusqlite::Connection, [u8; 32], String) {
    let dir = TempDir::new().unwrap();
    let salt = key_derivation::generate_salt();
    let key = key_derivation::derive_key(b"test-pass", &salt).unwrap();
    let conn = connection::init_db(dir.path(), &*key).unwrap();
    let vault = queries::create_vault(&conn, "Personal", None).unwrap();
    let mut out = [0u8; 32];
    out.copy_from_slice(&*key);
    (dir, conn, out, vault.id)
}

#[test]
fn create_entry_with_encrypted_password() {
    let (_dir, conn, key, vault_id) = setup();

    // Encrypt password field value (simulates what commands/entries.rs does)
    let plaintext = "my-secret-password";
    let encrypted = encryption::encrypt(&key, plaintext.as_bytes()).unwrap();

    let fields = vec![
        FieldInput {
            field_type: "username".to_string(),
            label: "Username".to_string(),
            value: b"user@example.com".to_vec(), // not encrypted (non-sensitive)
            sort_order: 0,
        },
        FieldInput {
            field_type: "password".to_string(),
            label: "Password".to_string(),
            value: encrypted.clone(), // encrypted (sensitive)
            sort_order: 1,
        },
    ];

    let created = queries::create_entry(&conn, &vault_id, "login", "Test Site", Some("user@example.com"), &fields).unwrap();
    assert_eq!(created.entry.title, "Test Site");
    assert_eq!(created.fields.len(), 2);

    // Fetch and verify
    let fetched = queries::get_entry(&conn, &created.entry.id).unwrap();

    // Username is plaintext
    assert_eq!(
        String::from_utf8(fetched.fields[0].value.clone()).unwrap(),
        "user@example.com"
    );

    // Password is encrypted — decrypt it
    let decrypted = encryption::decrypt(&key, &fetched.fields[1].value).unwrap();
    assert_eq!(String::from_utf8(decrypted).unwrap(), plaintext);
}

#[test]
fn sensitive_field_not_readable_without_key() {
    let (_dir, conn, key, vault_id) = setup();

    let encrypted = encryption::encrypt(&key, b"secret-value").unwrap();
    let fields = vec![FieldInput {
        field_type: "password".to_string(),
        label: "Password".to_string(),
        value: encrypted,
        sort_order: 0,
    }];

    let created = queries::create_entry(&conn, &vault_id, "login", "Test", None, &fields).unwrap();
    let fetched = queries::get_entry(&conn, &created.entry.id).unwrap();

    // Wrong key cannot decrypt
    let wrong_key = [0xBB; 32];
    assert!(encryption::decrypt(&wrong_key, &fetched.fields[0].value).is_err());
}

#[test]
fn create_list_get_update_trash_flow() {
    let (_dir, conn, _key, vault_id) = setup();

    // Create
    let entry = queries::create_entry(
        &conn, &vault_id, "note", "My Note", Some("First line"), &[
            FieldInput {
                field_type: "text".to_string(),
                label: "Note".to_string(),
                value: b"Hello world".to_vec(),
                sort_order: 0,
            },
        ],
    ).unwrap();

    // List
    let entries = queries::list_entries(&conn, None, None, false).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "My Note");

    // Get
    let detail = queries::get_entry(&conn, &entry.entry.id).unwrap();
    assert_eq!(detail.fields.len(), 1);
    assert_eq!(String::from_utf8(detail.fields[0].value.clone()).unwrap(), "Hello world");

    // Update
    queries::update_entry(
        &conn,
        &entry.entry.id,
        Some("Updated Note"),
        None,
        Some(&[FieldInput {
            field_type: "text".to_string(),
            label: "Note".to_string(),
            value: b"Updated content".to_vec(),
            sort_order: 0,
        }]),
    ).unwrap();

    let updated = queries::get_entry(&conn, &entry.entry.id).unwrap();
    assert_eq!(updated.entry.title, "Updated Note");
    assert_eq!(
        String::from_utf8(updated.fields[0].value.clone()).unwrap(),
        "Updated content"
    );

    // Trash (soft delete)
    queries::trash_entry(&conn, &entry.entry.id).unwrap();
    let active = queries::list_entries(&conn, None, None, false).unwrap();
    assert_eq!(active.len(), 0);
    let trashed = queries::list_entries(&conn, None, None, true).unwrap();
    assert_eq!(trashed.len(), 1);
}

#[test]
fn password_history_saved_on_update() {
    let (_dir, conn, key, vault_id) = setup();

    let old_password = encryption::encrypt(&key, b"old-pass").unwrap();
    let entry = queries::create_entry(
        &conn, &vault_id, "login", "Site", None, &[
            FieldInput {
                field_type: "password".to_string(),
                label: "Password".to_string(),
                value: old_password.clone(),
                sort_order: 0,
            },
        ],
    ).unwrap();

    // Save old password to history (simulates what commands/entries.rs does before update)
    queries::save_password_history(&conn, &entry.entry.id, &old_password).unwrap();

    // Update with new password
    let new_password = encryption::encrypt(&key, b"new-pass").unwrap();
    queries::update_entry(
        &conn,
        &entry.entry.id,
        None,
        None,
        Some(&[FieldInput {
            field_type: "password".to_string(),
            label: "Password".to_string(),
            value: new_password,
            sort_order: 0,
        }]),
    ).unwrap();

    // Verify current password is new
    let detail = queries::get_entry(&conn, &entry.entry.id).unwrap();
    let decrypted = encryption::decrypt(&key, &detail.fields[0].value).unwrap();
    assert_eq!(String::from_utf8(decrypted).unwrap(), "new-pass");

    // Verify history has old password
    let history: Vec<Vec<u8>> = conn
        .prepare("SELECT value FROM password_history WHERE entry_id = ?1")
        .unwrap()
        .query_map(rusqlite::params![entry.entry.id], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert_eq!(history.len(), 1);
    let old_decrypted = encryption::decrypt(&key, &history[0]).unwrap();
    assert_eq!(String::from_utf8(old_decrypted).unwrap(), "old-pass");
}

#[test]
fn category_filter_works() {
    let (_dir, conn, _key, vault_id) = setup();

    queries::create_entry(&conn, &vault_id, "login", "Site A", None, &[]).unwrap();
    queries::create_entry(&conn, &vault_id, "login", "Site B", None, &[]).unwrap();
    queries::create_entry(&conn, &vault_id, "card", "My Card", None, &[]).unwrap();
    queries::create_entry(&conn, &vault_id, "note", "A Note", None, &[]).unwrap();

    let all = queries::list_entries(&conn, None, None, false).unwrap();
    assert_eq!(all.len(), 4);

    let logins = queries::list_entries(&conn, None, Some("login"), false).unwrap();
    assert_eq!(logins.len(), 2);

    let cards = queries::list_entries(&conn, None, Some("card"), false).unwrap();
    assert_eq!(cards.len(), 1);

    let counts = queries::get_category_counts(&conn, None).unwrap();
    let login_count = counts.iter().find(|(c, _)| c == "login").map(|(_, n)| *n).unwrap_or(0);
    assert_eq!(login_count, 2);
}
