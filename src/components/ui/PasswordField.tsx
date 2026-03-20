import { useState, useRef, useCallback, type InputHTMLAttributes } from "react";
import { Eye, EyeOff, Copy, Check } from "lucide-react";
import { api } from "../../lib/commands";

interface PasswordFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange"> {
  value: string;
  onChange?: (value: string) => void;
  label?: string;
  error?: string;
  readOnly?: boolean;
  showCopy?: boolean;
}

export function PasswordField({
  value,
  onChange,
  label,
  error,
  readOnly = false,
  showCopy = false,
  ...props
}: PasswordFieldProps) {
  const [revealed, setRevealed] = useState(false);
  const [copied, setCopied] = useState(false);
  const autoMaskTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  const toggleReveal = useCallback(() => {
    const next = !revealed;
    setRevealed(next);
    if (autoMaskTimer.current) clearTimeout(autoMaskTimer.current);
    if (next) {
      autoMaskTimer.current = setTimeout(() => setRevealed(false), 30000);
    }
  }, [revealed]);

  const handleCopy = useCallback(async () => {
    try {
      await api.copyToClipboard(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Toast handled by Rust event
    }
  }, [value]);

  return (
    <div className="flex flex-col gap-[var(--spacing-xs)]">
      {label && (
        <label className="text-[var(--font-size-xs)] font-[var(--font-weight-medium)] text-[var(--color-text-secondary)]">
          {label}
        </label>
      )}
      <div className="relative">
        <input
          type={revealed ? "text" : "password"}
          value={value}
          onChange={onChange ? (e) => onChange(e.target.value) : undefined}
          readOnly={readOnly}
          aria-label={label ? `${label} (${revealed ? "visible" : "hidden"})` : undefined}
          className={`w-full h-10 pl-[var(--spacing-md)] pr-20 bg-[var(--color-bg-input)] border rounded-[var(--radius-md)] font-[var(--font-mono)] text-[var(--font-size-lg)] text-[var(--color-text-primary)] outline-none transition-colors duration-[var(--duration-fast)] ${
            error
              ? "border-[var(--color-error)]"
              : "border-[var(--color-border)] focus:border-[var(--color-primary)]"
          }`}
          {...props}
        />
        <div className="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-1">
          <button
            type="button"
            onClick={toggleReveal}
            className="p-1.5 text-[var(--color-text-tertiary)] hover:text-[var(--color-text-primary)] rounded transition-colors"
            aria-label={revealed ? "Hide password" : "Show password"}
          >
            {revealed ? <EyeOff size={16} /> : <Eye size={16} />}
          </button>
          {showCopy && (
            <button
              type="button"
              onClick={handleCopy}
              className="p-1.5 text-[var(--color-text-tertiary)] hover:text-[var(--color-primary)] rounded transition-colors"
              aria-label="Copy password"
            >
              {copied ? (
                <Check size={16} className="text-[var(--color-success)]" />
              ) : (
                <Copy size={16} />
              )}
            </button>
          )}
        </div>
      </div>
      {error && (
        <p role="alert" className="text-[var(--font-size-xs)] text-[var(--color-error)]">
          {error}
        </p>
      )}
    </div>
  );
}
