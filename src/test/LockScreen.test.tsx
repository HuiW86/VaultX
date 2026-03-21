import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { LockScreen } from "../components/lock/LockScreen";
import { useAppStore } from "../stores/appStore";
import { renderWithI18n } from "./test-utils";

const mockInvoke = vi.mocked(invoke);

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
});
