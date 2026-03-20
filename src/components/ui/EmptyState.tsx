import { Button } from "./Button";

interface EmptyStateProps {
  title: string;
  description?: string;
  actionLabel?: string;
  onAction?: () => void;
}

export function EmptyState({ title, description, actionLabel, onAction }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center h-full text-center px-[var(--spacing-xl)]">
      <h3 className="text-[var(--font-size-lg)] font-[var(--font-weight-medium)] text-[var(--color-text-primary)] mb-[var(--spacing-xs)]">
        {title}
      </h3>
      {description && (
        <p className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)] mb-[var(--spacing-lg)]">
          {description}
        </p>
      )}
      {actionLabel && onAction && (
        <Button variant="secondary" size="md" onClick={onAction}>
          {actionLabel}
        </Button>
      )}
    </div>
  );
}
