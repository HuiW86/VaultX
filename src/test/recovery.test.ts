import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "../stores/appStore";
import { useVaultStore } from "../stores/vaultStore";
import { useSearchStore } from "../stores/searchStore";
import { useSettingsStore } from "../stores/settingsStore";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  vi.clearAllMocks();
  useAppStore.setState({ status: "locked", error: null });
});

// Contract C9: after a successful recovery, decrypted entry and search caches are cleared.
describe("appStore.recover", () => {
  it("clears entry, search and settings caches after a successful recovery", async () => {
    useVaultStore.setState({
      entries: [{ id: "1", vault_id: "v1", category: "login", title: "Stale", subtitle: null, icon_url: null, favorite: false, trashed: false, updated_at: "" }],
      selectedEntryId: "1",
      selectedEntry: { entry: { id: "1" } as never, fields: [] },
    });
    useSearchStore.setState({ query: "sta", results: useVaultStore.getState().entries, isActive: true });
    useSettingsStore.setState({ settings: { ...useSettingsStore.getState().settings, touch_id_enabled: true }, loaded: true });
    mockInvoke.mockResolvedValueOnce({ touch_id_cleanup_failed: false });

    const result = await useAppStore.getState().recover("ABCD-EFGH", "new-password");

    expect(mockInvoke).toHaveBeenCalledWith("recover_with_key", { recoveryKey: "ABCD-EFGH", newPassword: "new-password" });
    expect(result.touch_id_cleanup_failed).toBe(false);
    expect(useAppStore.getState().status).toBe("unlocked");
    expect(useVaultStore.getState().entries).toEqual([]);
    expect(useVaultStore.getState().selectedEntry).toBeNull();
    expect(useSearchStore.getState().results).toEqual([]);
    expect(useSearchStore.getState().query).toBe("");
    expect(useSettingsStore.getState().loaded).toBe(false);
    expect(useSettingsStore.getState().settings.touch_id_enabled).toBe(false);
  });

  it("keeps caches and stays locked when recovery fails", async () => {
    useSearchStore.setState({ query: "x" });
    mockInvoke.mockRejectedValueOnce("Invalid recovery key");

    await expect(useAppStore.getState().recover("WRONG", "new-password")).rejects.toBe("Invalid recovery key");
    expect(useAppStore.getState().status).toBe("locked");
  });
});
