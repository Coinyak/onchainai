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
  ];

  return {
    rules: [
      {
        userAgent: "*",
        allow: "/",
        disallow: privatePaths,
      },
      // Non-Search Google crawlers (metrics: GoogleOther ≈ 1M req / 7d).
      // Search indexing still uses Googlebot (allowed above).
      {
        userAgent: "GoogleOther",
        disallow: ["/"],
      },
      {
        userAgent: "Google-Extended",
        disallow: ["/"],
      },
      {
        userAgent: "GPTBot",
        disallow: ["/"],
      },
      {
        userAgent: "CCBot",
        disallow: ["/"],
      },
      {
        userAgent: "Bytespider",
        disallow: ["/"],
      },
    ],
    sitemap: `${SITE_ORIGIN}/sitemap.xml`,
  };
}