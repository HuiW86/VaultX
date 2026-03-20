import { invoke } from "@tauri-apps/api/core";

// -- Types --

export type AppStatus =
  | "first_run"
  | "locked"
  | "unlocked"
  | { corrupted: { reason: string } };

export interface UnlockResult {
  success: boolean;
}

export interface UnlockError {
  kind: "wrong_password" | "db_corrupted" | "rate_limited";
  message: string;
  retry_after_ms?: number;
}

export interface EntrySummary {
  id: string;
  vault_id: string;
  category: string;
  title: string;
  subtitle: string | null;
  icon_url: string | null;
  favorite: boolean;
  trashed: boolean;
  updated_at: string;
}

export interface DecryptedField {
  id: string;
  entry_id: string;
  field_type: string;
  label: string;
  value: string;
  sort_order: number;
  sensitive: boolean;
}

export interface EntryDetailResponse {
  entry: {
    id: string;
    vault_id: string;
    category: string;
    title: string;
    subtitle: string | null;
    icon_url: string | null;
    favorite: boolean;
    trashed: boolean;
    created_at: string;
    updated_at: string;
  };
  fields: DecryptedField[];
}

export interface FieldInputDto {
  field_type: string;
  label: string;
  value: string;
  sort_order: number;
}

export interface Vault {
  id: string;
  name: string;
  icon: string | null;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

// -- API calls --

export const api = {
  getAppStatus: () => invoke<AppStatus>("get_app_status"),
  setupVault: (password: string) => invoke<void>("setup_vault", { password }),
  unlock: (password: string) => invoke<UnlockResult>("unlock", { password }),
  lock: () => invoke<void>("lock"),

  listVaults: () => invoke<Vault[]>("list_vaults"),
  createEntry: (input: {
    vault_id: string;
    category: string;
    title: string;
    subtitle?: string;
    fields: FieldInputDto[];
  }) => invoke<EntrySummary>("create_entry", { input }),

  getEntry: (entryId: string) =>
    invoke<EntryDetailResponse>("get_entry", { entryId }),

  listEntries: (params: {
    vaultId?: string;
    category?: string;
    trashed?: boolean;
  }) => invoke<EntrySummary[]>("list_entries", params),

  updateEntry: (input: {
    entry_id: string;
    title?: string;
    subtitle?: string;
    fields?: FieldInputDto[];
  }) => invoke<void>("update_entry", { input }),

  trashEntry: (entryId: string) => invoke<void>("trash_entry", { entryId }),

  getCategoryCounts: (vaultId?: string) =>
    invoke<[string, number][]>("get_category_counts", { vaultId }),

  generatePassword: () => invoke<string>("generate_password"),

  copyToClipboard: (value: string, clearAfterMs?: number) =>
    invoke<void>("copy_to_clipboard", { value, clearAfterMs }),

  // Search
  searchEntries: (query: string, limit?: number) =>
    invoke<EntrySummary[]>("search_entries", { query, limit }),

  recentEntries: (limit?: number) =>
    invoke<EntrySummary[]>("recent_entries", { limit }),
};
