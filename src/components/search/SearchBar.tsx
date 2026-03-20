import { useRef, useEffect } from "react";
import { Search, X } from "lucide-react";
import { useSearchStore } from "../../stores/searchStore";

interface SearchBarProps {
  inputRef?: React.RefObject<HTMLInputElement | null>;
}

export function SearchBar({ inputRef: externalRef }: SearchBarProps) {
  const localRef = useRef<HTMLInputElement>(null);
  const ref = externalRef || localRef;
  const query = useSearchStore((s) => s.query);
  const setQuery = useSearchStore((s) => s.setQuery);
  const clear = useSearchStore((s) => s.clear);

  // Focus on Cmd+K (handled by parent, ref passed down)
  useEffect(() => {
    return () => {
      // Clear search when component unmounts (e.g., lock)
      clear();
    };
  }, [clear]);

  return (
    <div className="relative px-[var(--spacing-sm)] py-[var(--spacing-sm)]">
      <div className="relative">
        <Search
          size={14}
          className="absolute left-[var(--spacing-sm)] top-1/2 -translate-y-1/2 text-[var(--color-text-tertiary)] pointer-events-none"
        />
        <input
          ref={ref}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              clear();
              (ref as React.RefObject<HTMLInputElement>).current?.blur();
            }
          }}
          placeholder="Search... (⌘K)"
          aria-label="Search entries"
          className="w-full h-8 pl-8 pr-8 bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-sm)] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] outline-none focus:border-[var(--color-primary)] transition-colors duration-[var(--duration-fast)]"
        />
        {query && (
          <button
            onClick={clear}
            className="absolute right-[var(--spacing-xs)] top-1/2 -translate-y-1/2 p-1 text-[var(--color-text-tertiary)] hover:text-[var(--color-text-primary)] rounded"
            aria-label="Clear search"
          >
            <X size={14} />
          </button>
        )}
      </div>
    </div>
  );
}
