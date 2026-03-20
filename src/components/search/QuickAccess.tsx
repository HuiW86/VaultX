import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Search, Lock } from "lucide-react";
import { motion } from "framer-motion";
import type { EntrySummary } from "../../lib/commands";
import { EntryCard } from "../entry/EntryCard";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";

type QAStatus = "loading" | "locked" | "unlocked";

export function QuickAccess() {
  const [status, setStatus] = useState<QAStatus>("loading");
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<EntrySummary[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [password, setPassword] = useState("");
  const [unlockError, setUnlockError] = useState("");
  const [unlockLoading, setUnlockLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  // Check vault status on mount and when window becomes visible
  const checkStatus = useCallback(async () => {
    try {
      const s = await invoke<string>("get_app_status");
      if (s === "unlocked") {
        setStatus("unlocked");
        // Load recent entries
        const recent = await invoke<EntrySummary[]>("recent_entries", { limit: 5 });
        setResults(recent);
      } else {
        setStatus("locked");
      }
    } catch {
      setStatus("locked");
    }
  }, []);

  useEffect(() => {
    checkStatus();
    const unlisten = listen("quickaccess:show", () => {
      checkStatus();
      setQuery("");
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    });
    return () => { unlisten.then((f) => f()); };
  }, [checkStatus]);

  // Search with debounce
  useEffect(() => {
    if (!query.trim()) {
      // Show recent when no query
      if (status === "unlocked") {
        invoke<EntrySummary[]>("recent_entries", { limit: 5 })
          .then(setResults)
          .catch(() => {});
      }
      return;
    }
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(async () => {
      try {
        const r = await invoke<EntrySummary[]>("search_entries", { query, limit: 8 });
        setResults(r);
        setSelectedIndex(0);
      } catch {}
    }, 150);
  }, [query, status]);

  const hideWindow = useCallback(() => {
    getCurrentWebviewWindow().hide();
    setQuery("");
    setSelectedIndex(0);
  }, []);

  // Copy password of selected entry and close
  const copyPassword = useCallback(async (entryId: string) => {
    try {
      const entry = await invoke<{ entry: any; fields: any[] }>("get_entry", { entryId });
      const pwField = entry.fields.find((f: any) => f.field_type === "password");
      if (pwField) {
        await invoke("copy_to_clipboard", { value: pwField.value });
      }
    } catch {}
    hideWindow();
  }, [hideWindow]);

  // Copy username of selected entry
  const copyUsername = useCallback(async (entryId: string) => {
    try {
      const entry = await invoke<{ entry: any; fields: any[] }>("get_entry", { entryId });
      const userField = entry.fields.find((f: any) => f.field_type === "username");
      if (userField) {
        await invoke("copy_to_clipboard", { value: userField.value });
      }
    } catch {}
    hideWindow();
  }, [hideWindow]);

  // Keyboard navigation
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      hideWindow();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && results.length > 0) {
      e.preventDefault();
      const entry = results[selectedIndex];
      if (e.shiftKey) {
        copyUsername(entry.id);
      } else if (e.metaKey) {
        // Cmd+Enter: open URL (TODO: open in browser)
      } else {
        copyPassword(entry.id);
      }
    }
  }, [results, selectedIndex, hideWindow, copyPassword, copyUsername]);

  // Unlock handler
  const handleUnlock = useCallback(async () => {
    setUnlockLoading(true);
    setUnlockError("");
    try {
      await invoke("unlock", { password });
      setStatus("unlocked");
      setPassword("");
      const recent = await invoke<EntrySummary[]>("recent_entries", { limit: 5 });
      setResults(recent);
      setTimeout(() => inputRef.current?.focus(), 50);
    } catch (e: any) {
      setUnlockError(typeof e === "string" ? e : "Incorrect password");
    } finally {
      setUnlockLoading(false);
    }
  }, [password]);

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.15, ease: [0, 0, 0.2, 1] }}
      className="w-full h-full flex flex-col bg-[var(--color-bg-elevated)] rounded-[var(--radius-xl)] border border-[var(--color-border)] shadow-[var(--shadow-lg)] overflow-hidden"
      onKeyDown={handleKeyDown}
    >
      {status === "locked" ? (
        /* Locked state — inline unlock */
        <div className="flex flex-col items-center justify-center h-full p-[var(--spacing-xl)]">
          <Lock size={32} className="text-[var(--color-primary)] mb-[var(--spacing-lg)]" />
          <p className="text-[var(--font-size-md)] text-[var(--color-text-secondary)] mb-[var(--spacing-lg)]">
            Vault is locked
          </p>
          <form
            onSubmit={(e) => { e.preventDefault(); handleUnlock(); }}
            className="w-full max-w-[300px] flex flex-col gap-[var(--spacing-sm)]"
          >
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Master password"
              autoFocus
              className="h-9 px-[var(--spacing-md)] bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-md)] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] outline-none focus:border-[var(--color-primary)]"
            />
            {unlockError && (
              <p className="text-[var(--font-size-xs)] text-[var(--color-error)] text-center">
                {unlockError}
              </p>
            )}
            <Button type="submit" variant="primary" size="md" loading={unlockLoading} className="w-full">
              Unlock
            </Button>
          </form>
        </div>
      ) : (
        /* Unlocked state — search */
        <>
          {/* Search input */}
          <div className="flex items-center gap-[var(--spacing-sm)] px-[var(--spacing-lg)] py-[var(--spacing-md)] border-b border-[var(--color-border)]">
            <Search size={18} className="text-[var(--color-text-tertiary)] shrink-0" />
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Quick Access..."
              autoFocus
              className="flex-1 bg-transparent text-[var(--font-size-lg)] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] outline-none"
              aria-label="Quick Access search"
            />
          </div>

          {/* Results */}
          <div className="flex-1 overflow-y-auto">
            {!query && results.length > 0 && (
              <div className="px-[var(--spacing-lg)] py-[var(--spacing-xs)]">
                <span className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)]">
                  Recently used
                </span>
              </div>
            )}
            {results.length === 0 ? (
              <EmptyState
                title={query ? "No matching items" : "No recent items"}
                description={query ? "Try a different search" : undefined}
              />
            ) : (
              results.map((entry, index) => (
                <EntryCard
                  key={entry.id}
                  entry={entry}
                  selected={index === selectedIndex}
                  onClick={() => copyPassword(entry.id)}
                  compact
                />
              ))
            )}
          </div>

          {/* Keyboard hints */}
          <div className="flex items-center justify-center gap-[var(--spacing-lg)] px-[var(--spacing-lg)] py-[var(--spacing-xs)] border-t border-[var(--color-border-light)] text-[var(--font-size-xs)] text-[var(--color-text-tertiary)]">
            <span>↑↓ Navigate</span>
            <span>↵ Copy password</span>
            <span>⇧↵ Copy username</span>
            <span>esc Close</span>
          </div>
        </>
      )}
    </motion.div>
  );
}
