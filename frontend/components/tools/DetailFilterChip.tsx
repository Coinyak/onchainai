import Link from "next/link";

interface DetailFilterChipProps {
  href: string;
  label: string;
  children?: React.ReactNode;
}

export function DetailFilterChip({ href, label, children }: DetailFilterChipProps) {
  return (
    <Link href={href} scroll={false} className="detail-filter-chip no-underline">
      {children}
      <span>{label}</span>
    </Link>
  );
}