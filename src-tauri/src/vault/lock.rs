//! Cross-process exclusive lock over the vault files (contract C10).
//!
//! Every flow that creates, renames or deletes vault files (create, unlock
//! incl. migration and reconcile, Touch ID enable/unlock, recovery kit,
//! recovery, password change, Keychain invalidation) holds this lock for its
//! whole duration, so a second VaultX process cannot e.g. reconcile away a
//! shadow DB while the first one is migrating into it.
//!
//! Implementation: BSD `flock(2)` on `<data dir>/.vaultx-lock`, declared
//! directly against the C library (no extra crate). The kernel releases the
//! lock when the holding process exits or crashes, so there is no stale lock
//! to clean up. `flock` locks belong to the open file description, so two
//! `VaultLock`s in the same process also exclude each other (used by tests).
//! Locks must not be nested: flows take it once at their public entry point
//! and call `*_locked` internals.

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::time::{Duration, Instant};

pub const LOCK_FILENAME: &str = ".vaultx-lock";
/// How long a flow waits for another process before giving up.
pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(25);

pub const BUSY_MESSAGE: &str = "The vault is busy in another VaultX window. Try again.";

/// Held exclusive lock; released on drop (closing the descriptor).
#[derive(Debug)]
pub struct VaultLock {
    _file: File,
}

#[cfg(unix)]
mod sys {
    use std::os::raw::c_int;
    use std::os::unix::io::AsRawFd;

    pub const LOCK_EX: c_int = 2;
    pub const LOCK_NB: c_int = 4;

    extern "C" {
        fn flock(fd: c_int, operation: c_int) -> c_int;
    }

    pub enum TryLock {
        Acquired,
        WouldBlock,
        Failed(std::io::Error),
    }

    pub fn try_lock_exclusive(file: &std::fs::File) -> TryLock {
        // SAFETY: `flock` only reads the descriptor, which stays valid for
        // the lifetime of `file`.
        let rc = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };
        if rc == 0 {
            return TryLock::Acquired;
        }
        let err = std::io::Error::last_os_error();
        match err.kind() {
            std::io::ErrorKind::WouldBlock => TryLock::WouldBlock,
            std::io::ErrorKind::Interrupted => TryLock::WouldBlock,
            _ => TryLock::Failed(err),
        }
    }
}

impl VaultLock {
    /// Acquire the lock, waiting up to `DEFAULT_LOCK_TIMEOUT`.
    pub fn acquire(dir: &Path) -> Result<Self, String> {
        Self::acquire_timeout(dir, DEFAULT_LOCK_TIMEOUT)
    }

    /// Acquire the lock, waiting up to `timeout`; `Err(BUSY_MESSAGE)` on timeout.
    pub fn acquire_timeout(dir: &Path, timeout: Duration) -> Result<Self, String> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join(LOCK_FILENAME))
            .map_err(|e| format!("Failed to open vault lock: {e}"))?;
        Self::lock_file(file, timeout)
    }

    #[cfg(unix)]
    fn lock_file(file: File, timeout: Duration) -> Result<Self, String> {
        let deadline = Instant::now() + timeout;
        loop {
            match sys::try_lock_exclusive(&file) {
                sys::TryLock::Acquired => return Ok(VaultLock { _file: file }),
                sys::TryLock::Failed(e) => return Err(format!("Failed to lock vault: {e}")),
                sys::TryLock::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(BUSY_MESSAGE.to_string());
                    }
                    std::thread::sleep(POLL_INTERVAL);
                }
            }
        }
    }

    /// Non-unix targets are not shipped (the app is macOS-only); the lock is
    /// a no-op there. Recorded in the contract (C10).
    #[cfg(not(unix))]
    fn lock_file(file: File, _timeout: Duration) -> Result<Self, String> {
        let _ = Instant::now();
        Ok(VaultLock { _file: file })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn second_holder_times_out_until_first_releases() {
        let dir = TempDir::new().unwrap();
        let first = VaultLock::acquire_timeout(dir.path(), Duration::from_millis(50)).unwrap();
        let err = VaultLock::acquire_timeout(dir.path(), Duration::from_millis(100)).unwrap_err();
        assert_eq!(err, BUSY_MESSAGE);
        drop(first);
        VaultLock::acquire_timeout(dir.path(), Duration::from_millis(100)).unwrap();
    }

    #[test]
    fn waiter_gets_lock_after_release() {
        let dir = TempDir::new().unwrap();
        let first = VaultLock::acquire(dir.path()).unwrap();
        let path = dir.path().to_path_buf();
        let waiter = std::thread::spawn(move || VaultLock::acquire_timeout(&path, Duration::from_secs(5)).is_ok());
        std::thread::sleep(Duration::from_millis(150));
        drop(first);
        assert!(waiter.join().unwrap());
    }
}
