import type { NextConfig } from "next";
import { resolveApiOrigin } from "./lib/api-origin";

const API_PROXY_TARGET = process.env.API_PROXY_TARGET ?? resolveApiOrigin();

const SECURITY_HEADERS = [
  { key: "X-Frame-Options", value: "DENY" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  {
    key: "Permissions-Policy",
    value: "camera=(), microphone=(), geolocation=()",
  },
];

const nextConfig: NextConfig = {
  env: {
    NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL ?? "",
    NEXT_PUBLIC_GITHUB_REPO:
      process.env.NEXT_PUBLIC_GITHUB_REPO ??
      "https://github.com/Coinyak/onchainai",
  },
  async headers() {
    // chains/*.svg?v=… is content-addressed via query; safe to pin forever.
    const immutableVersioned = [
      ...SECURITY_HEADERS,
      {
        key: "Cache-Control",
        value: "public, max-age=31536000, immutable",
      },
    ];
    // brand/clients are overwritten in place (no ?v=); short TTL + SWR.
    const revalidatingStatic = [
      ...SECURITY_HEADERS,
      {
        key: "Cache-Control",
        value: "public, max-age=86400, stale-while-revalidate=604800",
      },
    ];
    return [
      {
        source: "/:path*",
        headers: SECURITY_HEADERS,
      },
      { source: "/chains/:path*", headers: immutableVersioned },
      { source: "/brand/:path*", headers: revalidatingStatic },
      { source: "/clients/:path*", headers: revalidatingStatic },
    ];
  },
  async rewrites() {
    return [
      {
        source: "/api/:path*",
        destination: `${API_PROXY_TARGET}/api/:path*`,
      },
      {
        source: "/auth/:path*",
        destination: `${API_PROXY_TARGET}/auth/:path*`,
      },
      {
        source: "/onboarding/:path*",
        destination: `${API_PROXY_TARGET}/onboarding/:path*`,
      },
      {
        source: "/mcp",
        destination: `${API_PROXY_TARGET}/mcp`,
      },
      {
        source: "/mcp/:path*",
        destination: `${API_PROXY_TARGET}/mcp/:path*`,
      },
    ];
  },
};

export default nextConfig;
