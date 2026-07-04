import Link from "next/link";

interface EmptyStateProps {
  message?: string;
  filterLines?: string[];
  clearHref?: string;
}

export function EmptyState({
  message = "No tools match your filters.",
  filterLines = [],
  clearHref = "",
}: EmptyStateProps) {
  const hasFilters = filterLines.length > 0;
  const showClear = hasFilters && clearHref;

  return (
    <div className="empty-state-panel">
      <p className="empty-state-message">{message}</p>
      {hasFilters && (
        <div className="empty-state-filters" aria-label="Active filters">
          <p className="empty-state-filters-heading">Current filters</p>
          <ul className="empty-state-filter-list">
            {filterLines.map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
        </div>
      )}
      <p className="empty-state-hint">
        Try a different keyword, suggest a tool for operator review, or clear your filters.
      </p>
      <div className="empty-state-actions">
        {showClear && (
          <Link
            href={clearHref}
            scroll={false}
            className="empty-state-clear-btn"
            data-testid="empty-state-clear-filters"
          >
            Clear filters
          </Link>
        )}
        <Link href="/submit" className="empty-state-submit-btn">
          Suggest a tool
        </Link>
      </div>
    </div>
  );
}