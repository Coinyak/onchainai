import type { PublicTool } from "@/lib/api";
import { SITE_ORIGIN } from "@/lib/site";

/** Safe for dangerouslySetInnerHTML in application/ld+json script tags. */
export function serializeJsonLd(payload: Record<string, unknown>): string {
  return JSON.stringify(payload).replace(/</g, "\\u003c");
}

export function buildSoftwareApplicationJsonLd(tool: PublicTool): Record<string, unknown> {
  const pageUrl = `${SITE_ORIGIN}/tools/${tool.slug}`;
  const image = tool.logo_url?.startsWith("http")
    ? tool.logo_url
    : tool.logo_url
      ? `${SITE_ORIGIN}${tool.logo_url.startsWith("/") ? "" : "/"}${tool.logo_url}`
      : `${SITE_ORIGIN}/brand/onchainai-logo.png`;

  const jsonLd: Record<string, unknown> = {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: tool.name,
    description: tool.description || `${tool.name} — ${tool.type} tool on OnchainAI`,
    applicationCategory: tool.function || "DeveloperApplication",
    operatingSystem: "Web",
    url: pageUrl,
    image,
  };

  if (tool.repo_url) {
    jsonLd.codeRepository = tool.repo_url;
  }

  if (tool.homepage) {
    jsonLd.sameAs = [tool.homepage];
  }

  if (tool.license) {
    jsonLd.license = tool.license;
  }

  if (tool.pricing && tool.pricing !== "unknown") {
    const offers: Record<string, unknown> = {
      "@type": "Offer",
      priceCurrency: "USD",
      category: tool.pricing,
    };
    if (tool.pricing === "free") {
      offers.price = "0";
    }
    jsonLd.offers = offers;
  }

  return jsonLd;
}