import { clientApiBase } from "@/lib/api";
import { SITE_ORIGIN } from "@/lib/site";

export { clientApiBase };

/** True on Vercel preview deployments (*.vercel.app). */
export function isVercelPreviewHost(hostname: string): boolean {
  return hostname.endsWith(".vercel.app");
}

/**
 * GitHub OAuth callback is registered for www.onchain-ai.xyz only. Preview hosts
 * must start OAuth on production or the state cookie and callback host mismatch.
 */
export function githubSignInHref(): string {
  if (typeof window !== "undefined" && isVercelPreviewHost(window.location.hostname)) {
    return `${SITE_ORIGIN}/auth/github`;
  }
  const base = clientApiBase();
  return `${base}/auth/github`;
}

export function githubSwitchHref(): string {
  if (typeof window !== "undefined" && isVercelPreviewHost(window.location.hostname)) {
    return `${SITE_ORIGIN}/auth/github/switch`;
  }
  const base = clientApiBase();
  return `${base}/auth/github/switch`;
}

export function productionLoginHref(): string {
  return `${SITE_ORIGIN}/login`;
}