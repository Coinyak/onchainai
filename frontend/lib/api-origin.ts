/** Default Rust API origin (Railway production). */
export const DEFAULT_RAILWAY_API = "https://onchainai-production.up.railway.app";

const LOCAL_DEV_API = "http://localhost:3000";

/**
 * Absolute API origin for server-side fetches (SSR, sitemap, metadata).
 * Vercel preview often has no env vars — fall back to Railway like next.config rewrites.
 */
export function resolveApiOrigin(): string {
  const fromEnv =
    process.env.API_PROXY_TARGET?.replace(/\/$/, "") ||
    process.env.NEXT_PUBLIC_API_URL?.replace(/\/$/, "");
  if (fromEnv) return fromEnv;
  if (process.env.VERCEL) return DEFAULT_RAILWAY_API;
  return LOCAL_DEV_API;
}