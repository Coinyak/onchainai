"use client";

import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { getSiteSettings } from "@/lib/api";

export function SiteFooter() {
  const settingsQuery = useQuery({
    queryKey: ["site-settings"],
    queryFn: getSiteSettings,
    retry: false,
  });

  const links = settingsQuery.data?.footer_links ?? [];

  return (
    <footer className="site-footer border-t border-border mt-auto px-gutter md:px-6 py-6">
      <nav
        className="flex flex-wrap gap-x-6 gap-y-2 text-body-sm"
        aria-label="Footer"
      >
        <Link
          href="/llms.txt"
          className="text-secondary hover:text-primary no-underline"
          data-testid="footer-llms-txt"
        >
          llms.txt
        </Link>
        <Link
          href="/connect"
          className="text-secondary hover:text-primary no-underline"
          data-testid="footer-connect"
        >
          Connect MCP
        </Link>
        {links.map((link) => (
          <Link
            key={`${link.label}-${link.url}`}
            href={link.url}
            className="text-secondary hover:text-primary no-underline"
          >
            {link.label}
          </Link>
        ))}
      </nav>
    </footer>
  );
}