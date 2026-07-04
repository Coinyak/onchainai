const STORAGE_KEY = "onchain-ai-return-to";
const INTENT_KEY = "onchain-ai-return-to-intent";

/** Same-origin path (+ optional query) only — blocks open redirects. */
export function safeReturnTo(raw: string | null | undefined): string | null {
  if (!raw?.trim()) return null;
  const path = raw.trim();
  if (!path.startsWith("/") || path.startsWith("//")) return null;
  const [pathname] = path.split(/[?#]/, 1);
  if (!pathname || pathname.includes(":")) return null;
  return path;
}

export function persistReturnTo(
  path: string | null,
  options?: { fromLoginPage?: boolean },
): void {
  if (typeof window === "undefined") return;
  if (path) {
    sessionStorage.setItem(STORAGE_KEY, path);
    if (options?.fromLoginPage) sessionStorage.setItem(INTENT_KEY, "1");
  } else {
    sessionStorage.removeItem(STORAGE_KEY);
    sessionStorage.removeItem(INTENT_KEY);
  }
}

export function peekReturnTo(): string | null {
  if (typeof window === "undefined") return null;
  if (sessionStorage.getItem(INTENT_KEY) !== "1") return null;
  return safeReturnTo(sessionStorage.getItem(STORAGE_KEY));
}

export function consumeReturnTo(): string | null {
  const path = peekReturnTo();
  if (typeof window !== "undefined") {
    sessionStorage.removeItem(STORAGE_KEY);
    sessionStorage.removeItem(INTENT_KEY);
  }
  return path;
}