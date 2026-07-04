/**
 * Same-origin path (+ optional query) only — blocks open redirects.
 * @param {string | null | undefined} raw
 * @returns {string | null}
 */
export function safeReturnTo(raw) {
  if (!raw?.trim()) return null;
  const path = raw.trim();
  if (!path.startsWith("/") || path.startsWith("//")) return null;
  if (path.includes("\\")) return null;
  const [pathname] = path.split(/[?#]/, 1);
  if (!pathname || pathname.includes(":")) return null;
  return path;
}
