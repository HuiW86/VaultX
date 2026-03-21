import { AlertCircle } from "lucide-react";
import { Button } from "./Button";
import { useTranslation } from "../../i18n";

interface ErrorStateProps {
  title?: string;
  message: string;
  onRetry?: () => void;
}

export function ErrorState({
  title,
  message,
  onRetry,
}: ErrorStateProps) {
  const { t } = useTranslation();
  const resolvedTitle = title ?? t("error.something_wrong");
  return (
    <div className="flex flex-col items-center justify-center h-full text-center px-[var(--spacing-xl)]">
      <AlertCircle
        size={48}
        className="text-[var(--color-error)] mb-[var(--spacing-lg)]"
      />
      <h3 className="text-[var(--font-size-lg)] font-[var(--font-weight-medium)] text-[var(--color-text-primary)] mb-[var(--spacing-xs)]">
        {resolvedTitle}
      </h3>
      <p className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)] mb-[var(--spacing-lg)]">
        {message}
      </p>
      {onRetry && (
        <Button variant="secondary" onClick={onRetry}>
          {t("error.retry")}
        </Button>
      )}
    </div>
  );
}
