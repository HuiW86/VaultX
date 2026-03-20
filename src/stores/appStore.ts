import { create } from "zustand";
import { api, type UnlockError } from "../lib/commands";

interface AppState {
  status: "loading" | "first_run" | "locked" | "unlocked" | "corrupted";
  corruptReason: string | null;
  error: string | null;
  retryAfterMs: number | null;

  init: () => Promise<void>;
  setup: (password: string) => Promise<void>;
  unlock: (password: string) => Promise<void>;
  lock: () => Promise<void>;
  clearError: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  status: "loading",
  corruptReason: null,
  error: null,
  retryAfterMs: null,

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
      const msg = unlockErr?.message || (typeof e === "string" ? e : "Unlock failed");
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
    const { useVaultStore } = await import("./vaultStore");
    const { useSettingsStore } = await import("./settingsStore");
    useVaultStore.getState().reset();
    useSettingsStore.getState().reset();
    set({ status: "locked", error: null, retryAfterMs: null });
  },

  clearError: () => set({ error: null }),
}));
