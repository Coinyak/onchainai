import Image from "next/image";
import Link from "next/link";

interface SidebarBrandProps {
  collapsed?: boolean;
}

/** Brand block in sidebar — uses sidebar-brand class for smoke tests. */
export function SidebarBrand({ collapsed = false }: SidebarBrandProps) {
  return (
    <div
      className={collapsed ? "sidebar-brand sidebar-brand-icon-only" : "sidebar-brand"}
      data-testid="sidebar-brand"
    >
      <Link
        href="/"
        className="sidebar-brand-logo no-underline text-primary"
        aria-label="OnchainAI home"
      >
        <Image
          className="sidebar-brand-mark"
          src="/brand/onchainai-logo.png"
          alt=""
          width={28}
          height={28}
        />
        <span className="sidebar-brand-text">OnchainAI</span>
      </Link>
    </div>
  );
}