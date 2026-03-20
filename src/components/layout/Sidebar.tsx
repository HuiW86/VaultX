import { useEffect } from "react";
import {
  Key,
  CreditCard,
  FileText,
  User,
  Terminal,
  Star,
  Trash2,
  Lock,
  LayoutGrid,
} from "lucide-react";
import { useVaultStore } from "../../stores/vaultStore";
import { useAppStore } from "../../stores/appStore";

const categories = [
  { id: null, label: "All Items", icon: LayoutGrid },
  { id: "login", label: "Logins", icon: Key },
  { id: "card", label: "Cards", icon: CreditCard },
  { id: "note", label: "Notes", icon: FileText },
  { id: "identity", label: "Identities", icon: User },
  { id: "ssh_key", label: "SSH Keys", icon: Terminal },
] as const;

export function Sidebar() {
  const categoryFilter = useVaultStore((s) => s.categoryFilter);
  const showTrash = useVaultStore((s) => s.showTrash);
  const setCategoryFilter = useVaultStore((s) => s.setCategoryFilter);
  const setShowTrash = useVaultStore((s) => s.setShowTrash);
  const categoryCounts = useVaultStore((s) => s.categoryCounts);
  const fetchCategoryCounts = useVaultStore((s) => s.fetchCategoryCounts);
  const lock = useAppStore((s) => s.lock);

  useEffect(() => {
    fetchCategoryCounts();
  }, [fetchCategoryCounts]);

  const totalCount = Object.values(categoryCounts).reduce((a, b) => a + b, 0);

  return (
    <div className="flex flex-col h-full px-[var(--spacing-sm)] py-[var(--spacing-sm)]">
      {/* Vaults section */}
      <div className="mb-[var(--spacing-lg)]">
        <h3 className="text-[var(--font-size-xs)] font-[var(--font-weight-medium)] text-[var(--color-text-tertiary)] uppercase px-[var(--spacing-sm)] mb-[var(--spacing-xs)]">
          Vaults
        </h3>
        <div className="px-[var(--spacing-sm)] py-[var(--spacing-xs)] text-[var(--font-size-md)] font-[var(--font-weight-medium)] text-[var(--color-text-primary)]">
          Personal
        </div>
      </div>

      {/* Categories */}
      <div className="flex-1">
        <h3 className="text-[var(--font-size-xs)] font-[var(--font-weight-medium)] text-[var(--color-text-tertiary)] uppercase px-[var(--spacing-sm)] mb-[var(--spacing-xs)]">
          Categories
        </h3>
        <div className="flex flex-col gap-px">
          {categories.map(({ id, label, icon: Icon }) => {
            const active = !showTrash && categoryFilter === id;
            const count = id === null ? totalCount : (categoryCounts[id] || 0);
            return (
              <button
                key={label}
                onClick={() => {
                  setShowTrash(false);
                  setCategoryFilter(id);
                }}
                className={`flex items-center gap-[var(--spacing-sm)] px-[var(--spacing-sm)] py-[5px] rounded-[var(--radius-md)] text-[var(--font-size-md)] font-[var(--font-weight-medium)] transition-colors cursor-pointer ${
                  active
                    ? "bg-[var(--color-primary-bg)] text-[var(--color-primary)]"
                    : "text-[var(--color-text-primary)] hover:bg-[var(--color-bg-hover)]"
                }`}
              >
                <Icon size={16} />
                <span className="flex-1 text-left">{label}</span>
                <span className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)]">
                  {count || ""}
                </span>
              </button>
            );
          })}
        </div>

        {/* Favorites + Trash */}
        <div className="mt-[var(--spacing-lg)] flex flex-col gap-px">
          <button
            onClick={() => {
              setShowTrash(false);
              // TODO: favorites filter in M3
            }}
            className="flex items-center gap-[var(--spacing-sm)] px-[var(--spacing-sm)] py-[5px] rounded-[var(--radius-md)] text-[var(--font-size-md)] font-[var(--font-weight-medium)] text-[var(--color-text-primary)] hover:bg-[var(--color-bg-hover)] cursor-pointer"
          >
            <Star size={16} />
            <span>Favorites</span>
          </button>
          <button
            onClick={() => setShowTrash(true)}
            className={`flex items-center gap-[var(--spacing-sm)] px-[var(--spacing-sm)] py-[5px] rounded-[var(--radius-md)] text-[var(--font-size-md)] font-[var(--font-weight-medium)] transition-colors cursor-pointer ${
              showTrash
                ? "bg-[var(--color-primary-bg)] text-[var(--color-primary)]"
                : "text-[var(--color-text-primary)] hover:bg-[var(--color-bg-hover)]"
            }`}
          >
            <Trash2 size={16} />
            <span>Trash</span>
          </button>
        </div>
      </div>

      {/* Bottom actions */}
      <div className="border-t border-[var(--color-border-light)] pt-[var(--spacing-sm)] flex flex-col gap-px">
        <button
          onClick={lock}
          className="flex items-center gap-[var(--spacing-sm)] px-[var(--spacing-sm)] py-[5px] rounded-[var(--radius-md)] text-[var(--font-size-md)] font-[var(--font-weight-medium)] text-[var(--color-text-primary)] hover:bg-[var(--color-bg-hover)] cursor-pointer"
        >
          <Lock size={16} />
          <span>Lock</span>
          <span className="ml-auto text-[var(--font-size-xs)] text-[var(--color-text-tertiary)]">
            ⌘L
          </span>
        </button>
      </div>
    </div>
  );
}
