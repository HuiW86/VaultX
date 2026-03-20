import { useState, useEffect, useRef } from "react";
import zxcvbn from "zxcvbn";

interface StrengthMeterProps {
  password: string;
}

const labels = ["Very weak", "Weak", "Fair", "Strong", "Very strong"];
const colors = [
  "var(--color-error)",
  "var(--color-error)",
  "var(--color-warning)",
  "var(--color-success)",
  "var(--color-success)",
];
const widths = ["10%", "25%", "50%", "75%", "100%"];

export function StrengthMeter({ password }: StrengthMeterProps) {
  const [score, setScore] = useState(0);
  const timerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    if (!password) {
      setScore(0);
      return;
    }
    // 150ms debounce per DS:§4.5 + performance review
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      setScore(zxcvbn(password).score);
    }, 150);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [password]);

  if (!password) return null;

  return (
    <div className="flex items-center gap-[var(--spacing-sm)]">
      <div
        className="flex-1 h-1 rounded-full bg-[var(--color-border)] overflow-hidden"
        role="meter"
        aria-valuenow={score}
        aria-valuemin={0}
        aria-valuemax={4}
        aria-label="Password strength"
      >
        <div
          className="h-full rounded-full transition-all duration-[var(--duration-normal)]"
          style={{
            width: widths[score],
            backgroundColor: colors[score],
          }}
        />
      </div>
      <span
        className="text-[var(--font-size-xs)] whitespace-nowrap"
        style={{ color: colors[score] }}
      >
        {labels[score]}
      </span>
    </div>
  );
}
