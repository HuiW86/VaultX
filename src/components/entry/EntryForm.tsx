import { useState, useEffect, useCallback, type FormEvent } from "react";
import { Key, CreditCard, FileText, User, Terminal } from "lucide-react";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { PasswordField } from "../ui/PasswordField";
import { StrengthMeter } from "../ui/StrengthMeter";
import { Modal } from "../ui/Modal";
import { useToast } from "../ui/Toast";
import { useVaultStore } from "../../stores/vaultStore";
import { api, type EntryDetailResponse, type FieldInputDto } from "../../lib/commands";

// Category templates — PS:§2.4
const categoryTemplates: Record<string, { field_type: string; label: string }[]> = {
  login: [
    { field_type: "username", label: "Username" },
    { field_type: "password", label: "Password" },
    { field_type: "url", label: "Website" },
  ],
  card: [
    { field_type: "text", label: "Cardholder Name" },
    { field_type: "card_number", label: "Card Number" },
    { field_type: "text", label: "Expiry" },
    { field_type: "hidden", label: "CVV" },
    { field_type: "hidden", label: "PIN" },
  ],
  note: [{ field_type: "text", label: "Note" }],
  identity: [
    { field_type: "text", label: "Full Name" },
    { field_type: "text", label: "Email" },
    { field_type: "text", label: "Phone" },
    { field_type: "text", label: "Address" },
  ],
  ssh_key: [
    { field_type: "hidden", label: "Private Key" },
    { field_type: "text", label: "Public Key" },
    { field_type: "text", label: "Fingerprint" },
    { field_type: "password", label: "Passphrase" },
  ],
};

const categoryMeta = [
  { id: "login", label: "Login", icon: Key, color: "var(--color-primary)" },
  { id: "card", label: "Card", icon: CreditCard, color: "var(--color-warning)" },
  { id: "note", label: "Note", icon: FileText, color: "var(--color-accent)" },
  { id: "identity", label: "Identity", icon: User, color: "var(--color-success)" },
  { id: "ssh_key", label: "SSH Key", icon: Terminal, color: "var(--color-text-secondary)" },
];

interface EntryFormProps {
  mode: "new" | "edit";
  category: string;
  existingEntry?: EntryDetailResponse;
  onSave: (entryId?: string) => void;
  onCancel: () => void;
}

export function EntryForm({ mode, category, existingEntry, onSave, onCancel }: EntryFormProps) {
  const [title, setTitle] = useState(existingEntry?.entry.title || "");
  const [fields, setFields] = useState<{ field_type: string; label: string; value: string }[]>([]);
  const [titleError, setTitleError] = useState("");
  const [loading, setLoading] = useState(false);
  const [showDiscardModal, setShowDiscardModal] = useState(false);
  const [dirty, setDirty] = useState(false);
  const { toast } = useToast();
  const createEntry = useVaultStore((s) => s.createEntry);
  const updateEntry = useVaultStore((s) => s.updateEntry);

  // Initialize fields from template or existing entry
  useEffect(() => {
    if (existingEntry) {
      setFields(
        existingEntry.fields.map((f) => ({
          field_type: f.field_type,
          label: f.label,
          value: f.value,
        }))
      );
    } else {
      const template = categoryTemplates[category] || [];
      setFields(template.map((t) => ({ ...t, value: "" })));
      // Auto-generate password for new Login entries
      if (category === "login") {
        api.generatePassword().then((pwd) => {
          setFields((prev) =>
            prev.map((f) =>
              f.field_type === "password" ? { ...f, value: pwd } : f
            )
          );
        });
      }
    }
  }, [category, existingEntry]);

  const updateField = useCallback((index: number, value: string) => {
    setFields((prev) => prev.map((f, i) => (i === index ? { ...f, value } : f)));
    setDirty(true);
  }, []);

  const handleSubmit = useCallback(
    async (e?: FormEvent) => {
      e?.preventDefault();
      if (!title.trim()) {
        setTitleError("Title is required");
        return;
      }
      setLoading(true);
      try {
        const fieldInputs: FieldInputDto[] = fields.map((f, i) => ({
          field_type: f.field_type,
          label: f.label,
          value: f.value,
          sort_order: i,
        }));

        // Compute subtitle
        let subtitle: string | undefined;
        if (category === "login") subtitle = fields.find((f) => f.field_type === "username")?.value;
        else if (category === "card") {
          const num = fields.find((f) => f.field_type === "card_number")?.value || "";
          subtitle = num.length >= 4 ? `•••• ${num.slice(-4)}` : undefined;
        } else if (category === "note") subtitle = fields[0]?.value?.slice(0, 40);
        else if (category === "identity") subtitle = fields.find((f) => f.label === "Full Name")?.value;
        else if (category === "ssh_key") subtitle = fields.find((f) => f.label === "Fingerprint")?.value;

        if (mode === "new") {
          // Need vault ID — get first vault
          const vaults = await api.listVaults();
          if (vaults.length === 0) throw new Error("No vault found");
          const result = await createEntry({
            vault_id: vaults[0].id,
            category,
            title: title.trim(),
            subtitle,
            fields: fieldInputs,
          });
          toast("Saved", "success");
          onSave(result.id);
        } else if (existingEntry) {
          await updateEntry({
            entry_id: existingEntry.entry.id,
            title: title.trim(),
            subtitle,
            fields: fieldInputs,
          });
          toast("Saved", "success");
          onSave(existingEntry.entry.id);
        }
      } catch (e) {
        toast(String(e), "error");
      } finally {
        setLoading(false);
      }
    },
    [title, fields, category, mode, existingEntry, createEntry, updateEntry, onSave, toast]
  );

  const handleCancel = useCallback(() => {
    if (dirty) {
      setShowDiscardModal(true);
    } else {
      onCancel();
    }
  }, [dirty, onCancel]);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "s" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        handleSubmit();
      } else if (e.key === "Escape") {
        handleCancel();
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [handleSubmit, handleCancel]);

  return (
    <div className="p-[var(--spacing-2xl)] h-full overflow-y-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-[var(--spacing-xl)]">
        <h2 className="text-[var(--font-size-h2)] font-[var(--font-weight-semibold)]">
          {mode === "new" ? `New ${categoryMeta.find((c) => c.id === category)?.label}` : "Edit"}
        </h2>
        <div className="flex gap-[var(--spacing-sm)]">
          <Button variant="ghost" onClick={handleCancel}>Cancel</Button>
          <Button variant="primary" onClick={() => handleSubmit()} loading={loading}>
            Save
          </Button>
        </div>
      </div>

      {/* Category selector for new entries */}
      {mode === "new" && (
        <div className="flex gap-[var(--spacing-sm)] mb-[var(--spacing-xl)]">
          {categoryMeta.map(({ id, label, icon: Icon, color }) => (
            <button
              key={id}
              onClick={() => {
                // Re-initialize form with new category
                window.location.hash = ""; // Force re-render
                setFields(
                  (categoryTemplates[id] || []).map((t) => ({ ...t, value: "" }))
                );
                setTitle("");
              }}
              className={`flex items-center gap-[var(--spacing-xs)] px-[var(--spacing-md)] py-[var(--spacing-sm)] rounded-[var(--radius-md)] text-[var(--font-size-sm)] transition-colors cursor-pointer ${
                id === category
                  ? "bg-[var(--color-primary-bg)] text-[var(--color-primary)]"
                  : "bg-[var(--color-bg-hover)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]"
              }`}
            >
              <Icon size={14} style={{ color: id === category ? color : undefined }} />
              {label}
            </button>
          ))}
        </div>
      )}

      {/* Form */}
      <form onSubmit={handleSubmit} className="flex flex-col gap-[var(--spacing-lg)]">
        <Input
          label="Title"
          required
          value={title}
          onChange={(e) => {
            setTitle(e.target.value);
            setTitleError("");
            setDirty(true);
          }}
          onBlur={() => {
            if (!title.trim()) setTitleError("Title is required");
          }}
          error={titleError}
          autoFocus
        />

        {fields.map((field, index) => {
          if (field.field_type === "password") {
            return (
              <div key={index} className="flex flex-col gap-[var(--spacing-xs)]">
                <PasswordField
                  label={field.label}
                  value={field.value}
                  onChange={(v) => updateField(index, v)}
                />
                <StrengthMeter password={field.value} />
              </div>
            );
          }

          if (field.field_type === "hidden" || field.field_type === "card_number") {
            return (
              <PasswordField
                key={index}
                label={field.label}
                value={field.value}
                onChange={(v) => updateField(index, v)}
              />
            );
          }

          if (field.label === "Note" || field.label === "Address" || field.label === "Private Key" || field.label === "Public Key") {
            return (
              <div key={index} className="flex flex-col gap-[var(--spacing-xs)]">
                <label className="text-[var(--font-size-xs)] font-[var(--font-weight-medium)] text-[var(--color-text-secondary)]">
                  {field.label}
                </label>
                <textarea
                  value={field.value}
                  onChange={(e) => updateField(index, e.target.value)}
                  rows={field.label === "Private Key" || field.label === "Public Key" ? 6 : 3}
                  className="px-[var(--spacing-md)] py-[var(--spacing-sm)] bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-md)] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-primary)] resize-y font-[var(--font-mono)]"
                  style={{ fontFamily: field.label.includes("Key") ? "var(--font-mono)" : undefined }}
                />
              </div>
            );
          }

          return (
            <Input
              key={index}
              label={field.label}
              value={field.value}
              onChange={(e) => updateField(index, e.target.value)}
            />
          );
        })}
      </form>

      <Modal
        open={showDiscardModal}
        onClose={() => setShowDiscardModal(false)}
        title="Discard changes?"
        danger
        confirmLabel="Discard"
        cancelLabel="Keep Editing"
        onConfirm={() => {
          setShowDiscardModal(false);
          onCancel();
        }}
      >
        <p>You have unsaved changes that will be lost.</p>
      </Modal>
    </div>
  );
}
