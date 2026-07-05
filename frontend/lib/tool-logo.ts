/** GitHub org avatar URLs without a custom upload often render the generic gray octocat. */
export function isGithubOrgAvatarUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    if (parsed.hostname !== "avatars.githubusercontent.com") return false;
    const segment = parsed.pathname.replace(/^\/+/, "").split("/")[0] ?? "";
    return segment.length > 0 && segment !== "u" && !segment.startsWith("u/");
  } catch {
    return false;
  }
}

/** Prefer monogram over generic GitHub placeholders for trusted catalog entries. */
export function shouldPreferMonogramOverLogo(
  logoUrl: string | null | undefined,
  status?: string | null,
): boolean {
  if (!logoUrl || !status) return false;
  if (status !== "official" && status !== "verified") return false;
  return isGithubOrgAvatarUrl(logoUrl);
}