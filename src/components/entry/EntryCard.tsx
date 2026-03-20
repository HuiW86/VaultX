import { Key, CreditCard, FileText, User, Terminal } from "lucide-react";
import type { EntrySummary } from "../../lib/commands";

const categoryIcons: Record<string, React.ComponentType<any>> = {
  login: Key,
  card: CreditCard,
  note: FileText,
  identity: User,
  ssh_key: Terminal,
};

const categoryColors: Record<string, string> = {
  login: "var(--color-primary)",
  card: "var(--color-warning)",
  note: "var(--color-accent)",
  identity: "var(--color-success)",
  ssh_key: "var(--color-text-secondary)",
};

interface EntryCardProps {
  entry: EntrySummary;
  selected: boolean;
  onClick: () => void;
  compact?: boolean;
  onContextMenu?: (e: React.MouseEvent) => void;
}

export function EntryCard({ entry, selected, onClick, compact, onContextMenu }: EntryCardProps) {
  const Icon = categoryIcons[entry.category] || Key;
  const color = categoryColors[entry.category] || "var(--color-text-secondary)";

  return (
    <button
      role="option"
      aria-selected={selected}
      onClick={onClick}
      onContextMenu={onContextMenu}
      className={`w-full flex items-center gap-[var(--spacing-sm)] px-[var(--spacing-md)] text-left transition-colors cursor-pointer ${
        compact ? "py-1 min-h-[40px]" : "py-[var(--spacing-sm)] min-h-[60px]"
      } ${
        selected
          ? "bg-[var(--color-primary-bg)] border-l-[3px] border-l-[var(--color-primary)]"
          : "hover:bg-[var(--color-bg-hover)] border-l-[3px] border-l-transparent"
      } ${entry.trashed ? "opacity-50" : ""}`}
    >
      <div
        className={`rounded-full flex items-center justify-center shrink-0 ${
          compact ? "w-6 h-6" : "w-8 h-8"
        }`}
        style={{ backgroundColor: `${color}20` }}
      >
        <Icon size={compact ? 12 : 16} style={{ color }} />
      </div>
      <div className="flex-1 min-w-0">
        <div className={`font-[var(--font-weight-medium)] text-[var(--color-text-primary)] truncate ${
          compact ? "text-[var(--font-size-sm)]" : "text-[var(--font-size-md)]"
        }`}>
          {entry.title}
        </div>
        {entry.subtitle && (
          <div className={`text-[var(--color-text-secondary)] truncate ${
            compact ? "text-[var(--font-size-xs)]" : "text-[var(--font-size-sm)]"
          }`}>
            {entry.subtitle}
          </div>
        )}
      </div>
    </button>
  );
}
