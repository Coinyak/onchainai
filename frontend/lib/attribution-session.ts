const STORAGE_KEY = "onchainai_attribution_session";

/** Stable anonymous session id for install-guide attribution dedup (X4). */
export function getAttributionSession(): string {
  if (typeof window === "undefined") return "anonymous";
  try {
    const existing = window.localStorage.getItem(STORAGE_KEY)?.trim();
    if (existing) return existing;
    const created =
      typeof crypto !== "undefined" && "randomUUID" in crypto
        ? crypto.randomUUID()
        : `sess-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    window.localStorage.setItem(STORAGE_KEY, created);
    return created;
  } catch {
    return "anonymous";
  }
}