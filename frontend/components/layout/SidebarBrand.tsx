import Link from "next/link";

/** Brand block in sidebar — uses sidebar-brand class for smoke tests. */
export function SidebarBrand() {
  return (
    <div className="sidebar-brand" data-testid="sidebar-brand">
      <Link href="/" className="sidebar-brand-logo no-underline text-primary">
        OnchainAI
      </Link>
    </div>
  );
}