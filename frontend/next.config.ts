import type { NextConfig } from "next";

const API_PROXY_TARGET =
  process.env.API_PROXY_TARGET ?? "https://onchainai-production.up.railway.app";

const nextConfig: NextConfig = {
  env: {
    NEXT_PUBLIC_API_URL:
      process.env.NEXT_PUBLIC_API_URL ?? "https://www.onchain-ai.xyz",
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
