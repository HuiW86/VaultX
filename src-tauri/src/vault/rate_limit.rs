//! Persistent brute-force limiter shared by password unlock and recovery
//! (contract C12). One counter file per data directory, so alternating
//! between the two entry points does not reset or bypass the delay.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub const FAILED_ATTEMPTS_FILENAME: &str = ".vaultx-failed-attempts";

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct FailedAttempts {
    pub count: u32,
    pub last_failed_at: Option<u64>, // unix timestamp ms
}

pub fn read(base_dir: &Path) -> FailedAttempts {
    fs::read_to_string(base_dir.join(FAILED_ATTEMPTS_FILENAME))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write(base_dir: &Path, data: &FailedAttempts) {
    if let Ok(json) = serde_json::to_string(data) {
        let _ = fs::write(base_dir.join(FAILED_ATTEMPTS_FILENAME), json);
    }
}

/// Reset the counter after a successful unlock or recovery.
pub fn clear(base_dir: &Path) {
    let _ = fs::remove_file(base_dir.join(FAILED_ATTEMPTS_FILENAME));
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Delay in ms after `failed` consecutive failures.
/// 1-4: 0, 5: 5s, 6: 15s, 7: 30s, 8+: 60s
pub fn delay_ms(failed: u32) -> u64 {
    match failed {
        0..=4 => 0,
        5 => 5_000,
        6 => 15_000,
        7 => 30_000,
        _ => 60_000,
    }
}

/// `Err(remaining_ms)` while the caller must wait before another attempt.
pub fn check(base_dir: &Path, now: u64) -> Result<(), u64> {
    let attempts = read(base_dir);
    let delay = delay_ms(attempts.count);
    if delay > 0 {
        if let Some(last) = attempts.last_failed_at {
            let elapsed = now.saturating_sub(last);
            if elapsed < delay {
                return Err(delay - elapsed);
            }
        }
    }
    Ok(())
}

/// Record one failure; returns the delay now required before the next attempt.
pub fn record_failure(base_dir: &Path, now: u64) -> u64 {
    let count = read(base_dir).count.saturating_add(1);
    write(base_dir, &FailedAttempts { count, last_failed_at: Some(now) });
    delay_ms(count)
}

/// User-facing rate-limit message (fixed text, C4).
pub fn message(remaining_ms: u64) -> String {
    format!("Too many failed attempts. Wait {} seconds.", remaining_ms / 1000 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn delay_grows_after_four_failures_and_clears() {
        let dir = TempDir::new().unwrap();
        let t0 = 1_000_000;
        for _ in 0..4 {
            assert_eq!(check(dir.path(), t0), Ok(()));
            assert_eq!(record_failure(dir.path(), t0), 0);
        }
        assert_eq!(record_failure(dir.path(), t0), 5_000);
        assert_eq!(check(dir.path(), t0 + 1_000), Err(4_000));
        assert_eq!(check(dir.path(), t0 + 5_000), Ok(()));
        clear(dir.path());
        assert_eq!(read(dir.path()), FailedAttempts::default());
    }
}
