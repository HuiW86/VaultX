# Vault Key Lifecycle Contract

Version 2 (2026-09-28). Status: **confirmed** (owner: project owner). Supersedes v1 (same date), which assumed the password-derived key encrypted the DB and fields directly; that design lost field data on recovery (fields stayed encrypted with the old key).

This file is the authoritative source for how the vault data key, key-encryption keys, KDF salt, recovery kit and Keychain copy change together. Code must follow it; to change behavior described here, update this file first (or in the same commit) and keep the checks below passing.

Scope: `src-tauri/src/commands/{auth,recovery,security,entries}.rs`, `src-tauri/src/db/`, `src-tauri/src/crypto/`, frontend stores cleared after recovery. Any flow that creates, unlocks, migrates, recovers or changes the credentials of a vault is in scope.

## Key hierarchy

- **DEK** (data encryption key): 32 random bytes generated once per vault. It is the SQLCipher key **and** the AES-256-GCM field key. It never changes on password change or recovery. (Full key rotation is a separate, future operation.)
- **Password KEK**: Argon2id(password, salt). Only wraps the DEK.
- **Recovery KEK**: derived from the recovery key. Only wraps the DEK.
- Wrapping uses AES-256-GCM with a purpose label bound as associated data (e.g. `vaultx:dek:password`, `vaultx:dek:recovery`) so one wrap cannot be substituted for another.
- `.vaultx-meta` version 2 holds: KDF params + salt, `dek_wrapped_by_password`, optional `dek_wrapped_by_recovery` (replaces `recovery_blob`). Version 1 meta (no wrapped DEK) is a legacy vault and must be migrated (C8).

## Conventions

| ID | Convention (target behavior) | Coverage | Check |
|---|---|---|---|
| C1 | If any step of a credential-changing or migration flow fails or the process crashes at any point, **at least one previously valid credential (old password, new password, or recovery key) still opens the vault with all data readable** (C7). A state where no credential works is forbidden. | recovery, password change, v1→v2 migration | Fault-injection tests at every commit point, including simulated restart |
| C2 | The DB and all encrypted fields use the DEK. Password change and recovery only rewrap the DEK and atomically rewrite meta; they **never** rekey the DB or re-encrypt fields. | recovery, password change | Test: DB file bytes unchanged by recovery; fields readable after |
| C3 | `.vaultx-meta` (and any pending/marker file) is written atomically (temp file + rename). | all meta writes | Existing behavior; do not regress |
| C4 | Error messages never contain keys, salts, passwords or recovery key characters; key material (DEK, KEKs, raw recovery key, hex key strings) stays in `Zeroizing`. | all flows | Review + tests asserting error strings |
| C5 | After a successful recovery, the recovery kit used is invalidated (`dek_wrapped_by_recovery` removed); the user must generate a new kit. On failure the old kit keeps working. | recovery | Success-path and failure-path tests |
| C6 | Keychain (Touch ID) stores a copy of the DEK, never a password KEK. After a successful recovery, the Keychain item is deleted and `touch_id_enabled` set to false; if deletion fails the command reports it explicitly (not silent success). Keychain calls sit behind a function that tests can replace. | recovery, Touch ID setup/unlock | Test with a fake Keychain |
| C7 | After create, unlock, migration, recovery or password change, **all data is readable**: every sensitive field type (`password`, `hidden`, `card_number`), all `password_history` values, and trashed entries. "The DB opens" is not sufficient evidence. | all flows | Tests decrypt every field and history row |
| C8 | A v1 vault is migrated to v2 on first successful password unlock: export to a shadow DB keyed with a new random DEK (e.g. `sqlcipher_export`), re-encrypt all fields and history in the shadow, verify every value decrypts and the shadow reopens, then commit through explicit, restart-safe states; the original DB is kept until the commit completes. | v1 vaults | Migration tests incl. fault injection at each state |
| C9 | After a successful recovery the frontend clears decrypted entry and search caches. | frontend | Review |

## Implementation decisions (2026-09-28, recorded with the v2 implementation)

These refine how the conventions above are met; they do not relax any of them.

- **Code location.** Flows live in `src-tauri/src/vault/` (`lifecycle.rs`: create, unlock, Touch ID, recovery kit, recovery, password change, Keychain invalidation, `reconcile`; `migration.rs`: v1→v2; `keystore.rs`: `KeyStore` trait + in-memory fake; `recovery_key.rs`). Wrapping is `crypto/key_wrap.rs`. Tauri commands only call these. Every flow takes a fault hook called at each commit point; production passes `no_faults`.
- **Meta.** `.vaultx-meta` v2 also carries `db_path` (one of `vault.db`, `vault.v2.db`; anything else is rejected) and `keychain_invalidation_pending`. Meta writes are temp file + fsync + rename + directory fsync (C3).
- **Migration commit (C8).** The shadow DB is `vault.v2.db`. The single commit point is the atomic rename of a v2 meta whose `db_path` is `vault.v2.db`. Before it, the v1 meta and `vault.db` are untouched; after it, `vault.db` is deleted. `reconcile` (run at the start of every flow) deletes whichever known DB file the current meta does not reference, only if the referenced DB exists, plus a stale `.vaultx-meta.tmp`.
- **Migration failure (C1 over C8).** If migration fails during a password unlock, the unlock still succeeds on the v1 vault ("legacy mode") and migration is retried at the next password unlock. In legacy mode, generating a recovery kit and enabling Touch ID are refused so the legacy key is never persisted again. Touch ID unlock is refused for v1 vaults.
- **v1 recovery.** Recovering a v1 vault decrypts the legacy key from `recovery_blob` and runs the same migration with the new password; the resulting v2 meta has no recovery wrap (C5).
- **Keychain invalidation (C6).** Recovery and migration set `keychain_invalidation_pending` in the same atomic meta write that commits them. Afterwards (and on every later password unlock while the flag is set) the app sets `touch_id_enabled` to false, deletes the Keychain item, then clears the flag. Touch ID unlock is refused while the flag is set. If deletion fails after a recovery, `recover_with_key` returns `touch_id_cleanup_failed: true` and the UI shows an error toast.
- **Password change.** No IPC command or UI exists yet; `vault::lifecycle::change_password` is the only implementation and is covered by the C1/C2 tests. Any future command must call it.

## Reporting and arbitration

Any change in scope must report the status of **every** convention above: satisfied (with test or code location), or an exception recorded in this file with scope, reason and date. Skipping a convention silently is a violation. If code and this contract disagree, fix the code unless the owner changes the contract.
