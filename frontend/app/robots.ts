import type { MetadataRoute } from "next";
import { SITE_ORIGIN } from "@/lib/site";

/**
 * Crawl policy for Edge-request budget:
 * - Keep Googlebot (Search) on the catalog for SEO.
 * - Disallow private UX + API rewrites for polite bots (scrapers still need WAF).
 * - Soften non-search Google / AI training crawlers that inflated Edge usage.
 */
export default function robots(): MetadataRoute.Robots {
  const privatePaths = [
    "/admin/",
    "/dashboard/",
    "/toolkit/",
    "/blueprints/",
    "/api/",
    "/auth/",
    "/onboarding/",
    "/mcp", // machine-only; no SEO value (agents use CONNECT/docs)
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