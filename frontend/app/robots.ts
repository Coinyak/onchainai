import type { MetadataRoute } from "next";
import { SITE_ORIGIN } from "@/lib/site";

/**
 * Crawl policy for Edge-request budget:
 * - Keep Googlebot (Search) on the catalog for SEO.
 * - Disallow private UX + API rewrites for polite bots (scrapers still need WAF).
 * - Soften non-search Google / AI training crawlers that inflated Edge usage.
 */
export default function robots(): MetadataRoute.Robots {
  // Both `/admin` and `/admin/` so exact + nested paths match reliably.
  const privatePaths = [
    "/admin",
    "/admin/",
    "/dashboard",
    "/dashboard/",
    "/toolkit",
    "/toolkit/",
    "/blueprints",
    "/blueprints/",
    "/api",
    "/api/",
    "/auth",
    "/auth/",
    "/onboarding",
    "/onboarding/",
    "/mcp", // machine-only; no SEO value (agents use CONNECT/docs)
    "/mcp/",
  ];

  const blockEntireSite = [
    "GoogleOther",
    "Google-Extended",
    "GPTBot",
    "CCBot",
    "Bytespider",
    "ClaudeBot",
    "anthropic-ai",
    "PerplexityBot",
    "Amazonbot",
    "Applebot-Extended",
    "meta-externalagent",
    "Diffbot",
  ];

  return {
    rules: [
      {
        userAgent: "*",
        allow: "/",
        disallow: privatePaths,
      },
      // Non-search / training crawlers. Googlebot (Search) stays on catalog.
      ...blockEntireSite.map((userAgent) => ({
        userAgent,
        disallow: ["/"] as string[],
      })),
    ],
    sitemap: `${SITE_ORIGIN}/sitemap.xml`,
  };
}