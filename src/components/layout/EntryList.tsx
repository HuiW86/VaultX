import { useEffect, useCallback } from "react";
import { Plus } from "lucide-react";
import { useVaultStore } from "../../stores/vaultStore";
import { useSearchStore } from "../../stores/searchStore";
import { EntryCard } from "../entry/EntryCard";
import { SearchBar } from "../search/SearchBar";
import { EmptyState } from "../ui/EmptyState";
import { ErrorState } from "../ui/ErrorState";
import { EntryCardSkeleton } from "../ui/Skeleton";

interface EntryListProps {
  onNewEntry?: () => void;
  searchInputRef?: React.RefObject<HTMLInputElement | null>;
}

export function EntryList({ onNewEntry, searchInputRef }: EntryListProps) {
  const entries = useVaultStore((s) => s.entries);
  const selectedEntryId = useVaultStore((s) => s.selectedEntryId);
  const selectEntry = useVaultStore((s) => s.selectEntry);
  const fetchEntries = useVaultStore((s) => s.fetchEntries);
  const loading = useVaultStore((s) => s.loading);
  const error = useVaultStore((s) => s.error);
  const showTrash = useVaultStore((s) => s.showTrash);

  const searchResults = useSearchStore((s) => s.results);
  const isSearchActive = useSearchStore((s) => s.isActive);

  useEffect(() => {
    fetchEntries();
  }, [fetchEntries]);

  // Use search results when search is active, otherwise vault entries
  const displayEntries = isSearchActive ? searchResults : entries;

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (displayEntries.length === 0) return;
      const currentIdx = displayEntries.findIndex((en) => en.id === selectedEntryId);

      if (e.key === "ArrowDown") {
        e.preventDefault();
        const next = Math.min(currentIdx + 1, displayEntries.length - 1);
        selectEntry(displayEntries[next].id);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        const prev = Math.max(currentIdx - 1, 0);
        selectEntry(displayEntries[prev].id);
      }
    },
    [displayEntries, selectedEntryId, selectEntry]
  );

  return (
    <div
      className="flex flex-col h-full"
      role="listbox"
      aria-label="Entries"
      tabIndex={0}
      onKeyDown={handleKeyDown}
    >
      {/* Search bar */}
      <SearchBar inputRef={searchInputRef} />

      {/* Header */}
      <div className="flex items-center justify-between px-[var(--spacing-md)] py-[var(--spacing-xs)]">
        <span className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)]">
          {isSearchActive
            ? `${searchResults.length} results`
            : showTrash
              ? "Trash"
              : `${entries.length} items`}
        </span>
        {!showTrash && !isSearchActive && onNewEntry && (
          <button
            onClick={onNewEntry}
            className="p-1 text-[var(--color-text-tertiary)] hover:text-[var(--color-primary)] rounded transition-colors"
            aria-label="New entry"
          >
            <Plus size={18} />
          </button>
        )}
      </div>

      {/* List */}
      <div className="flex-1 overflow-y-auto">
        {loading && entries.length === 0 && !isSearchActive ? (
          <>
            {[...Array(5)].map((_, i) => (
              <EntryCardSkeleton key={i} />
            ))}
          </>
        ) : error && !isSearchActive ? (
          <ErrorState message={error} onRetry={fetchEntries} />
        ) : displayEntries.length === 0 ? (
          isSearchActive ? (
            <EmptyState
              title="No matching items"
              description="Try a different search"
            />
          ) : showTrash ? (
            <EmptyState
              title="Trash is empty"
              description="Items you delete will appear here"
            />
          ) : (
            <EmptyState
              title="Your vault is empty"
              description="Add your first login to get started"
              actionLabel="+ Add Item"
              onAction={onNewEntry}
            />
          )
        ) : (
          displayEntries.map((entry) => (
            <EntryCard
              key={entry.id}
              entry={entry}
              selected={entry.id === selectedEntryId}
              onClick={() => selectEntry(entry.id)}
            />
          ))
        )}
      </div>
    </div>
  );
}
