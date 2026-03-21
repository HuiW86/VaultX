import { useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Copy, Pencil, Trash2, Link, User } from "lucide-react";
import { useTranslation } from "../../i18n";

interface ContextMenuItem {
  label: string;
  icon: React.ComponentType<{ size: number }>;
  shortcut?: string;
  danger?: boolean;
  action: () => void;
}

interface ContextMenuProps {
  open: boolean;
  x: number;
  y: number;
  onClose: () => void;
  items: ContextMenuItem[];
}

export function ContextMenu({ open, x, y, onClose, items }: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);

  // Close on click outside or Escape
  useEffect(() => {
    if (!open) return;
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [open, onClose]);

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          ref={menuRef}
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0, scale: 0.95 }}
          transition={{ duration: 0.1 }}
          className="fixed z-50 min-w-[180px] max-w-[260px] py-1 bg-[var(--color-bg-elevated)] border border-[var(--color-border)] rounded-[var(--radius-lg)] shadow-[var(--shadow-md)]"
          style={{ left: x, top: y }}
          role="menu"
        >
          {items.map((item, index) => (
            <button
              key={index}
              role="menuitem"
              onClick={() => {
                item.action();
                onClose();
              }}
              className={`w-full flex items-center gap-[var(--spacing-sm)] px-[var(--spacing-md)] h-8 text-[var(--font-size-sm)] transition-colors cursor-pointer ${
                item.danger
                  ? "text-[var(--color-error)] hover:bg-[var(--color-error-bg)]"
                  : "text-[var(--color-text-primary)] hover:bg-[var(--color-bg-hover)]"
              }`}
            >
              <item.icon size={14} />
              <span className="flex-1 text-left">{item.label}</span>
              {item.shortcut && (
                <span className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)]">
                  {item.shortcut}
                </span>
              )}
            </button>
          ))}
        </motion.div>
      )}
    </AnimatePresence>
  );
}

// Pre-built menu items factory for entries
export function useEntryContextMenu() {
  const { t } = useTranslation();
  return {
    buildItems: (callbacks: {
      onCopyUsername?: () => void;
      onCopyPassword?: () => void;
      onCopyUrl?: () => void;
      onEdit?: () => void;
      onTrash?: () => void;
    }): ContextMenuItem[] => {
      const items: ContextMenuItem[] = [];
      if (callbacks.onCopyUsername) {
        items.push({
          label: t("context.copy_username"),
          icon: User,
          shortcut: "⇧⌘U",
          action: callbacks.onCopyUsername,
        });
      }
      if (callbacks.onCopyPassword) {
        items.push({
          label: t("context.copy_password"),
          icon: Copy,
          shortcut: "⇧⌘C",
          action: callbacks.onCopyPassword,
        });
      }
      if (callbacks.onCopyUrl) {
        items.push({
          label: t("context.copy_url"),
          icon: Link,
          action: callbacks.onCopyUrl,
        });
      }
      if (items.length > 0 && (callbacks.onEdit || callbacks.onTrash)) {
        // Visual separator via margin
      }
      if (callbacks.onEdit) {
        items.push({
          label: t("context.edit"),
          icon: Pencil,
          shortcut: "⌘E",
          action: callbacks.onEdit,
        });
      }
      if (callbacks.onTrash) {
        items.push({
          label: t("context.move_to_trash"),
          icon: Trash2,
          shortcut: "⌘⌫",
          danger: true,
          action: callbacks.onTrash,
        });
      }
      return items;
    },
  };
}
