interface SkeletonProps {
  className?: string;
}

export function Skeleton({ className = "" }: SkeletonProps) {
  return (
    <div
      className={`animate-pulse rounded-[var(--radius-md)] bg-[var(--color-bg-hover)] ${className}`}
    />
  );
}

export function EntryCardSkeleton() {
  return (
    <div className="flex items-center gap-[var(--spacing-sm)] px-[var(--spacing-md)] py-[var(--spacing-sm)] h-[60px]">
      <Skeleton className="w-8 h-8 rounded-full shrink-0" />
      <div className="flex-1 flex flex-col gap-1">
        <Skeleton className="h-3.5 w-32" />
        <Skeleton className="h-3 w-24" />
      </div>
    </div>
  );
}

export function DetailSkeleton() {
  return (
    <div className="p-[var(--spacing-2xl)] flex flex-col gap-[var(--spacing-xl)]">
      <Skeleton className="h-6 w-48" />
      <Skeleton className="h-4 w-32" />
      <div className="flex flex-col gap-[var(--spacing-lg)]">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="flex flex-col gap-1">
            <Skeleton className="h-3 w-16" />
            <Skeleton className="h-9 w-full" />
          </div>
        ))}
      </div>
    </div>
  );
}
