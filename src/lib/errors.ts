/**
 * Error code the backend returns when the migrated vault DB failed
 * verification while the legacy DB still exists (contract G2/E4): the vault
 * was not opened and the user must restore manually. `unlock` reports it as
 * `UnlockError.kind`; Touch ID unlock and recovery return it as the error
 * string. Mirrors `lifecycle::MANUAL_RESTORE_REQUIRED` in Rust.
 */
export const MANUAL_RESTORE_REQUIRED = "manual_restore_required";

export function isManualRestoreError(e: unknown): boolean {
  if (e === MANUAL_RESTORE_REQUIRED) return true;
  return typeof e === "object" && e !== null && (e as { kind?: unknown }).kind === MANUAL_RESTORE_REQUIRED;
}
