/**
 * GitHub orgs confirmed (prod walkthrough LG1) to serve the shared generic octocat
 * placeholder avatar (ETag 2ae73e12…). Custom org avatars (e.g. rainbow-me, wevm)
 * are not listed here.
 */
const KNOWN_GENERIC_GITHUB_ORG_SLUGS = new Set([
  "smartcontractkit",
  "circle-fin",
  "reown-com",
  "web3-mcp-hub",
]);

/** Extract GitHub org slug from `https://avatars.githubusercontent.com/{org}`. */
export function githubOrgSlugFromAvatarUrl(url: string): string | null {
  try {
    const parsed = new URL(url);
    if (parsed.hostname !== "avatars.githubusercontent.com") return null;
    const segment = parsed.pathname.replace(/^\/+/, "").split("/")[0] ?? "";
    if (!segment || segment === "u" || segment.startsWith("u/")) return null;
    return segment.toLowerCase();
  } catch {
    return null;
  }
}

/** True when the URL is a GitHub org avatar known to render the generic placeholder. */
export function isKnownGenericGithubOrgAvatar(url: string): boolean {
  const slug = githubOrgSlugFromAvatarUrl(url);
  return slug !== null && KNOWN_GENERIC_GITHUB_ORG_SLUGS.has(slug);
}

/** Prefer monogram over generic GitHub placeholders for trusted catalog entries. */
export function shouldPreferMonogramOverLogo(
  logoUrl: string | null | undefined,
  status?: string | null,
): boolean {
  if (!logoUrl || !status) return false;
  if (status !== "official" && status !== "verified") return false;
  return isKnownGenericGithubOrgAvatar(logoUrl);
}