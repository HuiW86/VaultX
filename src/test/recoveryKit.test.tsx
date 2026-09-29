import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { RecoveryKitBanner } from "../components/layout/RecoveryKitBanner";
import { SettingsPanel } from "../components/settings/SettingsPanel";
import { RecoveryKitDialog } from "../components/settings/RecoveryKitDialog";
import { useAppStore } from "../stores/appStore";
import { useSettingsStore } from "../stores/settingsStore";
import { I18nProvider } from "../i18n";
import en from "../i18n/en";
import zhCN from "../i18n/zh-CN";
import { renderWithI18n } from "./test-utils";

// Contract C11: missing recovery kit notice and regenerate entry.

const mockInvoke = vi.mocked(invoke);
const KIT = { recovery_key: "ABCD-EFGH-IJKL-MNOP-QRST-UVWX-YZ23", file_content: "kit file" };

let kitPresent = false;
let canGenerate = true;

function mockBackend() {
  mockInvoke.mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case "get_recovery_kit_status":
        return { present: kitPresent, can_generate: canGenerate };
      case "generate_recovery_kit":
        kitPresent = true;
        return KIT;
      case "get_settings":
        return useSettingsStore.getState().settings;
      case "is_touch_id_available":
        return false;
      default:
        return undefined;
    }
  });
}

const createObjectURL = vi.fn(() => "blob:kit");
const revokeObjectURL = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  kitPresent = false;
  canGenerate = true;
  useAppStore.setState({ status: "unlocked", error: null, recoveryKit: null });
  mockBackend();
  Object.assign(URL, { createObjectURL, revokeObjectURL });
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("RecoveryKitBanner", () => {
  it("shows a notice after unlock when the vault has no recovery kit", async () => {
    renderWithI18n(<RecoveryKitBanner />);
    expect(await screen.findByText(en["recovery_kit.banner"])).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith("get_recovery_kit_status");
  });

  it("stays hidden when a kit exists or cannot be generated yet", async () => {
    kitPresent = true;
    const { unmount } = renderWithI18n(<RecoveryKitBanner />);
    await waitFor(() => expect(useAppStore.getState().recoveryKit).not.toBeNull());
    expect(screen.queryByText(en["recovery_kit.banner"])).not.toBeInTheDocument();
    unmount();

    kitPresent = false;
    canGenerate = false; // legacy mode: generation is refused by the backend
    useAppStore.setState({ recoveryKit: null });
    renderWithI18n(<RecoveryKitBanner />);
    await waitFor(() => expect(useAppStore.getState().recoveryKit?.can_generate).toBe(false));
    expect(screen.queryByText(en["recovery_kit.banner"])).not.toBeInTheDocument();
  });

  it("regenerates the kit from the notice and offers the download", async () => {
    const user = userEvent.setup();
    renderWithI18n(<RecoveryKitBanner />);
    await user.click(await screen.findByRole("button", { name: en["recovery_kit.banner_action"] }));
    await user.click(await screen.findByRole("button", { name: en["recovery_kit.confirm"] }));

    expect(mockInvoke).toHaveBeenCalledWith("generate_recovery_kit");
    expect(await screen.findByTestId("recovery-key")).toHaveTextContent(KIT.recovery_key);
    const done = screen.getByRole("button", { name: en["recovery_kit.done"] });
    expect(done).toBeDisabled();

    await user.click(screen.getByRole("button", { name: en["setup.recovery_download"] }));
    expect(createObjectURL).toHaveBeenCalledTimes(1);
    expect(done).toBeEnabled();

    // Status was refreshed: the notice is gone once the dialog closes.
    expect(useAppStore.getState().recoveryKit).toEqual({ present: true, can_generate: true });
    await user.click(done);
    await waitFor(() => expect(screen.queryByText(en["recovery_kit.banner"])).not.toBeInTheDocument());
    expect(screen.queryByText(KIT.recovery_key)).not.toBeInTheDocument();
  });

  it("is translated in zh-CN", async () => {
    render(
      <I18nProvider locale="zh-CN">
        <RecoveryKitBanner />
      </I18nProvider>
    );
    expect(await screen.findByText(zhCN["recovery_kit.banner"])).toBeInTheDocument();
    expect(screen.getByRole("button", { name: zhCN["recovery_kit.banner_action"] })).toBeInTheDocument();
  });
});

describe("SettingsPanel recovery kit entry", () => {
  beforeEach(() => {
    useSettingsStore.setState({ loaded: true });
  });

  it("marks a missing kit and regenerates it on confirm", async () => {
    const user = userEvent.setup();
    renderWithI18n(<SettingsPanel onClose={() => {}} />);
    expect(await screen.findByText(en["recovery_kit.missing_status"])).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: en["recovery_kit.regenerate"] }));
    expect(screen.getByText(en["recovery_kit.confirm_desc"])).toBeInTheDocument();
    expect(mockInvoke).not.toHaveBeenCalledWith("generate_recovery_kit");
    await user.click(screen.getByRole("button", { name: en["recovery_kit.confirm"] }));

    expect(await screen.findByTestId("recovery-key")).toHaveTextContent(KIT.recovery_key);
    await waitFor(() => expect(screen.queryByText(en["recovery_kit.missing_status"])).not.toBeInTheDocument());
  });

  it("offers regeneration even when a kit exists, and shows an error on failure", async () => {
    kitPresent = true;
    const user = userEvent.setup();
    renderWithI18n(<SettingsPanel onClose={() => {}} />);
    const button = await screen.findByRole("button", { name: en["recovery_kit.regenerate"] });
    await waitFor(() => expect(useAppStore.getState().recoveryKit?.present).toBe(true));
    expect(screen.queryByText(en["recovery_kit.missing_status"])).not.toBeInTheDocument();

    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "generate_recovery_kit") throw "Vault upgrade pending";
      if (cmd === "get_recovery_kit_status") return { present: true, can_generate: true };
      return undefined;
    });
    await user.click(button);
    await user.click(screen.getByRole("button", { name: en["recovery_kit.confirm"] }));
    expect(await screen.findByRole("alert")).toHaveTextContent(en["recovery_kit.failed"]);
  });

  it("has every recovery kit text in both languages", () => {
    const keys = Object.keys(en).filter((k) => k.startsWith("recovery_kit."));
    expect(keys.length).toBeGreaterThan(5);
    for (const k of keys) {
      const zh = (zhCN as Record<string, string>)[k];
      expect(zh, k).toBeTruthy();
      expect(zh, k).not.toBe((en as Record<string, string>)[k]);
    }
  });
});

describe("RecoveryKitDialog close guard (G1)", () => {
  type ClosePath = "x" | "overlay" | "escape";

  async function attemptClose(user: ReturnType<typeof userEvent.setup>, path: ClosePath) {
    if (path === "x") {
      await user.click(screen.getByRole("button", { name: en["modal.close"] }));
    } else if (path === "overlay") {
      const overlay = screen.getByRole("dialog").previousElementSibling as HTMLElement;
      await user.click(overlay);
    } else {
      await user.keyboard("{Escape}");
    }
  }

  async function openWithNewKit() {
    const onClose = vi.fn();
    const user = userEvent.setup();
    renderWithI18n(<RecoveryKitDialog open onClose={onClose} />);
    await user.click(screen.getByRole("button", { name: en["recovery_kit.confirm"] }));
    expect(await screen.findByTestId("recovery-key")).toHaveTextContent(KIT.recovery_key);
    return { onClose, user };
  }

  for (const path of ["x", "overlay", "escape"] as const) {
    it(`does not close silently via ${path} before the kit is downloaded`, async () => {
      const { onClose, user } = await openWithNewKit();

      await attemptClose(user, path);
      expect(onClose).not.toHaveBeenCalled();
      expect(screen.getByRole("alert")).toHaveTextContent(en["recovery_kit.close_confirm_desc"]);

      // The same close path on the confirmation only goes back to the key.
      await attemptClose(user, path);
      expect(onClose).not.toHaveBeenCalled();
      expect(screen.getByTestId("recovery-key")).toHaveTextContent(KIT.recovery_key);

      // Closing needs the explicit "close anyway" choice.
      await attemptClose(user, path);
      await user.click(screen.getByRole("button", { name: en["recovery_kit.close_anyway"] }));
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it(`closes via ${path} once the kit is downloaded`, async () => {
      const { onClose, user } = await openWithNewKit();
      await user.click(screen.getByRole("button", { name: en["setup.recovery_download"] }));
      await attemptClose(user, path);
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  }

  it("offers a way back to the download from the confirmation", async () => {
    const { onClose, user } = await openWithNewKit();
    await attemptClose(user, "x");
    await user.click(screen.getByRole("button", { name: en["recovery_kit.close_back"] }));
    await user.click(screen.getByRole("button", { name: en["setup.recovery_download"] }));
    expect(createObjectURL).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: en["recovery_kit.done"] }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes freely before a kit is generated (the old kit still works)", async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    renderWithI18n(<RecoveryKitDialog open onClose={onClose} />);
    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(mockInvoke).not.toHaveBeenCalledWith("generate_recovery_kit");
  });
});

describe("RecoveryKitDialog while generating (H3)", () => {
  type Path = "x" | "overlay" | "escape" | "cancel";

  function deferGenerate() {
    let resolve!: (kit: typeof KIT) => void;
    const pending = new Promise<typeof KIT>((r) => (resolve = r));
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case "get_recovery_kit_status":
          return { present: kitPresent, can_generate: true };
        case "generate_recovery_kit": {
          const kit = await pending;
          kitPresent = true; // the backend committed; the old kit is gone
          return kit;
        }
        case "get_settings":
          return useSettingsStore.getState().settings;
        case "is_touch_id_available":
          return false;
        default:
          return undefined;
      }
    });
    return () => resolve(KIT);
  }

  async function attempt(user: ReturnType<typeof userEvent.setup>, path: Path) {
    if (path === "x") {
      await user.click(screen.getByRole("button", { name: en["modal.close"] }));
    } else if (path === "overlay") {
      await user.click(screen.getByRole("dialog").previousElementSibling as HTMLElement);
    } else if (path === "cancel") {
      await user.click(screen.getByRole("button", { name: en["modal.cancel"] }));
    } else {
      await user.keyboard("{Escape}");
    }
  }

  const entries = {
    banner: async (user: ReturnType<typeof userEvent.setup>) => {
      renderWithI18n(<RecoveryKitBanner />);
      await user.click(await screen.findByRole("button", { name: en["recovery_kit.banner_action"] }));
    },
    settings: async (user: ReturnType<typeof userEvent.setup>) => {
      useSettingsStore.setState({ loaded: true });
      renderWithI18n(<SettingsPanel onClose={() => {}} />);
      await user.click(await screen.findByRole("button", { name: en["recovery_kit.regenerate"] }));
    },
  };

  for (const [entry, open] of Object.entries(entries)) {
    for (const path of ["x", "overlay", "escape", "cancel"] as const) {
      it(`${entry}: ${path} cannot close the dialog before the new key is shown`, async () => {
        const finish = deferGenerate();
        const user = userEvent.setup();
        await open(user);
        await user.click(screen.getByRole("button", { name: en["recovery_kit.confirm"] }));
        await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("generate_recovery_kit"));

        await attempt(user, path);
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText(en["recovery_kit.confirm_desc"])).toBeInTheDocument();

        finish();
        expect(await screen.findByTestId("recovery-key")).toHaveTextContent(KIT.recovery_key);
      });
    }
  }

  it("disables the cancel and close buttons while generating", async () => {
    const finish = deferGenerate();
    const onClose = vi.fn();
    const user = userEvent.setup();
    renderWithI18n(<RecoveryKitDialog open onClose={onClose} />);
    await user.click(screen.getByRole("button", { name: en["recovery_kit.confirm"] }));
    expect(screen.getByRole("button", { name: en["modal.cancel"] })).toBeDisabled();
    expect(screen.getByRole("button", { name: en["modal.close"] })).toBeDisabled();
    for (const path of ["x", "overlay", "escape", "cancel"] as const) await attempt(user, path);
    expect(onClose).not.toHaveBeenCalled();
    finish();
    expect(await screen.findByTestId("recovery-key")).toHaveTextContent(KIT.recovery_key);
  });
});

describe("appStore recovery kit status", () => {
  it("forgets the old status after a recovery (the used kit is invalid)", async () => {
    useAppStore.setState({ status: "locked", recoveryKit: { present: true, can_generate: true } });
    mockInvoke.mockResolvedValueOnce({ touch_id_cleanup_failed: false });
    await useAppStore.getState().recover("ABCD", "new-password");
    expect(useAppStore.getState().recoveryKit).toBeNull();
  });
});
