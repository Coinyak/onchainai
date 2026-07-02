export function ToolCardSkeleton() {
  return (
    <article className="tool-card skeleton-card" aria-hidden="true">
      <div className="tool-card-inner">
        <div className="tool-logo skeleton-block" />
        <div className="tool-card-body">
          <div className="skeleton-line skeleton-title" />
          <div className="skeleton-line skeleton-desc" />
          <div className="skeleton-line skeleton-meta" />
        </div>
      </div>
    </article>
  );
}

export function ToolListSkeleton({ count = 6 }: { count?: number }) {
  return (
    <div className="tool-list">
      {Array.from({ length: count }, (_, i) => (
        <ToolCardSkeleton key={i} />
      ))}
    </div>
  );
}