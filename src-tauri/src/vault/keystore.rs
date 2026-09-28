//! Keychain abstraction (contract C6). The Keychain only ever holds a copy of
//! the DEK. Production uses the macOS Keychain (see `commands::security`);
//! tests use `MemoryKeyStore` and never touch the real Keychain.

use std::cell::{Cell, RefCell};
use zeroize::Zeroizing;

pub trait KeyStore {
    /// Store the DEK (replacing any existing item).
    fn store(&self, dek: &[u8; 32]) -> Result<(), String>;
    /// Read the stored key (may prompt for biometrics).
    fn read(&self) -> Result<Zeroizing<Vec<u8>>, String>;
    /// Delete the stored key. Deleting a missing item is success.
    fn delete(&self) -> Result<(), String>;
}

/// In-memory fake Keychain for tests.
#[derive(Default)]
pub struct MemoryKeyStore {
    item: RefCell<Option<Zeroizing<Vec<u8>>>>,
    pub fail_delete: Cell<bool>,
    pub fail_store: Cell<bool>,
}

impl MemoryKeyStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Put arbitrary bytes into the fake Keychain (e.g. a legacy v1 key).
    pub fn put_raw(&self, bytes: &[u8]) {
        *self.item.borrow_mut() = Some(Zeroizing::new(bytes.to_vec()));
    }

    pub fn has_item(&self) -> bool {
        self.item.borrow().is_some()
    }
}

impl KeyStore for MemoryKeyStore {
    fn store(&self, dek: &[u8; 32]) -> Result<(), String> {
        if self.fail_store.get() {
            return Err("Keychain store failed".into());
        }
        self.put_raw(dek);
        Ok(())
    }

    fn read(&self) -> Result<Zeroizing<Vec<u8>>, String> {
        self.item
            .borrow()
            .as_ref()
            .map(|v| Zeroizing::new(v.to_vec()))
            .ok_or_else(|| "Touch ID authentication failed or key not found".to_string())
    }

    fn delete(&self) -> Result<(), String> {
        if self.fail_delete.get() {
            return Err("Keychain delete failed".into());
        }
        *self.item.borrow_mut() = None;
        Ok(())
    }
}
