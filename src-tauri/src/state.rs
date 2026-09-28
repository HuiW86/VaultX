use rusqlite::Connection;
use std::path::PathBuf;
use std::time::Instant;
use zeroize::Zeroizing;

/// Application state held in memory, wrapped in Mutex by Tauri.
///
/// Security invariants:
/// - dek is zeroized on lock and app exit
/// - db connection is closed on lock
/// - No sensitive data persists in this struct after lock
pub struct AppState {
    /// Open database connection (None when locked)
    pub db: Option<Connection>,
    /// Vault data encryption key: SQLCipher key and field key (None when locked).
    /// In legacy mode this is the v1 password-derived key.
    pub dek: Option<Zeroizing<[u8; 32]>>,
    /// True while a v1 vault is open because its migration to v2 failed.
    pub legacy: bool,
    /// Path to the data directory
    pub data_dir: PathBuf,
    /// Last user activity timestamp (memory-only, for auto-lock)
    pub last_activity: Option<Instant>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            db: None,
            dek: None,
            legacy: false,
            data_dir,
            last_activity: None,
        }
    }

    /// Install an unlocked vault.
    pub fn set_unlocked(&mut self, unlocked: crate::vault::lifecycle::Unlocked) {
        self.db = Some(unlocked.conn);
        self.dek = Some(unlocked.key);
        self.legacy = unlocked.legacy;
        self.touch_activity();
    }

    /// Clear all sensitive state. Called on lock and exit.
    /// dek is automatically zeroized when dropped (via Zeroizing wrapper).
    pub fn clear(&mut self) {
        self.dek = None; // Zeroizing<T> zeros memory on drop
        self.db = None; // Closes connection
        self.legacy = false;
        self.last_activity = None;
    }

    pub fn is_unlocked(&self) -> bool {
        self.db.is_some() && self.dek.is_some()
    }

    /// Record user activity for auto-lock tracking
    pub fn touch_activity(&mut self) {
        self.last_activity = Some(Instant::now());
    }
}
