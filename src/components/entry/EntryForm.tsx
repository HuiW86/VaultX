import { useState, useEffect, useCallback, type FormEvent } from "react";
import { Key, CreditCard, FileText, User, Terminal } from "lucide-react";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { PasswordField } from "../ui/PasswordField";
import { StrengthMeter } from "../ui/StrengthMeter";
import { PasswordGenerator } from "./PasswordGenerator";
import { Modal } from "../ui/Modal";
import { useToast } from "../ui/Toast";
import { useVaultStore } from "../../stores/vaultStore";
import { api, type EntryDetailResponse, type FieldInputDto } from "../../lib/commands";
import { useTranslation, type TranslationKey } from "../../i18n";

// Category templates — PS:§2.4
// Labels use i18n keys — translated at render time via t()
const categoryTemplates: Record<string, { field_type: string; labelKey: TranslationKey }[]> = {
  login: [
    { field_type: "username", labelKey: "form.username" },
    { field_type: "password", labelKey: "form.password" },
    { field_type: "url", labelKey: "form.website" },
  ],
  card: [
    { field_type: "text", labelKey: "form.cardholder" },
    { field_type: "card_number", labelKey: "form.card_number" },
    { field_type: "text", labelKey: "form.expiry" },
    { field_type: "hidden", labelKey: "form.cvv" },
    { field_type: "hidden", labelKey: "form.pin" },
  ],
  note: [{ field_type: "text", labelKey: "form.note" }],
  identity: [
    { field_type: "text", labelKey: "form.full_name" },
    { field_type: "text", labelKey: "form.email" },
    { field_type: "text", labelKey: "form.phone" },
    { field_type: "text", labelKey: "form.address" },
  ],
  ssh_key: [
    { field_type: "hidden", labelKey: "form.private_key" },
    { field_type: "text", labelKey: "form.public_key" },
    { field_type: "text", labelKey: "form.fingerprint" },
    { field_type: "password", labelKey: "form.passphrase" },
  ],
};

const categoryMeta: { id: string; labelKey: TranslationKey; icon: typeof Key; color: string }[] = [
  { id: "login", labelKey: "category.login", icon: Key, color: "var(--color-primary)" },
  { id: "card", labelKey: "category.card", icon: CreditCard, color: "var(--color-warning)" },
  { id: "note", labelKey: "category.note", icon: FileText, color: "var(--color-accent)" },
  { id: "identity", labelKey: "category.identity", icon: User, color: "var(--color-success)" },
  { id: "ssh_key", labelKey: "category.ssh_key", icon: Terminal, color: "var(--color-text-secondary)" },
];

// Reverse lookup: stored label text → i18n key (for editing existing entries)
const labelToKey: Record<string, TranslationKey> = {
  // English labels
  "Username": "form.username", "Password": "form.password", "Website": "form.website",
  "Cardholder Name": "form.cardholder", "Card Number": "form.card_number", "Expiry": "form.expiry",
  "CVV": "form.cvv", "PIN": "form.pin", "Note": "form.note",
  "Full Name": "form.full_name", "Email": "form.email", "Phone": "form.phone",
  "Address": "form.address", "Private Key": "form.private_key", "Public Key": "form.public_key",
  "Fingerprint": "form.fingerprint", "Passphrase": "form.passphrase",
  // Chinese labels
  "用户名": "form.username", "密码": "form.password", "网址": "form.website",
  "持卡人姓名": "form.cardholder", "卡号": "form.card_number", "有效期": "form.expiry",
  "备注": "form.note", "姓名": "form.full_name", "邮箱": "form.email",
  "电话": "form.phone", "地址": "form.address", "私钥": "form.private_key",
  "公钥": "form.public_key", "指纹": "form.fingerprint", "密码短语": "form.passphrase",
};

interface EntryFormProps {
  mode: "new" | "edit";
  category: string;
  existingEntry?: EntryDetailResponse;
  onSave: (entryId?: string) => void;
  onCancel: () => void;
}

export function EntryForm({ mode, category: initialCategory, existingEntry, onSave, onCancel }: EntryFormProps) {
  const { t } = useTranslation();
  const [category, setCategory] = useState(initialCategory);
  const [title, setTitle] = useState(existingEntry?.entry.title || "");
  const [fields, setFields] = useState<{ field_type: string; labelKey: TranslationKey; value: string }[]>([]);
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
          labelKey: labelToKey[f.label] || f.label,
          value: f.value,
        }))
      );
    } else {
      const template = categoryTemplates[category] || [];
      setFields(template.map((tmpl) => ({ ...tmpl, value: "" })));
      // Auto-generate password for new Login entries
      if (category === "login") {
        api.generatePassword().then(({ password }) => {
          setFields((prev) =>
            prev.map((f) =>
              f.field_type === "password" ? { ...f, value: password } : f
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
        setTitleError(t("form.title_required"));
        return;
      }
      setLoading(true);
      try {
        const fieldInputs: FieldInputDto[] = fields.map((f, i) => ({
          field_type: f.field_type,
          label: t(f.labelKey),
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
        else if (category === "identity") subtitle = fields.find((f) => f.labelKey === "form.full_name")?.value;
        else if (category === "ssh_key") subtitle = fields.find((f) => f.labelKey === "form.fingerprint")?.value;

        if (mode === "new") {
          // Need vault ID — get first vault
          const vaults = await api.listVaults();
          if (vaults.length === 0) throw new Error(t("form.no_vault"));
          const result = await createEntry({
            vault_id: vaults[0].id,
            category,
            title: title.trim(),
            subtitle,
            fields: fieldInputs,
          });
          toast(t("form.saved"), "success");
          onSave(result.id);
        } else if (existingEntry) {
          await updateEntry({
            entry_id: existingEntry.entry.id,
            title: title.trim(),
            subtitle,
            fields: fieldInputs,
          });
          toast(t("form.saved"), "success");
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
          {mode === "new" ? t("form.new", { category: t(categoryMeta.find((c) => c.id === category)?.labelKey ?? "category.login") }) : t("form.edit")}
        </h2>
        <div className="flex gap-[var(--spacing-sm)]">
          <Button variant="ghost" onClick={handleCancel}>{t("form.cancel")}</Button>
          <Button variant="primary" onClick={() => handleSubmit()} loading={loading}>
            {t("form.save")}
          </Button>
        </div>
      </div>

      {/* Category selector for new entries */}
      {mode === "new" && (
        <div className="flex gap-[var(--spacing-sm)] mb-[var(--spacing-xl)]">
          {categoryMeta.map(({ id, labelKey, icon: Icon, color }) => (
            <button
              key={id}
              onClick={() => {
                setCategory(id);
                setFields(
                  (categoryTemplates[id] || []).map((tmpl) => ({ ...tmpl, value: "" }))
                );
                setTitle("");
                setDirty(false);
              }}
              className={`flex items-center gap-[var(--spacing-xs)] px-[var(--spacing-md)] py-[var(--spacing-sm)] rounded-[var(--radius-md)] text-[var(--font-size-sm)] transition-colors cursor-pointer ${
                id === category
                  ? "bg-[var(--color-primary-bg)] text-[var(--color-primary)]"
                  : "bg-[var(--color-bg-hover)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]"
              }`}
            >
              <Icon size={14} style={{ color: id === category ? color : undefined }} />
              {t(labelKey)}
            </button>
          ))}
        </div>
      )}

      {/* Form */}
      <form onSubmit={handleSubmit} className="flex flex-col gap-[var(--spacing-lg)]">
        <Input
          label={t("form.title")}
          required
          value={title}
          onChange={(e) => {
            setTitle(e.target.value);
            setTitleError("");
            setDirty(true);
          }}
          onBlur={() => {
            if (!title.trim()) setTitleError(t("form.title_required"));
          }}
          error={titleError}
          autoFocus
        />

        {fields.map((field, index) => {
          const label = t(field.labelKey);
          if (field.field_type === "password") {
            return (
              <div key={index} className="flex flex-col gap-[var(--spacing-xs)]">
                <PasswordField
                  label={label}
                  value={field.value}
                  onChange={(v) => updateField(index, v)}
                />
                <StrengthMeter password={field.value} />
                <PasswordGenerator onUse={(v) => updateField(index, v)} />
              </div>
            );
          }

          if (field.field_type === "hidden" || field.field_type === "card_number") {
            return (
              <PasswordField
                key={index}
                label={label}
                value={field.value}
                onChange={(v) => updateField(index, v)}
              />
            );
          }

          const textareaKeys = ["form.note", "form.address", "form.private_key", "form.public_key"];
          const isKeyField = field.labelKey === "form.private_key" || field.labelKey === "form.public_key";
          if (textareaKeys.includes(field.labelKey)) {
            return (
              <div key={index} className="flex flex-col gap-[var(--spacing-xs)]">
                <label className="text-[var(--font-size-xs)] font-[var(--font-weight-medium)] text-[var(--color-text-secondary)]">
                  {label}
                </label>
                <textarea
                  value={field.value}
                  onChange={(e) => updateField(index, e.target.value)}
                  rows={isKeyField ? 6 : 3}
                  className="px-[var(--spacing-md)] py-[var(--spacing-sm)] bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-md)] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-primary)] resize-y font-[var(--font-mono)]"
                  style={{ fontFamily: isKeyField ? "var(--font-mono)" : undefined }}
                />
              </div>
            );
          }

          return (
            <Input
              key={index}
              label={label}
              value={field.value}
              onChange={(e) => updateField(index, e.target.value)}
            />
          );
        })}
      </form>

      <Modal
        open={showDiscardModal}
        onClose={() => setShowDiscardModal(false)}
        title={t("form.discard_title")}
        danger
        confirmLabel={t("form.discard")}
        cancelLabel={t("form.keep_editing")}
        onConfirm={() => {
          setShowDiscardModal(false);
          onCancel();
        }}
      >
        <p>{t("form.discard_desc")}</p>
      </Modal>
    </div>
  );
}
