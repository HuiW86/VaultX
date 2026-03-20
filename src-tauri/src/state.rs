use rusqlite::Connection;
use std::path::PathBuf;
use std::time::Instant;
use zeroize::Zeroizing;

/// Application state held in memory, wrapped in Mutex by Tauri.
///
/// Security invariants:
/// - master_key is zeroized on lock and app exit
/// - db connection is closed on lock
/// - No sensitive data persists in this struct after lock
pub struct AppState {
    /// Open database connection (None when locked)
    pub db: Option<Connection>,
    /// Derived master key for field-level encryption (None when locked)
    pub master_key: Option<Zeroizing<[u8; 32]>>,
    /// Path to the data directory
    pub data_dir: PathBuf,
    /// Last user activity timestamp (memory-only, for auto-lock)
    pub last_activity: Option<Instant>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            db: None,
            master_key: None,
            data_dir,
            last_activity: None,
        }
    }

    /// Clear all sensitive state. Called on lock and exit.
    /// master_key is automatically zeroized when dropped (via Zeroizing wrapper).
    pub fn clear(&mut self) {
        self.master_key = None; // Zeroizing<T> zeros memory on drop
        self.db = None; // Closes connection
        self.last_activity = None;
    }

    pub fn is_unlocked(&self) -> bool {
        self.db.is_some() && self.master_key.is_some()
    }

    /// Record user activity for auto-lock tracking
    pub fn touch_activity(&mut self) {
        self.last_activity = Some(Instant::now());
    }
}
