import { safeReturnTo } from "./return-to-guard.mjs";

export { safeReturnTo };

const STORAGE_KEY = "onchain-ai-return-to";
const INTENT_KEY = "onchain-ai-return-to-intent";

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