import { create } from "zustand";
import { api } from "../lib/commands";

interface AppState {
  status: "loading" | "first_run" | "locked" | "unlocked" | "corrupted";
  corruptReason: string | null;
  error: string | null;

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
      set({ error: null });
      await api.unlock(password);
      set({ status: "unlocked" });
    } catch (e: any) {
      const msg = typeof e === "string" ? e : e?.message || "Unlock failed";
      set({ error: msg });
      throw e;
    }
  },

  lock: async () => {
    await api.lock();
    // Security: import and reset vault store to clear decrypted data
    const { useVaultStore } = await import("./vaultStore");
    useVaultStore.getState().reset();
    set({ status: "locked", error: null });
  },

  clearError: () => set({ error: null }),
}));
