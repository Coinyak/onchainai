"use client";

import { Suspense } from "react";
import { useQuery } from "@tanstack/react-query";
import { getFeaturedCards, getSiteSettings } from "@/lib/api";
import { ToolsBrowser } from "@/components/tools/ToolsBrowser";
import { SearchBar } from "@/components/tools/SearchBar";
import { FeaturedCarousel } from "@/components/tools/FeaturedCarousel";
import { PromoCards } from "@/components/tools/PromoCards";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

const DEFAULT_SETTINGS = {
  slogan: "Crypto tools, unified.",
  description:
    "Discover, install, and share crypto MCP, CLI, SDK, API, x402, RWA, and AI agent tools — all in one place.",
  mcp_endpoint: "npx mcp-remote www.onchain-ai.xyz/mcp",
  hero_title: null as string | null,
  hero_subtitle: null as string | null,
};

function HomeHero() {
  const settingsQuery = useQuery({
    queryKey: ["site-settings"],
    queryFn: getSiteSettings,
    retry: false,
  });
  const featuredQuery = useQuery({
    queryKey: ["featured"],
    queryFn: getFeaturedCards,
  });

  const settings = settingsQuery.data ?? DEFAULT_SETTINGS;
  const featured = featuredQuery.data ?? [];
  const heroTitle = settings.hero_title?.trim() || settings.slogan;
  const heroSubtitle = settings.hero_subtitle?.trim() || settings.description;

  return (
    <div className="home-page px-gutter md:px-6 py-8 md:py-10">
      <section className="hero mb-8">
        <h1 className="text-h1 md:text-[36px] font-bold tracking-tight leading-tight mb-3">
          {heroTitle}
        </h1>
        <p className="text-secondary text-body-md md:text-mobile-body leading-relaxed mb-6 max-w-[720px]">
          {heroSubtitle}
        </p>
        <SearchBar />
      </section>
      <FeaturedCarousel cards={featured} />
      <section className="mb-6">
        <PromoCards mcpEndpoint={settings.mcp_endpoint} />
      </section>
    </div>
  );
}

function HomeContent() {
  return (
    <ToolsBrowser base="home" showToolbarSearch={false}>
      <HomeHero />
    </ToolsBrowser>
  );
}

export default function HomePage() {
  return (
    <Suspense fallback={<ToolListSkeleton count={6} />}>
      <HomeContent />
    </Suspense>
  );
}