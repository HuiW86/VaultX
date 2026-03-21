import { useState } from "react";
import { Key, CreditCard, FileText, User, Terminal, Pencil, Trash2, ExternalLink } from "lucide-react";
import type { EntryDetailResponse } from "../../lib/commands";
import { CopyButton } from "../ui/CopyButton";
import { PasswordField } from "../ui/PasswordField";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { useVaultStore } from "../../stores/vaultStore";
import { useTranslation } from "../../i18n";

const categoryIcons: Record<string, React.ComponentType<{ size: number }>> = {
  login: Key, card: CreditCard, note: FileText, identity: User, ssh_key: Terminal,
};

interface EntryDetailProps {
  entry: EntryDetailResponse;
  onEdit: () => void;
}

export function EntryDetail({ entry, onEdit }: EntryDetailProps) {
  const { t } = useTranslation();
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const trashEntry = useVaultStore((s) => s.trashEntry);
  const isTrashed = entry.entry.trashed;
  const Icon = categoryIcons[entry.entry.category] || Key;

  const handleDelete = async () => {
    await trashEntry(entry.entry.id);
    setShowDeleteModal(false);
  };

  return (
    <div className="p-[var(--spacing-2xl)] h-full overflow-y-auto">
      {/* Header */}
      <div className="flex items-start gap-[var(--spacing-md)] mb-[var(--spacing-xl)]">
        <div
          className="w-12 h-12 rounded-[var(--radius-lg)] flex items-center justify-center shrink-0"
          style={{ backgroundColor: "var(--color-primary-bg)" }}
        >
          <Icon size={24} />
        </div>
        <div className="flex-1 min-w-0">
          <h1 className="text-[var(--font-size-h1)] font-[var(--font-weight-semibold)] text-[var(--color-text-primary)] truncate">
            {entry.entry.title}
          </h1>
          {entry.entry.subtitle && (
            <p className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)]">
              {entry.entry.subtitle}
            </p>
          )}
        </div>
      </div>

      {/* Fields */}
      <div className="flex flex-col gap-[var(--spacing-lg)] mb-[var(--spacing-xl)]">
        {entry.fields.map((field) => (
          <div key={field.id} className="flex flex-col gap-[var(--spacing-xs)]">
            <label className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)]">
              {field.label}
            </label>
            {field.sensitive ? (
              <PasswordField
                value={field.value}
                readOnly
                showCopy
                label={field.label}
              />
            ) : (
              <div className="flex items-center gap-[var(--spacing-xs)]">
                <span
                  className={`flex-1 text-[var(--font-size-md)] text-[var(--color-text-primary)] ${
                    field.field_type === "url"
                      ? "text-[var(--color-primary)] cursor-pointer hover:underline"
                      : ""
                  }`}
                >
                  {field.value || "—"}
                </span>
                {field.value && (
                  <div className="flex items-center gap-0.5">
                    <CopyButton value={field.value} label={t("detail.copy_field", { label: field.label })} />
                    {field.field_type === "url" && field.value && (
                      <button
                        className="p-1.5 text-[var(--color-text-tertiary)] hover:text-[var(--color-primary)] rounded transition-colors"
                        aria-label={t("detail.open_url")}
                      >
                        <ExternalLink size={16} />
                      </button>
                    )}
                  </div>
                )}
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Metadata */}
      <div className="border-t border-[var(--color-border-light)] pt-[var(--spacing-lg)] mb-[var(--spacing-xl)]">
        <div className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)] flex flex-col gap-1">
          <span>{t("detail.created", { date: new Date(entry.entry.created_at).toLocaleDateString() })}</span>
          <span>{t("detail.modified", { date: new Date(entry.entry.updated_at).toLocaleDateString() })}</span>
        </div>
      </div>

      {/* Actions */}
      {!isTrashed && (
        <div className="flex gap-[var(--spacing-sm)]">
          <Button variant="secondary" onClick={onEdit}>
            <Pencil size={14} />
            {t("detail.edit")}
          </Button>
          <Button variant="ghost" onClick={() => setShowDeleteModal(true)}>
            <Trash2 size={14} />
            {t("detail.move_to_trash")}
          </Button>
        </div>
      )}

      <Modal
        open={showDeleteModal}
        onClose={() => setShowDeleteModal(false)}
        title={t("detail.trash_confirm_title")}
        danger
        confirmLabel={t("detail.move_to_trash")}
        cancelLabel={t("detail.keep")}
        onConfirm={handleDelete}
      >
        <p>{t("detail.trash_confirm_desc", { title: entry.entry.title })}</p>
      </Modal>
    </div>
  );
}
