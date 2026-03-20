import { useState, useCallback } from "react";
import { Copy, Check } from "lucide-react";
import { api } from "../../lib/commands";

interface CopyButtonProps {
  value: string;
  label?: string;
}

export function CopyButton({ value, label = "Copy" }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await api.copyToClipboard(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Error handled by Rust-side event → Toast
    }
  }, [value]);

  return (
    <button
      type="button"
      onClick={handleCopy}
      className="p-1.5 text-[var(--color-text-tertiary)] hover:text-[var(--color-primary)] rounded transition-colors duration-[var(--duration-fast)]"
      aria-label={label}
    >
      {copied ? (
        <Check size={16} className="text-[var(--color-success)]" />
      ) : (
        <Copy size={16} />
      )}
    </button>
  );
}
