import { type InputHTMLAttributes, forwardRef } from "react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  required?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, error, required, className = "", id, ...props }, ref) => {
    const inputId = id || label?.toLowerCase().replace(/\s/g, "-");
    return (
      <div className="flex flex-col gap-[var(--spacing-xs)]">
        {label && (
          <label
            htmlFor={inputId}
            className="text-[var(--font-size-xs)] font-[var(--font-weight-medium)] text-[var(--color-text-secondary)]"
          >
            {label}
            {required && <span className="text-[var(--color-error)] ml-1">*</span>}
          </label>
        )}
        <input
          ref={ref}
          id={inputId}
          aria-required={required}
          aria-invalid={!!error}
          className={`h-9 px-[var(--spacing-md)] bg-[var(--color-bg-input)] border rounded-[var(--radius-md)] text-[var(--font-size-md)] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] outline-none transition-colors duration-[var(--duration-fast)] ${
            error
              ? "border-[var(--color-error)] bg-[var(--color-error-bg)]"
              : "border-[var(--color-border)] focus:border-[var(--color-primary)]"
          } ${className}`}
          {...props}
        />
        {error && (
          <p
            role="alert"
            className="text-[var(--font-size-xs)] text-[var(--color-error)]"
          >
            {error}
          </p>
        )}
      </div>
    );
  }
);

Input.displayName = "Input";
