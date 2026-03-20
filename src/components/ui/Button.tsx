import { type ButtonHTMLAttributes, forwardRef } from "react";
import { Loader2 } from "lucide-react";

type Variant = "primary" | "secondary" | "ghost" | "danger" | "text";
type Size = "lg" | "md" | "sm";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  loading?: boolean;
}

const variantStyles: Record<Variant, string> = {
  primary:
    "bg-[var(--color-primary)] text-[var(--color-text-inverse)] hover:bg-[var(--color-primary-hover)] active:bg-[var(--color-primary-active)]",
  secondary:
    "bg-transparent text-[var(--color-primary)] border border-[var(--color-primary)] hover:bg-[var(--color-primary-bg)]",
  ghost:
    "bg-transparent text-[var(--color-text-primary)] border border-[var(--color-border)] hover:bg-[var(--color-bg-hover)]",
  danger:
    "bg-[var(--color-error)] text-[var(--color-text-inverse)] hover:opacity-90 active:opacity-80",
  text: "bg-transparent text-[var(--color-primary)] hover:underline",
};

const sizeStyles: Record<Size, string> = {
  lg: "h-10 px-6 text-[var(--font-size-md)]",
  md: "h-8 px-4 text-[var(--font-size-sm)]",
  sm: "h-7 px-3 text-[var(--font-size-xs)]",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = "primary", size = "md", loading, disabled, children, className = "", ...props }, ref) => {
    return (
      <button
        ref={ref}
        disabled={disabled || loading}
        className={`inline-flex items-center justify-center gap-[var(--spacing-xs)] rounded-[var(--radius-md)] font-[var(--font-weight-medium)] transition-all duration-[var(--duration-fast)] cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed ${variantStyles[variant]} ${sizeStyles[size]} ${className}`}
        {...props}
      >
        {loading && <Loader2 size={14} className="animate-spin" />}
        {children}
      </button>
    );
  }
);

Button.displayName = "Button";
