/** Matches FeaturedCarousel render filter — only http(s) URLs appear on home. */
export function isRenderableFeaturedImageUrl(url: string): boolean {
  const trimmed = url.trim();
  return trimmed.startsWith("http://") || trimmed.startsWith("https://");
}