import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

const defaultSettings = {
  auto_lock_timeout_minutes: 480,
  lock_on_sleep: true,
  clipboard_clear_seconds: 30,
  touch_id_enabled: false,
  theme: "dark",
  start_at_login: false,
  show_in_menu_bar: false,
  language: "en",
};

// Mock Tauri API
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_settings") return Promise.resolve(defaultSettings);
    if (cmd === "is_touch_id_available") return Promise.resolve(false);
    return Promise.resolve(undefined);
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));
