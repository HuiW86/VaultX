import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import App from "../App";
import { useAppStore } from "../stores/appStore";
import { useSettingsStore } from "../stores/settingsStore";
import { renderWithI18n } from "./test-utils";
import en from "../i18n/en";

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

// Contract C11: a new vault shows its first recovery kit, and the main
// window stays hidden until the kit has been downloaded.
describe("App new vault flow", () => {
  const KIT = { recovery_key: "ABCD-EFGH-IJKL-MNOP-QRST-UVWX-YZ23", file_content: "kit file" };
  const FLAG = "vaultx.firstRecoveryKitPending";
  const createObjectURL = vi.fn(() => "blob:kit");
  let kitPresent = false;
  let failGenerate = 0;

  function mockBackend(appStatus: string) {
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case "get_app_status":
          return appStatus;
        case "get_settings":
          return useSettingsStore.getState().settings;
        case "setup_vault":
        case "heartbeat":
          return undefined;
        case "unlock":
          return { success: true };
        case "generate_recovery_kit":
          if (failGenerate > 0) {
            failGenerate -= 1;
            throw "Vault is busy";
          }
          kitPresent = true;
          return KIT;
        case "get_recovery_kit_status":
          return { present: kitPresent, can_generate: true };
        case "is_touch_id_available":
          return false;
        case "list_vaults":
        case "list_entries":
        case "recent_entries":
        case "search_entries":
        case "get_category_counts":
          return [];
        default:
          return undefined;
      }
    });
  }

  const mainWindow = () => screen.queryByRole("navigation", { name: "Sidebar" });
  const generateCalls = () => mockInvoke.mock.calls.filter(([c]) => c === "generate_recovery_kit").length;

  beforeEach(() => {
    localStorage.clear();
    kitPresent = false;
    failGenerate = 0;
    useAppStore.setState({ status: "loading", error: null, recoveryKit: null });
    Object.assign(URL, { createObjectURL, revokeObjectURL: vi.fn() });
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
  });

  async function createVault(user: ReturnType<typeof userEvent.setup>) {
    await user.type(await screen.findByLabelText(en["setup.master_password"]), "correct horse battery");
    await user.type(screen.getByLabelText(en["setup.confirm_password"]), "correct horse battery");
    await user.click(screen.getByRole("button", { name: en["setup.create_vault"] }));
  }

  it("shows the first recovery kit and enters the main window only after the download", async () => {
    mockBackend("first_run");
    const user = userEvent.setup();
    renderWithI18n(<App />);
    await createVault(user);

    expect(await screen.findByTestId("setup-recovery-key")).toHaveTextContent(KIT.recovery_key);
    expect(mockInvoke).toHaveBeenCalledWith("setup_vault", { password: "correct horse battery" });
    expect(generateCalls()).toBe(1);
    expect(useAppStore.getState().status).toBe("setup_recovery");
    expect(localStorage.getItem(FLAG)).toBe("1");
    expect(mainWindow()).not.toBeInTheDocument();

    const saved = screen.getByRole("button", { name: en["setup.recovery_saved"] });
    expect(saved).toBeDisabled();
    await user.click(saved);
    expect(mainWindow()).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: en["setup.recovery_download"] }));
    expect(createObjectURL).toHaveBeenCalledTimes(1);
    expect(localStorage.getItem(FLAG)).toBeNull();
    expect(saved).toBeEnabled();
    await user.click(saved);
    await user.click(await screen.findByRole("button", { name: en["setup.get_started"] }));

    await waitFor(() => expect(mainWindow()).toBeInTheDocument());
    expect(useAppStore.getState().status).toBe("unlocked");
    expect(generateCalls()).toBe(1);
    // The kit exists, so no missing-kit notice.
    await waitFor(() => expect(useAppStore.getState().recoveryKit?.present).toBe(true));
    expect(screen.queryByText(en["recovery_kit.banner"])).not.toBeInTheDocument();
  });

  it("offers a retry when creating the first kit fails, without showing the main window", async () => {
    mockBackend("first_run");
    failGenerate = 1;
    const user = userEvent.setup();
    renderWithI18n(<App />);
    await createVault(user);

    expect(await screen.findByRole("alert")).toHaveTextContent(en["setup.recovery_generate_failed"]);
    expect(mainWindow()).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: en["setup.recovery_generate"] }));
    expect(await screen.findByTestId("setup-recovery-key")).toHaveTextContent(KIT.recovery_key);
    expect(mainWindow()).not.toBeInTheDocument();
  });

  it("returns to the kit step after a restart when the first kit was never saved", async () => {
    localStorage.setItem(FLAG, "1");
    kitPresent = true; // generated before the restart, but never shown
    mockBackend("locked");
    const user = userEvent.setup();
    renderWithI18n(<App />);
    await user.type(await screen.findByPlaceholderText("Master password"), "correct horse battery");
    await user.click(screen.getByRole("button", { name: /unlock/i }));

    const create = await screen.findByRole("button", { name: en["setup.recovery_generate"] });
    expect(useAppStore.getState().status).toBe("setup_recovery");
    expect(mainWindow()).not.toBeInTheDocument();
    expect(generateCalls()).toBe(0);
    await user.click(create);
    expect(await screen.findByTestId("setup-recovery-key")).toHaveTextContent(KIT.recovery_key);
    await user.click(screen.getByRole("button", { name: en["setup.recovery_download"] }));
    await user.click(screen.getByRole("button", { name: en["setup.recovery_saved"] }));
    await user.click(await screen.findByRole("button", { name: en["setup.get_started"] }));
    await waitFor(() => expect(mainWindow()).toBeInTheDocument());
  });

  it("unlocks straight into the main window when no first kit is pending", async () => {
    mockBackend("locked");
    kitPresent = true;
    const user = userEvent.setup();
    renderWithI18n(<App />);
    await user.type(await screen.findByPlaceholderText("Master password"), "correct horse battery");
    await user.click(screen.getByRole("button", { name: /unlock/i }));
    await waitFor(() => expect(mainWindow()).toBeInTheDocument());
  });
});
