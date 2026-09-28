import { create } from "zustand";
import { api, type RecoverResult, type RecoveryKitStatus, type UnlockError } from "../lib/commands";
import { MANUAL_RESTORE_REQUIRED, isManualRestoreError } from "../lib/errors";

/**
 * Reset every store that caches decrypted entries, search results or
 * settings. Used on lock and after a successful recovery (contract C9).
 */
export async function clearSessionCaches(): Promise<void> {
  const { useVaultStore } = await import("./vaultStore");
  const { useSearchStore } = await import("./searchStore");
  const { useSettingsStore } = await import("./settingsStore");
  useVaultStore.getState().reset();
  useSearchStore.getState().reset();
  useSettingsStore.getState().reset();
}

interface AppState {
  status: "loading" | "first_run" | "locked" | "unlocked" | "corrupted";
  corruptReason: string | null;
  error: string | null;
  retryAfterMs: number | null;
  /** Last known recovery kit status of the unlocked vault (C11); null = unknown. */
  recoveryKit: RecoveryKitStatus | null;

  init: () => Promise<void>;
  setup: (password: string) => Promise<void>;
  unlock: (password: string) => Promise<void>;
  lock: () => Promise<void>;
  recover: (recoveryKey: string, newPassword: string) => Promise<RecoverResult>;
  clearError: () => void;
  refreshRecoveryKitStatus: () => Promise<void>;
}

export const useAppStore = create<AppState>((set) => ({
  status: "loading",
  corruptReason: null,
  error: null,
  retryAfterMs: null,
  recoveryKit: null,

  init: async () => {
    try {
      const status = await api.getAppStatus();
      if (typeof status === "string") {
        set({ status: status as "first_run" | "locked" | "unlocked", error: null });
      } else if ("corrupted" in status) {
        set({ status: "corrupted", corruptReason: status.corrupted.reason });
      }
    } catch (e) {
      set({ status: "corrupted", corruptReason: String(e) });
    }
  },

  setup: async (password: string) => {
    try {
      set({ error: null });
      await api.setupVault(password);
      set({ status: "unlocked" });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  unlock: async (password: string) => {
    try {
      set({ error: null, retryAfterMs: null });
      await api.unlock(password);
      set({ status: "unlocked" });
    } catch (e: any) {
      // Tauri IPC errors from Result<_, UnlockError> come as the serialized UnlockError
      const unlockErr = e as Partial<UnlockError>;
      // The lock screen renders the manual-restore code as a translated notice.
      const msg = isManualRestoreError(e)
        ? MANUAL_RESTORE_REQUIRED
        : unlockErr?.message || (typeof e === "string" ? e : "Unlock failed");
      set({
        error: msg,
        retryAfterMs: unlockErr?.retry_after_ms ?? null,
      });
      throw e;
    }
  },

  lock: async () => {
    await api.lock();
    // Security: reset all stores to clear decrypted data
    await clearSessionCaches();
    set({ status: "locked", error: null, retryAfterMs: null, recoveryKit: null });
  },

  recover: async (recoveryKey: string, newPassword: string) => {
    const result = await api.recoverWithKey(recoveryKey, newPassword);
    // Drop anything cached before recovery; Touch ID was disabled by the backend.
    await clearSessionCaches();
    // The used kit is now invalid (C5); force a fresh status for the notice (C11).
    set({ status: "unlocked", error: null, retryAfterMs: null, recoveryKit: null });
    return result;
  },

  clearError: () => set({ error: null }),

  refreshRecoveryKitStatus: async () => {
    try {
      const recoveryKit = await api.getRecoveryKitStatus();
      set({ recoveryKit: recoveryKit ?? null });
    } catch {
      set({ recoveryKit: null });
    }
  },
}));
