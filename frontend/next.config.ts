import type { NextConfig } from "next";

const API_PROXY_TARGET =
  process.env.API_PROXY_TARGET ?? "http://localhost:3000";

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
      "https://github.com/hoyeon4315-cpu/onchainai",
  },
  async headers() {
    return [
      {
        source: "/:path*",
        headers: SECURITY_HEADERS,
      },
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
