import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { LockScreen } from "../components/lock/LockScreen";
import { useAppStore } from "../stores/appStore";
import { renderWithI18n } from "./test-utils";
import { render } from "@testing-library/react";
import { I18nProvider } from "../i18n";
import en from "../i18n/en";
import zhCN from "../i18n/zh-CN";

const mockInvoke = vi.mocked(invoke);
// One test below replaces the store's unlock; later tests need the real one.
const realUnlock = useAppStore.getState().unlock;

beforeEach(() => {
  vi.clearAllMocks();
  useAppStore.setState({ status: "locked", error: null });
});

describe("LockScreen", () => {
  it("renders password input and unlock button", () => {
    renderWithI18n(<LockScreen />);
    expect(screen.getByPlaceholderText("Master password")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /unlock/i })).toBeInTheDocument();
  });

  it("calls unlock on form submit", async () => {
    mockInvoke.mockResolvedValueOnce({ success: true });
    const user = userEvent.setup();

    renderWithI18n(<LockScreen />);
    await user.type(screen.getByPlaceholderText("Master password"), "my-password");
    await user.click(screen.getByRole("button", { name: /unlock/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("unlock", { password: "my-password" });
    });
  });

  it("shows error on wrong password", async () => {
    mockInvoke.mockRejectedValueOnce("Incorrect master password");
    useAppStore.setState({
      status: "locked",
      error: null,
      unlock: async (_password: string) => {
        useAppStore.setState({ error: "Incorrect master password" });
        throw new Error("Incorrect master password");
      },
    });

    const user = userEvent.setup();
    renderWithI18n(<LockScreen />);
    await user.type(screen.getByPlaceholderText("Master password"), "wrong");
    await user.click(screen.getByRole("button", { name: /unlock/i }));

    await waitFor(() => {
      expect(screen.getByText("Incorrect master password")).toBeInTheDocument();
    });
  });

  describe("manual restore required (contract G2/E4)", () => {
    const code = "manual_restore_required";
    beforeEach(() => useAppStore.setState({ unlock: realUnlock }));
    const settings = (touchId: boolean) => ({
      auto_lock_timeout_minutes: 480, lock_on_sleep: true, clipboard_clear_seconds: 30,
      touch_id_enabled: touchId, theme: "dark", start_at_login: false, show_in_menu_bar: false, language: "en",
    });

    it("explains the manual restore after a password unlock instead of opening the vault", async () => {
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === "get_settings") return settings(false);
        if (cmd === "unlock") throw { kind: code, message: "backend text", retry_after_ms: null };
        return undefined;
      });
      const user = userEvent.setup();
      renderWithI18n(<LockScreen />);
      await user.type(screen.getByPlaceholderText("Master password"), "correct-password");
      await user.click(screen.getByRole("button", { name: /unlock/i }));

      expect(await screen.findByText(en["lock.manual_restore_title"])).toBeInTheDocument();
      expect(screen.getByRole("alert")).toHaveTextContent(en["lock.manual_restore_desc"]);
      expect(screen.queryByText(code)).not.toBeInTheDocument();
      expect(useAppStore.getState().status).toBe("locked");
    });

    it("is translated in zh-CN", async () => {
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === "get_settings") return settings(false);
        if (cmd === "unlock") throw { kind: code, message: "backend text", retry_after_ms: null };
        return undefined;
      });
      const user = userEvent.setup();
      render(
        <I18nProvider locale="zh-CN">
          <LockScreen />
        </I18nProvider>
      );
      await user.type(screen.getByPlaceholderText(zhCN["lock.master_password_placeholder"]), "correct-password");
      await user.click(screen.getByRole("button", { name: zhCN["lock.unlock"] }));
      expect(await screen.findByText(zhCN["lock.manual_restore_title"])).toBeInTheDocument();
      expect(screen.getByRole("alert")).toHaveTextContent(zhCN["lock.manual_restore_desc"]);
      expect(zhCN["lock.manual_restore_desc"]).not.toBe(en["lock.manual_restore_desc"]);
    });

    it("explains the manual restore when Touch ID unlock reports it", async () => {
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === "get_settings") return settings(true);
        if (cmd === "unlock_biometric") throw code;
        return undefined;
      });
      renderWithI18n(<LockScreen />);
      expect(await screen.findByText(en["lock.manual_restore_title"])).toBeInTheDocument();
      expect(useAppStore.getState().status).toBe("locked");
    });

    it("explains the manual restore when recovery reports it", async () => {
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === "get_settings") return settings(false);
        if (cmd === "recover_with_key") throw code;
        return undefined;
      });
      const user = userEvent.setup();
      renderWithI18n(<LockScreen />);
      await user.click(screen.getByText(en["lock.forgot_password"]));
      await user.type(screen.getByPlaceholderText(en["lock.recovery_key_placeholder"]), "ABCD-EFGH");
      await user.type(screen.getByPlaceholderText(en["lock.new_password_placeholder"]), "new-password-1");
      await user.type(screen.getByPlaceholderText(en["lock.confirm_new_placeholder"]), "new-password-1");
      await user.click(screen.getByRole("button", { name: en["lock.reset_password"] }));
      expect(await screen.findByText(en["lock.manual_restore_title"])).toBeInTheDocument();
      expect(screen.queryByText(code)).not.toBeInTheDocument();
    });
  });
});
