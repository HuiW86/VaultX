# Vault Key Lifecycle Contract

Status: **confirmed** (owner: project owner, 2026-09-28). This file is the authoritative source for how the vault master key, KDF salt, SQLCipher key, recovery blob and Keychain copy change together. Code must follow it; to change behavior described here, update this file first (or in the same commit) and keep the checks below passing.

Scope: `src-tauri/src/commands/{auth,recovery,security}.rs`, `src-tauri/src/db/connection.rs`. Any flow that changes the master key (recovery reset today; password change in future) is in scope.

| ID | Convention (target behavior) | Source | Coverage | Confidence | Check |
|---|---|---|---|---|---|
| C1 | If any step of a key-changing flow fails, **the old password or the new password must still unlock the vault**. A state where neither works is forbidden. | `recovery.rs` `recover_with_key` (rekey then write_meta) | recovery reset | high | Required: fault-injection test that forces meta write failure after rekey and asserts one password still unlocks |
| C2 | The SQLCipher key and the KDF salt in `.vaultx-meta` must always correspond to the same password. The order of rekey vs meta write, and the rollback on failure, must be explicit in code (e.g. roll the DB key back to the old key if meta write fails, or persist a pending state that unlock can resolve). | `recovery.rs`, `connection.rs:write_meta` | recovery reset, unlock | high | Covered by the C1 test plus a success-path test (new password unlocks, old does not) |
| C3 | `.vaultx-meta` is written atomically (temp file + rename). | `connection.rs:117-126` (already satisfied) | all meta writes | high | Existing behavior; do not regress |
| C4 | Error messages never contain keys, salts or passwords; key material stays in `Zeroizing`. | `CLAUDE.md` Security Rules | all flows | high | Review |
| C5 | After a successful key change, the old recovery blob is invalidated; the user must generate a new recovery kit. | `recovery.rs` (current behavior) | recovery reset | high | Success-path test asserts `recovery_blob` is `None` |
| C6 | After a successful key change, the master key copy stored in Keychain for Touch ID must be updated to the new key or removed (Touch ID disabled). A stale Keychain key is forbidden. | `security.rs` `setup_touch_id` / `unlock_biometric` | recovery reset, biometric unlock | high | Test or explicit code path; if Keychain cannot be exercised in tests, isolate the call behind a function and document it |

Arbitration: if code and this contract disagree, fix the code unless the owner changes the contract. Record exceptions here with scope and date.
