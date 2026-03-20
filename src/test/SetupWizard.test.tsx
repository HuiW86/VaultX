import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SetupWizard } from "../components/lock/SetupWizard";
import { useAppStore } from "../stores/appStore";

beforeEach(() => {
  vi.clearAllMocks();
  useAppStore.setState({ status: "first_run", error: null });
});

describe("SetupWizard", () => {
  it("renders password and confirm fields", () => {
    render(<SetupWizard />);
    expect(screen.getByLabelText("Master password")).toBeInTheDocument();
    expect(screen.getByLabelText("Confirm master password")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /create vault/i })).toBeInTheDocument();
  });

  it("shows error when passwords don't match", async () => {
    const user = userEvent.setup();
    render(<SetupWizard />);

    await user.type(screen.getByLabelText("Master password"), "password123!");
    await user.type(screen.getByLabelText("Confirm master password"), "different");
    await user.click(screen.getByRole("button", { name: /create vault/i }));

    await waitFor(() => {
      expect(screen.getByText("Passwords don't match")).toBeInTheDocument();
    });
  });

  it("shows error when password too short", async () => {
    const user = userEvent.setup();
    render(<SetupWizard />);

    await user.type(screen.getByLabelText("Master password"), "short");
    await user.type(screen.getByLabelText("Confirm master password"), "short");
    await user.click(screen.getByRole("button", { name: /create vault/i }));

    await waitFor(() => {
      expect(screen.getByText(/at least 8 characters/i)).toBeInTheDocument();
    });
  });
});
