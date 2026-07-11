/** Canonical production origin for SEO metadata, sitemap, and JSON-LD. */
export const SITE_ORIGIN =
  process.env.NEXT_PUBLIC_SITE_URL?.replace(/\/$/, "") ||
  "https://www.onchain-ai.xyz";

export const DEFAULT_OG_IMAGE_PATH = "/og-default.png";

/** ISR / server-fetch revalidate. Higher = fewer Edge regenerations under bot crawl. */
export const SEO_REVALIDATE_SECONDS = 300;