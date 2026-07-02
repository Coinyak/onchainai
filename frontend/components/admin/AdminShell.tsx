"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { SidebarBrand } from "@/components/layout/SidebarBrand";

const ADMIN_NAV = [
  { href: "/admin", label: "Dashboard" },
  { href: "/admin/tools", label: "Tools" },
  { href: "/admin/comments", label: "Comments" },
  { href: "/admin/users", label: "Users" },
  { href: "/admin/categories", label: "Categories" },
  { href: "/admin/crawler", label: "Crawler" },
  { href: "/admin/featured", label: "Featured" },
  { href: "/admin/settings", label: "Settings" },
];

function isActive(pathname: string, href: string): boolean {
  const path = pathname.split(/[?#]/)[0].replace(/\/$/, "") || "/";
  if (href === "/admin") return path === "/admin";
  return path === href || path.startsWith(`${href}/`);
}

interface AdminShellProps {
  children: React.ReactNode;
}

export function AdminShell({ children }: AdminShellProps) {
  const pathname = usePathname();

  return (
    <div className="site-layout">
      <aside className="tools-sidebar site-sidebar-chrome">
        <SidebarBrand />
        <nav className="admin-nav" aria-label="Admin navigation">
          <div className="admin-nav-heading">Admin</div>
          <ul className="sidebar-list admin-nav-list">
            {ADMIN_NAV.map((item) => (
              <li key={item.href}>
                <Link
                  href={item.href}
                  className={isActive(pathname, item.href) ? "sidebar-link active" : "sidebar-link"}
                >
                  <span className="sidebar-title-text">{item.label}</span>
                </Link>
              </li>
            ))}
          </ul>
        </nav>
      </aside>
      <main className="site-main">{children}</main>
    </div>
  );
}