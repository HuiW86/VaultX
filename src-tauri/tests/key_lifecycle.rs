//! Vault key lifecycle tests (docs/contracts/vault-key-lifecycle.md, C1–C8).
//!
//! All tests use temp directories and the in-memory fake Keychain.

use std::cell::{Cell, RefCell};
use std::fs;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine};
use tempfile::TempDir;
use vaultx_lib::crypto::{encryption, key_derivation, key_wrap};
use vaultx_lib::db::connection::{self, KdfConfig, VaultMeta};
use vaultx_lib::db::queries::{self, FieldInput};
use vaultx_lib::vault::keystore::{KeyStore, MemoryKeyStore};
use vaultx_lib::vault::lifecycle::{self, RecoverFailure, UnlockFailure};
use vaultx_lib::vault::migration::{self, VaultPlaintexts};
use vaultx_lib::vault::{no_faults, recovery_key};

const OLD_PW: &[u8] = b"old-password-123";
const NEW_PW: &[u8] = b"new-password-456";

// ---------- fixtures ----------

fn field(field_type: &str, value: &[u8], key: &[u8; 32], order: i32) -> FieldInput {
    let value = if encryption::is_sensitive_field_type(field_type) {
        encryption::encrypt(key, value).unwrap()
    } else {
        value.to_vec()
    };
    FieldInput { field_type: field_type.into(), label: field_type.into(), value, sort_order: order }
}

/// Fill a vault with every kind of data C7 cares about: all three sensitive
/// field types, password_history rows and a trashed entry.
fn populate(conn: &rusqlite::Connection, key: &[u8; 32]) {
    let vault_id = queries::list_vaults(conn).unwrap()[0].id.clone();

    let login = queries::create_entry(
        conn,
        &vault_id,
        "login",
        "GitHub",
        Some("octo"),
        &[
            field("username", b"octo@example.com", key, 0),
            field("password", b"pw-v1", key, 1),
            field("hidden", b"totp-seed-secret", key, 2),
        ],
    )
    .unwrap();
    // Two password changes -> two history rows (history stores ciphertext).
    for next in [&b"pw-v2"[..], &b"pw-v3"[..]] {
        let old = queries::get_entry(conn, &login.entry.id).unwrap();
        for f in old.fields.iter().filter(|f| f.field_type == "password") {
            queries::save_password_history(conn, &login.entry.id, &f.value).unwrap();
        }
        queries::update_entry(
            conn,
            &login.entry.id,
            None,
            None,
            Some(&[
                field("username", b"octo@example.com", key, 0),
                field("password", next, key, 1),
                field("hidden", b"totp-seed-secret", key, 2),
            ]),
        )
        .unwrap();
    }

    queries::create_entry(
        conn,
        &vault_id,
        "card",
        "Visa",
        None,
        &[field("card_number", b"4111111111111111", key, 0), field("hidden", b"123", key, 1)],
    )
    .unwrap();

    let trashed = queries::create_entry(
        conn,
        &vault_id,
        "login",
        "Old account",
        None,
        &[field("password", b"trashed-secret", key, 0), field("card_number", b"5500000000000004", key, 1)],
    )
    .unwrap();
    let old = queries::get_entry(conn, &trashed.entry.id).unwrap();
    queries::save_password_history(conn, &trashed.entry.id, &old.fields[0].value).unwrap();
    queries::trash_entry(conn, &trashed.entry.id).unwrap();
}

/// Decrypt everything with `key` and check the fixture is fully present.
fn snapshot(conn: &rusqlite::Connection, key: &[u8; 32]) -> VaultPlaintexts {
    let s = migration::collect_plaintexts(conn, key).expect("all data must decrypt");
    assert_eq!(s.counts.entries, 3);
    assert_eq!(s.counts.trashed_entries, 1);
    assert_eq!(s.counts.password_history, 3);
    assert_eq!(s.counts.fields, 7);
    let values: Vec<&[u8]> = s.fields.values().map(|v| v.as_slice()).collect();
    for expected in [&b"pw-v3"[..], b"totp-seed-secret", b"4111111111111111", b"trashed-secret", b"5500000000000004"] {
        assert!(values.contains(&expected), "missing field value");
    }
    let history: Vec<&[u8]> = s.history.values().map(|v| v.as_slice()).collect();
    for expected in [&b"pw-v1"[..], b"pw-v2", b"trashed-secret"] {
        assert!(history.contains(&expected), "missing history value");
    }
    s
}

/// New v2 vault with fixture data and a recovery kit.
/// Returns (baseline plaintexts, recovery key text).
fn make_v2_vault(dir: &Path) -> (VaultPlaintexts, String) {
    let unlocked = lifecycle::create_vault(dir, OLD_PW).unwrap();
    populate(&unlocked.conn, &unlocked.key);
    let baseline = snapshot(&unlocked.conn, &unlocked.key);
    let kit = lifecycle::generate_recovery_kit(dir, &unlocked.key, false, &no_faults).unwrap();
    (baseline, kit)
}

/// Legacy v1 vault exactly as the pre-DEK code wrote it: the password-derived
/// key encrypts the DB and fields, recovery_blob wraps that key.
/// Returns (baseline plaintexts, recovery key text, legacy key).
fn make_v1_vault(dir: &Path) -> (VaultPlaintexts, String, [u8; 32]) {
    let salt = key_derivation::generate_salt();
    let legacy = key_derivation::derive_key(OLD_PW, &salt).unwrap();
    let conn = connection::init_db(dir, &legacy).unwrap();
    queries::create_vault(&conn, "Personal", None).unwrap();
    populate(&conn, &legacy);
    let baseline = snapshot(&conn, &legacy);
    drop(conn);

    let raw = recovery_key::generate_raw();
    let rkek = recovery_key::derive_kek(&*raw).unwrap();
    let blob = encryption::encrypt(&rkek, &*legacy).unwrap();
    let meta = VaultMeta {
        version: 1,
        kdf: KdfConfig::argon2id(&salt),
        created_at: "2026-01-01T00:00:00Z".into(),
        db_path: "vault.db".into(),
        recovery_blob: Some(STANDARD.encode(blob)),
        dek_wrapped_by_password: None,
        dek_wrapped_by_recovery: None,
        keychain_invalidation_pending: false,
    };
    connection::write_meta(dir, &meta).unwrap();
    let mut key = [0u8; 32];
    key.copy_from_slice(&*legacy);
    (baseline, recovery_key::encode_grouped(&*raw), key)
}

fn copy_dir(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

fn set_touch_id_setting(dir: &Path, enabled: bool) {
    let json = serde_json::json!({
        "auto_lock_timeout_minutes": 480, "lock_on_sleep": true, "clipboard_clear_seconds": 30,
        "touch_id_enabled": enabled, "theme": "dark", "start_at_login": false,
        "show_in_menu_bar": false, "language": "en"
    });
    fs::write(dir.join(".vaultx-settings"), json.to_string()).unwrap();
}

fn touch_id_setting(dir: &Path) -> bool {
    let s = fs::read_to_string(dir.join(".vaultx-settings")).unwrap();
    serde_json::from_str::<serde_json::Value>(&s).unwrap()["touch_id_enabled"].as_bool().unwrap()
}

fn db_bytes(dir: &Path) -> Vec<u8> {
    fs::read(connection::db_file_path(dir).unwrap()).unwrap()
}

/// Password unlock after a "restart"; returns the decrypted data on success.
fn try_password(dir: &Path, pw: &[u8]) -> Option<VaultPlaintexts> {
    let ks = MemoryKeyStore::new();
    match lifecycle::unlock_with_password(dir, pw, &ks, &no_faults) {
        Ok(u) => Some(migration::collect_plaintexts(&u.conn, &u.key).expect("C7: all data readable")),
        Err(UnlockFailure::WrongCredential) => None,
        Err(UnlockFailure::Other(e)) => panic!("unexpected unlock error: {e}"),
    }
}

/// Recovery-key check on a scratch copy (recovery consumes the kit).
fn try_recovery_key(dir: &Path, key: &str) -> Option<VaultPlaintexts> {
    let scratch = TempDir::new().unwrap();
    copy_dir(dir, scratch.path());
    let ks = MemoryKeyStore::new();
    match lifecycle::recover(scratch.path(), key, b"probe-password", &ks, &no_faults) {
        Ok(o) => Some(migration::collect_plaintexts(&o.unlocked.conn, &o.unlocked.key).expect("C7: all data readable")),
        Err(RecoverFailure::InvalidKey) | Err(RecoverFailure::NoRecoveryKit) => None,
        Err(RecoverFailure::Other(e)) => panic!("unexpected recover error: {e}"),
    }
}

/// Hook recording every commit point reached.
fn recording_hook<'a>(log: &'a RefCell<Vec<&'static str>>) -> impl Fn(&'static str) -> Result<(), String> + 'a {
    move |p| {
        log.borrow_mut().push(p);
        Ok(())
    }
}

/// Hook that "crashes" at the n-th commit point (0-based).
fn crash_at(n: usize, counter: &Cell<usize>) -> impl Fn(&'static str) -> Result<(), String> + '_ {
    move |_| {
        let i = counter.get();
        counter.set(i + 1);
        if i == n { Err("injected crash".into()) } else { Ok(()) }
    }
}

// ---------- create / unlock (C7) ----------

#[test]
fn create_vault_writes_v2_meta_and_all_data_readable_after_unlock() {
    let dir = TempDir::new().unwrap();
    let (baseline, _) = make_v2_vault(dir.path());

    let meta = connection::read_meta(dir.path()).unwrap();
    assert_eq!(meta.version, 2);
    assert!(meta.dek_wrapped_by_password.is_some());
    assert!(meta.dek_wrapped_by_recovery.is_some());
    assert!(meta.recovery_blob.is_none());

    let got = try_password(dir.path(), OLD_PW).expect("password opens");
    assert_eq!(got, baseline);
}

#[test]
fn password_kek_is_not_the_dek() {
    let dir = TempDir::new().unwrap();
    make_v2_vault(dir.path());
    let meta = connection::read_meta(dir.path()).unwrap();
    let kek = key_derivation::derive_key(OLD_PW, &meta.kdf.salt).unwrap();
    assert!(connection::open_db(dir.path(), &kek).is_err(), "DB must be keyed by the DEK, not the KEK");
}

#[test]
fn wrong_password_is_rejected_without_leaking_it() {
    let dir = TempDir::new().unwrap();
    make_v2_vault(dir.path());
    let ks = MemoryKeyStore::new();
    let err = lifecycle::unlock_with_password(dir.path(), b"not-the-password", &ks, &no_faults).unwrap_err();
    assert_eq!(err, UnlockFailure::WrongCredential);
    assert!(!format!("{err:?}").contains("not-the-password"));
}

// ---------- recovery (C2, C5, C6, C7) ----------

#[test]
fn recovery_rewraps_only_and_keeps_all_data() {
    let dir = TempDir::new().unwrap();
    let (baseline, kit) = make_v2_vault(dir.path());
    let before = db_bytes(dir.path());

    let ks = MemoryKeyStore::new();
    ks.put_raw(&[9u8; 32]);
    set_touch_id_setting(dir.path(), true);

    let outcome = lifecycle::recover(dir.path(), &kit, NEW_PW, &ks, &no_faults).unwrap();
    assert!(outcome.keychain_error.is_none());
    assert_eq!(migration::collect_plaintexts(&outcome.unlocked.conn, &outcome.unlocked.key).unwrap(), baseline);
    drop(outcome);

    // C2: DB file untouched.
    assert_eq!(db_bytes(dir.path()), before, "recovery must not rewrite the DB");
    // C5: kit invalidated.
    let meta = connection::read_meta(dir.path()).unwrap();
    assert!(meta.dek_wrapped_by_recovery.is_none());
    assert!(try_recovery_key(dir.path(), &kit).is_none());
    // C6: Keychain copy deleted and Touch ID disabled.
    assert!(!ks.has_item());
    assert!(!touch_id_setting(dir.path()));
    assert!(!meta.keychain_invalidation_pending);
    // Credentials.
    assert!(try_password(dir.path(), OLD_PW).is_none());
    assert_eq!(try_password(dir.path(), NEW_PW).unwrap(), baseline);
}

#[test]
fn failed_recovery_keeps_old_kit_and_password() {
    let dir = TempDir::new().unwrap();
    let (baseline, kit) = make_v2_vault(dir.path());
    let ks = MemoryKeyStore::new();

    let wrong = recovery_key::encode_grouped(&[0x42; 16]);
    let err = lifecycle::recover(dir.path(), &wrong, NEW_PW, &ks, &no_faults).unwrap_err();
    assert_eq!(err, RecoverFailure::InvalidKey);
    // C4: fixed message, no key characters.
    assert_eq!(err.message(), "Invalid recovery key");
    let bad_chars = lifecycle::recover(dir.path(), "ZZZZ-0189-QQQQ", NEW_PW, &ks, &no_faults).unwrap_err();
    assert_eq!(bad_chars, RecoverFailure::InvalidKey);
    assert!(!bad_chars.message().contains("0189"));

    // C5 failure path: old kit and old password still work.
    assert_eq!(try_recovery_key(dir.path(), &kit).unwrap(), baseline);
    assert_eq!(try_password(dir.path(), OLD_PW).unwrap(), baseline);
}

#[test]
fn recovery_reports_keychain_delete_failure_and_retries_later() {
    let dir = TempDir::new().unwrap();
    let (baseline, kit) = make_v2_vault(dir.path());
    let ks = MemoryKeyStore::new();
    ks.put_raw(&[9u8; 32]);
    ks.fail_delete.set(true);
    set_touch_id_setting(dir.path(), true);

    let outcome = lifecycle::recover(dir.path(), &kit, NEW_PW, &ks, &no_faults).unwrap();
    assert!(outcome.keychain_error.is_some(), "C6: deletion failure must be reported");
    drop(outcome);
    assert!(!touch_id_setting(dir.path()));
    assert!(connection::read_meta(dir.path()).unwrap().keychain_invalidation_pending);

    // Touch ID unlock is refused while invalidation is pending.
    assert!(lifecycle::unlock_with_keystore(dir.path(), &ks).is_err());

    // Next password unlock retries and completes the invalidation.
    ks.fail_delete.set(false);
    let u = lifecycle::unlock_with_password(dir.path(), NEW_PW, &ks, &no_faults).unwrap();
    assert_eq!(migration::collect_plaintexts(&u.conn, &u.key).unwrap(), baseline);
    assert!(!ks.has_item());
    assert!(!connection::read_meta(dir.path()).unwrap().keychain_invalidation_pending);
}

// ---------- Touch ID (C6) ----------

#[test]
fn touch_id_stores_the_dek_and_unlocks() {
    let dir = TempDir::new().unwrap();
    let (baseline, _) = make_v2_vault(dir.path());
    let ks = MemoryKeyStore::new();
    let u = lifecycle::unlock_with_password(dir.path(), OLD_PW, &ks, &no_faults).unwrap();
    lifecycle::enable_touch_id(dir.path(), &u.key, u.legacy, &ks).unwrap();

    let stored = ks.read().unwrap();
    assert_eq!(stored.as_slice(), &u.key[..], "Keychain holds the DEK");
    let meta = connection::read_meta(dir.path()).unwrap();
    let kek = key_derivation::derive_key(OLD_PW, &meta.kdf.salt).unwrap();
    assert_ne!(stored.as_slice(), &kek[..], "never the password KEK");
    drop(u);

    let bio = lifecycle::unlock_with_keystore(dir.path(), &ks).unwrap();
    assert_eq!(migration::collect_plaintexts(&bio.conn, &bio.key).unwrap(), baseline);
}

#[test]
fn touch_id_refused_in_legacy_mode_and_on_v1_vaults() {
    let dir = TempDir::new().unwrap();
    let (_, _, legacy) = make_v1_vault(dir.path());
    let ks = MemoryKeyStore::new();
    ks.put_raw(&legacy); // what the old code stored
    assert!(lifecycle::unlock_with_keystore(dir.path(), &ks).is_err());
    assert!(lifecycle::enable_touch_id(dir.path(), &legacy, true, &ks).is_err());
}

// ---------- password change (C1, C2) ----------

#[test]
fn change_password_rewraps_only_and_keeps_recovery_kit() {
    let dir = TempDir::new().unwrap();
    let (baseline, kit) = make_v2_vault(dir.path());
    let before = db_bytes(dir.path());

    assert_eq!(
        lifecycle::change_password(dir.path(), b"wrong", NEW_PW, &no_faults).unwrap_err(),
        UnlockFailure::WrongCredential
    );
    lifecycle::change_password(dir.path(), OLD_PW, NEW_PW, &no_faults).unwrap();

    assert_eq!(db_bytes(dir.path()), before);
    assert!(try_password(dir.path(), OLD_PW).is_none());
    assert_eq!(try_password(dir.path(), NEW_PW).unwrap(), baseline);
    assert_eq!(try_recovery_key(dir.path(), &kit).unwrap(), baseline);
}

// ---------- v1 -> v2 migration (C8, C7) ----------

#[test]
fn v1_vault_is_migrated_on_password_unlock() {
    let dir = TempDir::new().unwrap();
    let (baseline, v1_kit, legacy) = make_v1_vault(dir.path());
    let ks = MemoryKeyStore::new();
    ks.put_raw(&legacy);
    set_touch_id_setting(dir.path(), true);

    let u = lifecycle::unlock_with_password(dir.path(), OLD_PW, &ks, &no_faults).unwrap();
    assert!(!u.legacy);
    assert_eq!(migration::collect_plaintexts(&u.conn, &u.key).unwrap(), baseline);
    assert_ne!(&u.key[..], &legacy[..], "new random DEK");
    drop(u);

    let meta = connection::read_meta(dir.path()).unwrap();
    assert_eq!(meta.version, 2);
    assert_eq!(meta.db_path, connection::MIGRATED_DB_FILENAME);
    assert!(meta.recovery_blob.is_none());
    assert!(!dir.path().join(connection::DB_FILENAME).exists(), "legacy DB removed after commit");
    // Legacy Keychain copy removed, Touch ID disabled (C6).
    assert!(!ks.has_item());
    assert!(!touch_id_setting(dir.path()));
    // The v1 recovery kit wrapped the legacy key; it cannot open v2.
    assert!(try_recovery_key(dir.path(), &v1_kit).is_none());
    assert_eq!(try_password(dir.path(), OLD_PW).unwrap(), baseline);
}

#[test]
fn failed_migration_falls_back_to_legacy_mode_with_all_data() {
    let dir = TempDir::new().unwrap();
    let (baseline, _, legacy) = make_v1_vault(dir.path());
    let ks = MemoryKeyStore::new();
    let always_fail = |_: &'static str| -> Result<(), String> { Err("disk full".into()) };

    let u = lifecycle::unlock_with_password(dir.path(), OLD_PW, &ks, &always_fail).unwrap();
    assert!(u.legacy);
    assert_eq!(&u.key[..], &legacy[..]);
    assert_eq!(migration::collect_plaintexts(&u.conn, &u.key).unwrap(), baseline);
    // Legacy mode refuses operations that would persist the legacy key.
    assert!(lifecycle::generate_recovery_kit(dir.path(), &u.key, true, &no_faults).is_err());
    drop(u);
    assert_eq!(connection::read_meta(dir.path()).unwrap().version, 1);

    // Next unlock migrates.
    let u = lifecycle::unlock_with_password(dir.path(), OLD_PW, &ks, &no_faults).unwrap();
    assert!(!u.legacy);
    assert_eq!(migration::collect_plaintexts(&u.conn, &u.key).unwrap(), baseline);
}

#[test]
fn v1_recovery_migrates_and_invalidates_kit() {
    let dir = TempDir::new().unwrap();
    let (baseline, kit, _) = make_v1_vault(dir.path());
    let ks = MemoryKeyStore::new();
    let o = lifecycle::recover(dir.path(), &kit, NEW_PW, &ks, &no_faults).unwrap();
    assert_eq!(migration::collect_plaintexts(&o.unlocked.conn, &o.unlocked.key).unwrap(), baseline);
    drop(o);
    let meta = connection::read_meta(dir.path()).unwrap();
    assert_eq!(meta.version, 2);
    assert!(meta.dek_wrapped_by_recovery.is_none() && meta.recovery_blob.is_none());
    assert!(try_recovery_key(dir.path(), &kit).is_none());
    assert!(try_password(dir.path(), OLD_PW).is_none());
    assert_eq!(try_password(dir.path(), NEW_PW).unwrap(), baseline);
}

// ---------- fault injection at every commit point (C1 + C7) ----------

/// Run `flow` once on a copy with a recording hook to enumerate commit
/// points, then for each point run it on a fresh copy crashing there, "restart"
/// and require at least one credential to open the vault with all data.
/// `expect_new` says, per crash index, whether the new credential should work.
fn fault_matrix<F>(
    template: &Path,
    baseline: &VaultPlaintexts,
    flow: F,
    old_pw: &[u8],
    new_pw: Option<&[u8]>,
    recovery: Option<&str>,
) -> Vec<&'static str>
where
    F: Fn(&Path, &dyn Fn(&'static str) -> Result<(), String>) -> Result<(), String>,
{
    let log = RefCell::new(Vec::new());
    {
        let scratch = TempDir::new().unwrap();
        copy_dir(template, scratch.path());
        flow(scratch.path(), &recording_hook(&log)).expect("dry run succeeds");
    }
    let points = log.into_inner();
    assert!(!points.is_empty());

    for (n, point) in points.iter().enumerate() {
        let dir = TempDir::new().unwrap();
        copy_dir(template, dir.path());
        let counter = Cell::new(0);
        let res = flow(dir.path(), &crash_at(n, &counter));
        assert!(res.is_err(), "crash at {point} must abort the flow");

        // Restart: only on-disk state survives.
        let old = try_password(dir.path(), old_pw);
        let new = new_pw.and_then(|pw| try_password(dir.path(), pw));
        let rec = recovery.and_then(|k| try_recovery_key(dir.path(), k));
        let working: Vec<&VaultPlaintexts> = [&old, &new, &rec].into_iter().flatten().collect();
        assert!(!working.is_empty(), "C1 violated: no credential works after crash at {point}");
        for data in working {
            assert_eq!(data, baseline, "C7 violated: data differs after crash at {point}");
        }
        // Exactly one of old/new password is valid (the commit is atomic).
        if new_pw.is_some() {
            assert!(old.is_some() ^ new.is_some(), "ambiguous password state after crash at {point}");
        }
    }
    points
}

#[test]
fn fault_injection_v2_recovery() {
    let template = TempDir::new().unwrap();
    let (baseline, kit) = make_v2_vault(template.path());
    let points = fault_matrix(
        template.path(),
        &baseline,
        |dir, hook| {
            let ks = MemoryKeyStore::new();
            lifecycle::recover(dir, &kit, NEW_PW, &ks, hook).map(|_| ()).map_err(|e| e.message())
        },
        OLD_PW,
        Some(NEW_PW),
        Some(&kit),
    );
    assert!(points.contains(&"meta:tmp_written") && points.contains(&"recover:committed"));
}

#[test]
fn fault_injection_password_change() {
    let template = TempDir::new().unwrap();
    let (baseline, kit) = make_v2_vault(template.path());
    fault_matrix(
        template.path(),
        &baseline,
        |dir, hook| lifecycle::change_password(dir, OLD_PW, NEW_PW, hook).map_err(|e| format!("{e:?}")),
        OLD_PW,
        Some(NEW_PW),
        Some(&kit),
    );
}

#[test]
fn fault_injection_generate_recovery_kit() {
    let template = TempDir::new().unwrap();
    let (baseline, old_kit) = make_v2_vault(template.path());
    fault_matrix(
        template.path(),
        &baseline,
        |dir, hook| {
            let ks = MemoryKeyStore::new();
            let u = lifecycle::unlock_with_password(dir, OLD_PW, &ks, &no_faults).map_err(|e| format!("{e:?}"))?;
            lifecycle::generate_recovery_kit(dir, &u.key, false, hook).map(|_| ())
        },
        OLD_PW,
        None,
        Some(&old_kit),
    );
}

#[test]
fn fault_injection_v1_migration() {
    let template = TempDir::new().unwrap();
    let (baseline, kit, legacy) = make_v1_vault(template.path());
    let points = fault_matrix(
        template.path(),
        &baseline,
        |dir, hook| migration::migrate_v1(dir, &legacy, OLD_PW, hook).map(|_| ()),
        OLD_PW,
        None,
        Some(&kit),
    );
    for p in ["migrate:before_export", "migrate:exported", "migrate:reencrypted", "migrate:verified", "meta:tmp_written", "migrate:committed"] {
        assert!(points.contains(&p), "missing commit point {p}");
    }
}

#[test]
fn fault_injection_v1_recovery() {
    let template = TempDir::new().unwrap();
    let (baseline, kit, _) = make_v1_vault(template.path());
    fault_matrix(
        template.path(),
        &baseline,
        |dir, hook| {
            let ks = MemoryKeyStore::new();
            lifecycle::recover(dir, &kit, NEW_PW, &ks, hook).map(|_| ()).map_err(|e| e.message())
        },
        OLD_PW,
        Some(NEW_PW),
        Some(&kit),
    );
}

#[test]
fn reconcile_removes_partial_shadow_but_never_the_live_db() {
    let dir = TempDir::new().unwrap();
    let (baseline, _, _) = make_v1_vault(dir.path());
    fs::write(dir.path().join(connection::MIGRATED_DB_FILENAME), b"partial garbage").unwrap();
    fs::write(dir.path().join(".vaultx-meta.tmp"), b"{half").unwrap();
    lifecycle::reconcile(dir.path()).unwrap();
    assert!(!dir.path().join(connection::MIGRATED_DB_FILENAME).exists());
    assert!(!dir.path().join(".vaultx-meta.tmp").exists());
    assert!(dir.path().join(connection::DB_FILENAME).exists());
    assert_eq!(try_password(dir.path(), OLD_PW).unwrap(), baseline);
}

// ---------- C4 ----------

#[test]
fn meta_and_errors_contain_no_secrets() {
    let dir = TempDir::new().unwrap();
    let (_, kit) = make_v2_vault(dir.path());
    let meta_text = fs::read_to_string(dir.path().join(".vaultx-meta")).unwrap();
    assert!(!meta_text.contains(std::str::from_utf8(OLD_PW).unwrap()));
    assert!(!meta_text.contains(&kit));

    let err = connection::open_db(dir.path(), &[0u8; 32]).unwrap_err();
    assert_eq!(err, "Wrong encryption key or corrupted database");
    let err = key_wrap::unwrap_dek(&[0u8; 32], "!!notbase64", key_wrap::PURPOSE_PASSWORD).unwrap_err();
    assert_eq!(err, "Unable to unwrap vault key");
}
