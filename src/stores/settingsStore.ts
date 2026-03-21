import { create } from "zustand";
import { api, type VaultxSettings } from "../lib/commands";

interface SettingsState {
  settings: VaultxSettings;
  loaded: boolean;
  load: () => Promise<void>;
  update: (patch: Partial<VaultxSettings>) => Promise<void>;
  reset: () => void;
}

const defaults: VaultxSettings = {
  auto_lock_timeout_minutes: 480,
  lock_on_sleep: true,
  clipboard_clear_seconds: 30,
  touch_id_enabled: false,
  theme: "dark",
  start_at_login: false,
  show_in_menu_bar: false,
  language: "en",
};

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: defaults,
  loaded: false,

  load: async () => {
    const settings = await api.getSettings();
    set({ settings, loaded: true });
  },

  update: async (patch) => {
    const merged = { ...get().settings, ...patch };
    await api.saveSettings(merged);
    set({ settings: merged });
  },

  reset: () => set({ settings: defaults, loaded: false }),
}));
