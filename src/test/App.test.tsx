import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import App from "../App";
import { useAppStore } from "../stores/appStore";
import { useSettingsStore } from "../stores/settingsStore";
import { renderWithI18n } from "./test-utils";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  vi.clearAllMocks();
  useAppStore.setState({ status: "loading", error: null, corruptReason: null });
  useSettingsStore.setState({
    settings: {
      auto_lock_timeout_minutes: 480,
      lock_on_sleep: true,
      clipboard_clear_seconds: 30,
      touch_id_enabled: false,
      theme: "dark",
      start_at_login: false,
      show_in_menu_bar: false,
      language: "en",
    },
    loaded: true,
  });
});

describe("App", () => {
  it("shows setup wizard on first_run", async () => {
    mockInvoke.mockResolvedValueOnce("first_run");
    renderWithI18n(<App />);
    await waitFor(() => {
      expect(screen.getByText("Welcome to VaultX")).toBeInTheDocument();
    });
  });

  it("shows lock screen when locked", async () => {
    mockInvoke.mockResolvedValueOnce("locked");
    renderWithI18n(<App />);
    await waitFor(() => {
      expect(screen.getByPlaceholderText("Master password")).toBeInTheDocument();
    });
  });

  it("shows error state when corrupted", async () => {
    mockInvoke.mockResolvedValueOnce({ corrupted: { reason: "Meta file missing" } });
    renderWithI18n(<App />);
    await waitFor(() => {
      expect(screen.getByText("Vault corrupted")).toBeInTheDocument();
    });
  });
});
