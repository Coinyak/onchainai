import Link from "next/link";

interface FilterChipProps {
  href: string;
  label: string;
  active: boolean;
  count?: number;
  onNavigate?: () => void;
}

export function FilterChip({ href, label, active, count, onNavigate }: FilterChipProps) {
  return (
    <li>
      <Link
        href={href}
        scroll={false}
        className={active ? "sidebar-link active" : "sidebar-link"}
        onClick={onNavigate}
      >
        <span className="sidebar-title-text">{label}</span>
        {count !== undefined && <span className="sidebar-count">{count}</span>}
      </Link>
    </li>
  );
}