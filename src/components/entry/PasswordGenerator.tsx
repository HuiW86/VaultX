import { useState, useEffect, useCallback } from "react";
import { ChevronDown, ChevronUp, RefreshCw, Copy, Check } from "lucide-react";
import { Button } from "../ui/Button";
import { StrengthMeter } from "../ui/StrengthMeter";
import { api } from "../../lib/commands";
import { useTranslation } from "../../i18n";

interface PasswordGeneratorProps {
  onUse: (password: string) => void;
}

type Mode = "random" | "words";

export function PasswordGenerator({ onUse }: PasswordGeneratorProps) {
  const { t } = useTranslation();

  const separatorOptions = [
    { value: "-", label: t("generator.separator_hyphen") },
    { value: ".", label: t("generator.separator_dot") },
    { value: " ", label: t("generator.separator_space") },
    { value: "_", label: t("generator.separator_underscore") },
    { value: "", label: t("generator.separator_none") },
  ];

  const [open, setOpen] = useState(false);
  const [mode, setMode] = useState<Mode>("random");
  const [length, setLength] = useState(20);
  const [uppercase, setUppercase] = useState(true);
  const [lowercase, setLowercase] = useState(true);
  const [digits, setDigits] = useState(true);
  const [symbols, setSymbols] = useState(true);
  const [wordCount, setWordCount] = useState(5);
  const [separator, setSeparator] = useState("-");
  const [password, setPassword] = useState("");
  const [copied, setCopied] = useState(false);
  const [history, setHistory] = useState<string[]>([]);

  const generate = useCallback(async () => {
    const params =
      mode === "random"
        ? { length, uppercase, lowercase, digits, symbols, mode: mode as "random" }
        : { mode: mode as "words", word_count: wordCount, separator };
    const result = await api.generatePassword(params);
    setPassword(result.password);
    setHistory((prev) => [result.password, ...prev.filter((p) => p !== result.password)].slice(0, 20));
  }, [mode, length, uppercase, lowercase, digits, symbols, wordCount, separator]);

  // Generate on first open and when params change
  useEffect(() => {
    if (open) generate();
  }, [open, mode, length, uppercase, lowercase, digits, symbols, wordCount, separator]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleCopy = useCallback(async () => {
    await api.copyToClipboard(password);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [password]);

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="flex items-center gap-[var(--spacing-xs)] text-[var(--font-size-sm)] text-[var(--color-primary)] hover:underline cursor-pointer"
      >
        <ChevronDown size={14} />
        {t("generator.title")}
      </button>
    );
  }

  return (
    <div className="border border-[var(--color-border)] rounded-[var(--radius-lg)] overflow-hidden">
      {/* Header */}
      <button
        type="button"
        onClick={() => setOpen(false)}
        className="w-full flex items-center justify-between px-[var(--spacing-md)] py-[var(--spacing-sm)] bg-[var(--color-bg-elevated)] text-[var(--font-size-sm)] text-[var(--color-primary)] cursor-pointer"
      >
        <span>{t("generator.title")}</span>
        <ChevronUp size={14} />
      </button>

      <div className="px-[var(--spacing-md)] py-[var(--spacing-md)] space-y-[var(--spacing-md)]">
        {/* Mode toggle */}
        <div className="flex gap-[var(--spacing-xs)]">
          {(["random", "words"] as const).map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => setMode(m)}
              className={`px-[var(--spacing-md)] py-[var(--spacing-xs)] rounded-[var(--radius-md)] text-[var(--font-size-sm)] cursor-pointer transition-colors ${
                mode === m
                  ? "bg-[var(--color-primary-bg)] text-[var(--color-primary)]"
                  : "text-[var(--color-text-secondary)] hover:bg-[var(--color-bg-hover)]"
              }`}
            >
              {m === "random" ? t("generator.random") : t("generator.words")}
            </button>
          ))}
        </div>

        {mode === "random" ? (
          <>
            {/* Length slider */}
            <div className="flex items-center gap-[var(--spacing-md)]">
              <span className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)] w-14">
                {t("generator.length")}
              </span>
              <input
                type="range"
                min={8}
                max={128}
                value={length}
                onChange={(e) => setLength(Number(e.target.value))}
                className="flex-1 accent-[var(--color-primary)]"
              />
              <input
                type="number"
                min={8}
                max={128}
                value={length}
                onChange={(e) => setLength(Math.min(128, Math.max(8, Number(e.target.value))))}
                className="w-14 h-7 text-center bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-sm)] text-[var(--font-size-sm)] text-[var(--color-text-primary)] outline-none"
              />
            </div>

            {/* Charset checkboxes */}
            <div className="grid grid-cols-2 gap-[var(--spacing-sm)]">
              <Checkbox label={t("generator.uppercase")} checked={uppercase} onChange={setUppercase} />
              <Checkbox label={t("generator.lowercase")} checked={lowercase} onChange={setLowercase} />
              <Checkbox label={t("generator.digits")} checked={digits} onChange={setDigits} />
              <Checkbox label={t("generator.symbols")} checked={symbols} onChange={setSymbols} />
            </div>
          </>
        ) : (
          <>
            {/* Word count */}
            <div className="flex items-center gap-[var(--spacing-md)]">
              <span className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)] w-14">
                {t("generator.word_count")}
              </span>
              <input
                type="range"
                min={3}
                max={10}
                value={wordCount}
                onChange={(e) => setWordCount(Number(e.target.value))}
                className="flex-1 accent-[var(--color-primary)]"
              />
              <span className="w-14 text-center text-[var(--font-size-sm)] text-[var(--color-text-primary)]">
                {wordCount}
              </span>
            </div>

            {/* Separator */}
            <div className="flex items-center gap-[var(--spacing-md)]">
              <span className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)] w-14">
                {t("generator.separator")}
              </span>
              <select
                value={separator}
                onChange={(e) => setSeparator(e.target.value)}
                className="bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-sm)] text-[var(--font-size-sm)] text-[var(--color-text-primary)] px-[var(--spacing-sm)] py-[var(--spacing-xs)] outline-none cursor-pointer"
              >
                {separatorOptions.map((o) => (
                  <option key={o.label} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
            </div>
          </>
        )}

        {/* Live preview */}
        <div className="bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] px-[var(--spacing-md)] py-[var(--spacing-sm)]">
          <p
            className="text-[var(--font-size-md)] text-[var(--color-text-primary)] break-all select-all"
            style={{ fontFamily: "var(--font-mono)" }}
          >
            {password || "\u00A0"}
          </p>
        </div>

        <StrengthMeter password={password} />

        {/* Action buttons */}
        <div className="flex gap-[var(--spacing-sm)]">
          <Button
            type="button"
            variant="primary"
            size="sm"
            onClick={() => {
              onUse(password);
              setOpen(false);
            }}
          >
            {t("generator.use")}
          </Button>
          <Button type="button" variant="ghost" size="sm" onClick={generate}>
            <RefreshCw size={14} />
            {t("generator.refresh")}
          </Button>
          <Button type="button" variant="ghost" size="sm" onClick={handleCopy}>
            {copied ? <Check size={14} /> : <Copy size={14} />}
            {copied ? t("generator.copied") : t("generator.copy")}
          </Button>
        </div>

        {/* History */}
        {history.length > 1 && (
          <div>
            <p className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)] mb-[var(--spacing-xs)]">
              {t("generator.recent", { count: history.length - 1 })}
            </p>
            <div className="max-h-24 overflow-y-auto space-y-px">
              {history.slice(1, 6).map((h, i) => (
                <button
                  key={i}
                  type="button"
                  onClick={() => onUse(h)}
                  className="w-full text-left text-[var(--font-size-xs)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] truncate cursor-pointer px-[var(--spacing-xs)] py-px rounded-[var(--radius-sm)] hover:bg-[var(--color-bg-hover)]"
                  style={{ fontFamily: "var(--font-mono)" }}
                >
                  {h}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function Checkbox({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex items-center gap-[var(--spacing-xs)] text-[var(--font-size-sm)] text-[var(--color-text-primary)] cursor-pointer">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="accent-[var(--color-primary)]"
      />
      {label}
    </label>
  );
}
