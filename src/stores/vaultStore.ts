import { create } from "zustand";
import {
  api,
  type EntrySummary,
  type EntryDetailResponse,
} from "../lib/commands";

interface VaultState {
  entries: EntrySummary[];
  selectedEntryId: string | null;
  selectedEntry: EntryDetailResponse | null;
  categoryFilter: string | null;
  showTrash: boolean;
  categoryCounts: Record<string, number>;
  loading: boolean;
  error: string | null;

  fetchEntries: () => Promise<void>;
  fetchCategoryCounts: () => Promise<void>;
  selectEntry: (id: string | null) => Promise<void>;
  setCategoryFilter: (category: string | null) => void;
  setShowTrash: (show: boolean) => void;
  createEntry: (input: {
    vault_id: string;
    category: string;
    title: string;
    subtitle?: string;
    fields: { field_type: string; label: string; value: string; sort_order: number }[];
  }) => Promise<EntrySummary>;
  updateEntry: (input: {
    entry_id: string;
    title?: string;
    subtitle?: string;
    fields?: { field_type: string; label: string; value: string; sort_order: number }[];
  }) => Promise<void>;
  trashEntry: (id: string) => Promise<void>;
  reset: () => void;
}

const initialState = {
  entries: [],
  selectedEntryId: null,
  selectedEntry: null,
  categoryFilter: null,
  showTrash: false,
  categoryCounts: {},
  loading: false,
  error: null,
};

export const useVaultStore = create<VaultState>((set, get) => ({
  ...initialState,

  fetchEntries: async () => {
    try {
      set({ loading: true, error: null });
      const { categoryFilter, showTrash } = get();
      const entries = await api.listEntries({
        category: categoryFilter ?? undefined,
        trashed: showTrash,
      });
      set({ entries, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  fetchCategoryCounts: async () => {
    try {
      const counts = await api.getCategoryCounts();
      const map: Record<string, number> = {};
      for (const [cat, count] of counts) {
        map[cat] = count;
      }
      set({ categoryCounts: map });
    } catch {
      // Non-critical, ignore
    }
  },

  selectEntry: async (id: string | null) => {
    if (!id) {
      set({ selectedEntryId: null, selectedEntry: null });
      return;
    }
    try {
      set({ selectedEntryId: id, error: null });
      const entry = await api.getEntry(id);
      set({ selectedEntry: entry });
    } catch (e) {
      set({ error: String(e), selectedEntry: null });
    }
  },

  setCategoryFilter: (category: string | null) => {
    set({ categoryFilter: category, selectedEntryId: null, selectedEntry: null });
    get().fetchEntries();
  },

  setShowTrash: (show: boolean) => {
    set({ showTrash: show, categoryFilter: null, selectedEntryId: null, selectedEntry: null });
    get().fetchEntries();
  },

  createEntry: async (input) => {
    const result = await api.createEntry(input);
    await get().fetchEntries();
    await get().fetchCategoryCounts();
    return result;
  },

  updateEntry: async (input) => {
    await api.updateEntry(input);
    const { selectedEntryId } = get();
    if (selectedEntryId) {
      await get().selectEntry(selectedEntryId);
    }
    await get().fetchEntries();
  },

  trashEntry: async (id: string) => {
    await api.trashEntry(id);
    const { selectedEntryId } = get();
    if (selectedEntryId === id) {
      set({ selectedEntryId: null, selectedEntry: null });
    }
    await get().fetchEntries();
    await get().fetchCategoryCounts();
  },

  reset: () => set(initialState),
}));
