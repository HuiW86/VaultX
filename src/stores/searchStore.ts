import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { EntrySummary } from "../lib/commands";

interface SearchState {
  query: string;
  results: EntrySummary[];
  searching: boolean;
  isActive: boolean;

  setQuery: (q: string) => void;
  search: (q: string) => Promise<void>;
  clear: () => void;
  reset: () => void;
}

let debounceTimer: ReturnType<typeof setTimeout> | undefined;

export const useSearchStore = create<SearchState>((set, get) => ({
  query: "",
  results: [],
  searching: false,
  isActive: false,

  setQuery: (q: string) => {
    set({ query: q, isActive: q.length > 0 });
    if (debounceTimer) clearTimeout(debounceTimer);
    if (!q.trim()) {
      set({ results: [], searching: false, isActive: false });
      return;
    }
    set({ searching: true });
    debounceTimer = setTimeout(() => {
      get().search(q);
    }, 150);
  },

  search: async (q: string) => {
    try {
      const results = await invoke<EntrySummary[]>("search_entries", {
        query: q,
        limit: 20,
      });
      // Only update if query hasn't changed during the request
      if (get().query === q) {
        set({ results, searching: false });
      }
    } catch {
      set({ searching: false });
    }
  },

  clear: () => {
    if (debounceTimer) clearTimeout(debounceTimer);
    set({ query: "", results: [], searching: false, isActive: false });
  },

  reset: () => {
    if (debounceTimer) clearTimeout(debounceTimer);
    set({ query: "", results: [], searching: false, isActive: false });
  },
}));
