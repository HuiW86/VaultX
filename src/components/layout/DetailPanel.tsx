import { useState, useCallback, useEffect } from "react";
import { useVaultStore } from "../../stores/vaultStore";
import { EntryDetail } from "../entry/EntryDetail";
import { EntryForm } from "../entry/EntryForm";
import { DetailSkeleton } from "../ui/Skeleton";
import { ErrorState } from "../ui/ErrorState";

export type DetailMode = "view" | "edit" | "new";

export function DetailPanel() {
  const selectedEntryId = useVaultStore((s) => s.selectedEntryId);
  const selectedEntry = useVaultStore((s) => s.selectedEntry);
  const error = useVaultStore((s) => s.error);
  const selectEntry = useVaultStore((s) => s.selectEntry);
  const [mode, setMode] = useState<DetailMode>("view");
  const [newCategory, setNewCategory] = useState<string | null>(null);

  // Reset mode when selection changes
  useEffect(() => {
    setMode("view");
    setNewCategory(null);
  }, [selectedEntryId]);

  const handleNewEntry = useCallback((category: string) => {
    setNewCategory(category);
    setMode("new");
  }, []);

  const handleSaved = useCallback(
    (entryId?: string) => {
      setMode("view");
      setNewCategory(null);
      if (entryId) selectEntry(entryId);
    },
    [selectEntry]
  );

  const handleCancel = useCallback(() => {
    setMode("view");
    setNewCategory(null);
  }, []);

  // Expose handleNewEntry for EntryList's + button
  useEffect(() => {
    (window as any).__vaultx_newEntry = handleNewEntry;
    return () => {
      delete (window as any).__vaultx_newEntry;
    };
  }, [handleNewEntry]);

  // New entry mode
  if (mode === "new" && newCategory) {
    return (
      <EntryForm
        mode="new"
        category={newCategory}
        onSave={handleSaved}
        onCancel={handleCancel}
      />
    );
  }

  // Edit mode
  if (mode === "edit" && selectedEntry) {
    return (
      <EntryForm
        mode="edit"
        category={selectedEntry.entry.category}
        existingEntry={selectedEntry}
        onSave={handleSaved}
        onCancel={handleCancel}
      />
    );
  }

  // No selection
  if (!selectedEntryId) {
    return (
      <div className="h-full flex items-center justify-center">
        <p className="text-[var(--font-size-sm)] text-[var(--color-text-tertiary)]">
          Select an item to view details
        </p>
      </div>
    );
  }

  // Loading
  if (!selectedEntry && !error) {
    return <DetailSkeleton />;
  }

  // Error
  if (error) {
    return (
      <ErrorState
        message={error}
        onRetry={() => selectEntry(selectedEntryId)}
      />
    );
  }

  // View mode
  return (
    <EntryDetail
      entry={selectedEntry!}
      onEdit={() => setMode("edit")}
    />
  );
}
