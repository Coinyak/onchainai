/** Canonical production origin for SEO metadata, sitemap, and JSON-LD. */
export const SITE_ORIGIN =
  process.env.NEXT_PUBLIC_SITE_URL?.replace(/\/$/, "") ||
  "https://www.onchain-ai.xyz";

export const DEFAULT_OG_IMAGE_PATH = "/og-default.png";

export const SEO_REVALIDATE_SECONDS = 120;