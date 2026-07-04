import Link from "next/link";

interface FilterChipProps {
  href: string;
  label: string;
  active: boolean;
  count?: number;
  onNavigate?: () => void;
}

export function FilterChip({ href, label, active, count, onNavigate }: FilterChipProps) {
  const disabled = count === 0 && !active;
  const className = disabled
    ? "sidebar-link sidebar-link--disabled"
    : active
      ? "sidebar-link active"
      : "sidebar-link";
  const chipContent = (
    <>
      <span className="sidebar-title-text">{label}</span>
      {count !== undefined && <span className="sidebar-count">{count}</span>}
    </>
  );

  if (disabled) {
    return (
      <li>
        <span className={className} aria-disabled="true">
          {chipContent}
        </span>
      </li>
    );
  }

  return (
    <li>
      <Link href={href} scroll={false} className={className} onClick={onNavigate}>
        {chipContent}
      </Link>
    </li>
  );
}