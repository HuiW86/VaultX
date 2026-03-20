import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import App from "../App";
import { useAppStore } from "../stores/appStore";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  vi.clearAllMocks();
  useAppStore.setState({ status: "loading", error: null, corruptReason: null });
});

describe("App", () => {
  it("shows setup wizard on first_run", async () => {
    mockInvoke.mockResolvedValueOnce("first_run");
    render(<App />);
    await waitFor(() => {
      expect(screen.getByText("Welcome to VaultX")).toBeInTheDocument();
    });
  });

  it("shows lock screen when locked", async () => {
    mockInvoke.mockResolvedValueOnce("locked");
    render(<App />);
    await waitFor(() => {
      expect(screen.getByPlaceholderText("Master password")).toBeInTheDocument();
    });
  });

  it("shows error state when corrupted", async () => {
    mockInvoke.mockResolvedValueOnce({ corrupted: { reason: "Meta file missing" } });
    render(<App />);
    await waitFor(() => {
      expect(screen.getByText("Vault corrupted")).toBeInTheDocument();
    });
  });
});
