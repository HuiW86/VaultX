import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import { api, type VaultxSettings } from "../../lib/commands";
import { useTranslation } from "../../i18n";

interface SettingsPanelProps {
  onClose: () => void;
}

export function SettingsPanel({ onClose }: SettingsPanelProps) {
  const settings = useSettingsStore((s) => s.settings);
  const loaded = useSettingsStore((s) => s.loaded);
  const load = useSettingsStore((s) => s.load);
  const update = useSettingsStore((s) => s.update);
  const [touchIdAvailable, setTouchIdAvailable] = useState(false);
  const [touchIdLoading, setTouchIdLoading] = useState(false);
  const { t } = useTranslation();

  const autoLockOptions = [
    { value: 1, label: t("settings.1_minute") },
    { value: 5, label: t("settings.5_minutes") },
    { value: 15, label: t("settings.15_minutes") },
    { value: 60, label: t("settings.1_hour") },
    { value: 240, label: t("settings.4_hours") },
    { value: 480, label: t("settings.8_hours") },
    { value: -1, label: t("settings.never") },
  ];

  const clipboardOptions = [
    { value: 15, label: t("settings.15_seconds") },
    { value: 30, label: t("settings.30_seconds") },
    { value: 60, label: t("settings.1_minute") },
    { value: 120, label: t("settings.2_minutes") },
    { value: -1, label: t("settings.never") },
  ];

  const themeOptions = [
    { value: "dark", label: t("settings.dark") },
    { value: "light", label: t("settings.light") },
    { value: "system", label: t("settings.system") },
  ];

  useEffect(() => {
    if (!loaded) load();
    api.isTouchIdAvailable().then(setTouchIdAvailable).catch(() => {});
  }, [loaded, load]);

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);

  const handleChange = (key: keyof VaultxSettings, value: VaultxSettings[keyof VaultxSettings]) => {
    update({ [key]: value });
  };

  if (!loaded) return null;

  return (
    <div className="h-full flex flex-col bg-[var(--color-bg-panel)] overflow-y-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-[var(--spacing-lg)] py-[var(--spacing-md)] border-b border-[var(--color-border)]">
        <h2 className="text-[var(--font-size-lg)] font-[var(--font-weight-semibold)] text-[var(--color-text-primary)]">
          {t("settings.title")}
        </h2>
        <button
          onClick={onClose}
          className="p-[var(--spacing-xs)] rounded-[var(--radius-md)] text-[var(--color-text-tertiary)] hover:bg-[var(--color-bg-hover)] cursor-pointer"
        >
          <X size={18} />
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 px-[var(--spacing-lg)] py-[var(--spacing-md)] space-y-[var(--spacing-lg)]">
        {/* General */}
        <Section title={t("settings.general")}>
          <SwitchRow
            label={t("settings.start_at_login")}
            checked={settings.start_at_login}
            onChange={(v) => handleChange("start_at_login", v)}
          />
          <SwitchRow
            label={t("settings.show_in_menu_bar")}
            checked={settings.show_in_menu_bar}
            onChange={(v) => handleChange("show_in_menu_bar", v)}
          />
        </Section>

        {/* Security */}
        <Section title={t("settings.security")}>
          <SelectRow
            label={t("settings.auto_lock")}
            value={settings.auto_lock_timeout_minutes}
            options={autoLockOptions}
            onChange={(v) => handleChange("auto_lock_timeout_minutes", v)}
          />
          <SwitchRow
            label={t("settings.lock_on_sleep")}
            checked={settings.lock_on_sleep}
            onChange={(v) => handleChange("lock_on_sleep", v)}
          />
          <SelectRow
            label={t("settings.clipboard_clear")}
            value={settings.clipboard_clear_seconds}
            options={clipboardOptions}
            onChange={(v) => handleChange("clipboard_clear_seconds", v)}
          />
          <SwitchRow
            label={t("settings.touch_id")}
            checked={settings.touch_id_enabled}
            onChange={async (v) => {
              if (touchIdLoading) return;
              setTouchIdLoading(true);
              try {
                if (v) {
                  await api.setupTouchId();
                } else {
                  await api.disableTouchId();
                }
                await update({ touch_id_enabled: v });
              } catch {
                // Touch ID setup failed — keep current state
              } finally {
                setTouchIdLoading(false);
              }
            }}
            disabled={!touchIdAvailable || touchIdLoading}
          />
        </Section>

        {/* Appearance */}
        <Section title={t("settings.appearance")}>
          <SelectRow
            label={t("settings.theme")}
            value={settings.theme}
            options={themeOptions}
            onChange={(v) => handleChange("theme", v)}
          />
          <div className="flex items-center justify-between">
            <span>{t("settings.language")}</span>
            <select
              className="bg-[var(--color-bg-elevated)] text-[var(--color-text-primary)] text-[var(--font-size-sm)] px-[var(--spacing-sm)] py-[var(--spacing-xs)] rounded-[var(--radius-md)] border border-[var(--color-border)] cursor-pointer outline-none"
              value={settings.language}
              onChange={(e) => update({ language: e.target.value })}
            >
              <option value="en">{t("settings.lang_en")}</option>
              <option value="zh-CN">{t("settings.lang_zh")}</option>
            </select>
          </div>
        </Section>

        {/* Shortcuts (read-only) */}
        <Section title={t("settings.shortcuts")}>
          <ShortcutRow label={t("settings.quick_access")} shortcut="Cmd+Shift+Space" />
          <ShortcutRow label={t("settings.lock")} shortcut="Cmd+L" />
          <ShortcutRow label={t("settings.search")} shortcut="Cmd+K" />
          <ShortcutRow label={t("settings.new_item")} shortcut="Cmd+N" />
        </Section>

        {/* Data (placeholders for future) */}
        <Section title={t("settings.data")}>
          <ActionRow label={t("settings.import")} disabled />
          <ActionRow label={t("settings.export")} disabled />
        </Section>

        {/* About */}
        <Section title={t("settings.about")}>
          <div className="h-10 flex items-center text-[var(--font-size-md)] text-[var(--color-text-secondary)]">
            {t("settings.version")}
          </div>
          <div className="h-10 flex items-center text-[var(--font-size-md)] text-[var(--color-text-secondary)]">
            {t("settings.author")}
          </div>
          <a
            href="https://github.com/wh759705-creator/VaultX"
            target="_blank"
            rel="noopener noreferrer"
            className="h-10 flex items-center text-[var(--font-size-md)] text-[var(--color-primary)] hover:underline"
          >
            {t("settings.github")}
          </a>
          <a
            href="https://skillnav.dev"
            target="_blank"
            rel="noopener noreferrer"
            className="h-10 flex items-center text-[var(--font-size-md)] text-[var(--color-primary)] hover:underline"
          >
            {t("settings.skillnav")}
          </a>
        </Section>
      </div>
    </div>
  );
}

// -- Sub-components --

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h3 className="text-[var(--font-size-xs)] font-[var(--font-weight-medium)] text-[var(--color-text-tertiary)] uppercase mb-[var(--spacing-xs)]">
        {title}
      </h3>
      <div className="flex flex-col">{children}</div>
    </div>
  );
}

function SwitchRow({
  label,
  checked,
  onChange,
  disabled,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <div className="h-10 flex items-center justify-between">
      <span className={`text-[var(--font-size-md)] ${disabled ? "text-[var(--color-text-tertiary)]" : "text-[var(--color-text-primary)]"}`}>
        {label}
      </span>
      <button
        onClick={() => !disabled && onChange(!checked)}
        disabled={disabled}
        className={`w-10 h-[22px] rounded-full relative transition-colors cursor-pointer ${
          disabled ? "opacity-40 cursor-not-allowed" : ""
        } ${checked ? "bg-[var(--color-primary)]" : "bg-[var(--color-bg-elevated)]"}`}
      >
        <span
          className={`absolute top-[3px] w-4 h-4 rounded-full bg-white transition-transform ${
            checked ? "left-[22px]" : "left-[3px]"
          }`}
        />
      </button>
    </div>
  );
}

function SelectRow<T extends string | number>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
}) {
  return (
    <div className="h-10 flex items-center justify-between">
      <span className="text-[var(--font-size-md)] text-[var(--color-text-primary)]">{label}</span>
      <select
        value={value}
        onChange={(e) => {
          const raw = e.target.value;
          const parsed = typeof value === "number" ? (Number(raw) as T) : (raw as T);
          onChange(parsed);
        }}
        className="bg-[var(--color-bg-elevated)] text-[var(--color-text-primary)] text-[var(--font-size-sm)] px-[var(--spacing-sm)] py-[var(--spacing-xs)] rounded-[var(--radius-md)] border border-[var(--color-border)] cursor-pointer outline-none"
      >
        {options.map((opt) => (
          <option key={String(opt.value)} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}

function ShortcutRow({ label, shortcut }: { label: string; shortcut: string }) {
  return (
    <div className="h-10 flex items-center justify-between">
      <span className="text-[var(--font-size-md)] text-[var(--color-text-primary)]">{label}</span>
      <kbd className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)] bg-[var(--color-bg-elevated)] px-[var(--spacing-sm)] py-[2px] rounded-[var(--radius-sm)] border border-[var(--color-border)]">
        {shortcut}
      </kbd>
    </div>
  );
}

function ActionRow({ label, disabled }: { label: string; disabled?: boolean }) {
  return (
    <div className="h-10 flex items-center justify-between">
      <span className={`text-[var(--font-size-md)] ${disabled ? "text-[var(--color-text-tertiary)]" : "text-[var(--color-text-primary)]"}`}>
        {label}
      </span>
      <span className="text-[var(--color-text-tertiary)]">&rsaquo;</span>
    </div>
  );
}
