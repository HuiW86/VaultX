import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SetupWizard } from "../components/lock/SetupWizard";
import { useAppStore } from "../stores/appStore";
import { renderWithI18n } from "./test-utils";

beforeEach(() => {
  vi.clearAllMocks();
  useAppStore.setState({ status: "first_run", error: null });
});

describe("SetupWizard", () => {
  it("renders password and confirm fields", () => {
    renderWithI18n(<SetupWizard />);
    expect(screen.getByLabelText("Master Password")).toBeInTheDocument();
    expect(screen.getByLabelText("Confirm Password")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /create vault/i })).toBeInTheDocument();
  });

  it("shows error when passwords don't match", async () => {
    const user = userEvent.setup();
    renderWithI18n(<SetupWizard />);

    await user.type(screen.getByLabelText("Master Password"), "password123!");
    await user.type(screen.getByLabelText("Confirm Password"), "different");
    await user.click(screen.getByRole("button", { name: /create vault/i }));

    await waitFor(() => {
      expect(screen.getByText("Passwords don't match")).toBeInTheDocument();
    });
  });

  it("shows error when password too short", async () => {
    const user = userEvent.setup();
    renderWithI18n(<SetupWizard />);

    await user.type(screen.getByLabelText("Master Password"), "short");
    await user.type(screen.getByLabelText("Confirm Password"), "short");
    await user.click(screen.getByRole("button", { name: /create vault/i }));

    await waitFor(() => {
      expect(screen.getByText(/at least 8 characters/i)).toBeInTheDocument();
    });
  });
});
