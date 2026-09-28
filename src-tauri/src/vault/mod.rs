//! Vault key lifecycle: DEK/KEK hierarchy, create/unlock/recovery/password
//! change, v1→v2 migration and Keychain invalidation.
//!
//! Authoritative contract: `docs/contracts/vault-key-lifecycle.md` (C1–C9).
//! Everything here is independent of Tauri so it can be tested against temp
//! directories with a fake Keychain and injected faults.

pub mod keystore;
pub mod lifecycle;
pub mod lock;
pub mod rate_limit;
pub mod migration;
pub mod recovery_key;

/// Fault-injection hook. Flows call it with a stable label at every commit
/// point; returning `Err` aborts the flow immediately without any cleanup,
/// which is how tests simulate a crash. Production passes `no_faults`.
pub type FaultHook<'a> = &'a dyn Fn(&'static str) -> Result<(), String>;

/// Hook that never fails.
pub fn no_faults(_: &'static str) -> Result<(), String> {
    Ok(())
}
