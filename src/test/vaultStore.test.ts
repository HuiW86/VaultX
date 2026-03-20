import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useVaultStore } from "../stores/vaultStore";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  vi.clearAllMocks();
  useVaultStore.getState().reset();
});

describe("vaultStore", () => {
  it("reset clears all data", () => {
    useVaultStore.setState({
      entries: [{ id: "1", vault_id: "v1", category: "login", title: "Test", subtitle: null, icon_url: null, favorite: false, trashed: false, updated_at: "" }],
      selectedEntryId: "1",
      selectedEntry: { entry: {} as any, fields: [] },
    });

    useVaultStore.getState().reset();

    const state = useVaultStore.getState();
    expect(state.entries).toEqual([]);
    expect(state.selectedEntryId).toBeNull();
    expect(state.selectedEntry).toBeNull();
    expect(state.categoryFilter).toBeNull();
    expect(state.showTrash).toBe(false);
  });

  it("fetchEntries calls list_entries and updates state", async () => {
    const mockEntries = [
      { id: "1", vault_id: "v1", category: "login", title: "Site A", subtitle: null, icon_url: null, favorite: false, trashed: false, updated_at: "" },
    ];
    mockInvoke.mockResolvedValueOnce(mockEntries);

    await useVaultStore.getState().fetchEntries();

    expect(mockInvoke).toHaveBeenCalledWith("list_entries", {
      category: undefined,
      trashed: false,
    });
    expect(useVaultStore.getState().entries).toEqual(mockEntries);
    expect(useVaultStore.getState().loading).toBe(false);
  });

  it("selectEntry calls get_entry and stores result", async () => {
    const mockDetail = {
      entry: { id: "1", vault_id: "v1", category: "login", title: "Site", subtitle: null, icon_url: null, favorite: false, trashed: false, created_at: "", updated_at: "" },
      fields: [{ id: "f1", entry_id: "1", field_type: "username", label: "Username", value: "user", sort_order: 0, sensitive: false }],
    };
    mockInvoke.mockResolvedValueOnce(mockDetail);

    await useVaultStore.getState().selectEntry("1");

    expect(mockInvoke).toHaveBeenCalledWith("get_entry", { entryId: "1" });
    expect(useVaultStore.getState().selectedEntry).toEqual(mockDetail);
    expect(useVaultStore.getState().selectedEntryId).toBe("1");
  });

  it("selectEntry(null) clears selection", async () => {
    useVaultStore.setState({ selectedEntryId: "1", selectedEntry: { entry: {} as any, fields: [] } });

    await useVaultStore.getState().selectEntry(null);

    expect(useVaultStore.getState().selectedEntryId).toBeNull();
    expect(useVaultStore.getState().selectedEntry).toBeNull();
  });

  it("trashEntry calls trash_entry and refreshes", async () => {
    mockInvoke
      .mockResolvedValueOnce(undefined) // trash_entry
      .mockResolvedValueOnce([]) // fetchEntries
      .mockResolvedValueOnce([]); // fetchCategoryCounts

    useVaultStore.setState({ selectedEntryId: "1", selectedEntry: { entry: {} as any, fields: [] } });

    await useVaultStore.getState().trashEntry("1");

    expect(mockInvoke).toHaveBeenCalledWith("trash_entry", { entryId: "1" });
    expect(useVaultStore.getState().selectedEntryId).toBeNull();
    expect(useVaultStore.getState().selectedEntry).toBeNull();
  });
});
