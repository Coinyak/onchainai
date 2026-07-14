import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { cache } from "react";
import {
  dehydrate,
  HydrationBoundary,
  QueryClient,
} from "@tanstack/react-query";
import { ToolDetailPageShell } from "@/components/tools/ToolDetailPageClient";
import { buildSoftwareApplicationJsonLd } from "@/lib/json-ld";
import { typeBadgeLabel } from "@/lib/format";
import {
  getToolBySlugServer,
  getToolCommentCountServer,
  ServerApiError,
} from "@/lib/server-api";
import { DEFAULT_OG_IMAGE_PATH, SEO_REVALIDATE_SECONDS } from "@/lib/site";
import { serializeJsonLd } from "@/lib/json-ld";

// On-demand ISR (revalidate below): each slug renders on first request then
// caches. We intentionally do NOT generateStaticParams here — build-time
// prerendering of many slugs couples deploy success to live backend latency
// (a single slow tool-detail fetch fails the whole build). On-demand ISR
// already bounds per-slug cost; the real MCP-proxy cost lever is edge routing.

/** Must be a literal (Next segment config); keep in sync with SEO_REVALIDATE_SECONDS. */
export const revalidate = 300;

type PageProps = {
  params: Promise<{ slug: string }>;
};

function toolDescription(tool: { description: string | null; name: string; type: string }): string {
  if (tool.description?.trim()) return tool.description.trim();
  return `${tool.name} — ${typeBadgeLabel(tool.type)} tool in the OnchainAI crypto directory.`;
}

const fetchToolForPage = cache(async (slug: string) => {
  try {
    return await getToolBySlugServer(slug);
  } catch (error) {
    if (error instanceof ServerApiError && error.status === 404) {
      notFound();
    }
    throw error;
  }
});

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { slug } = await params;
  const tool = await fetchToolForPage(slug);
  const title = `${tool.name} — ${typeBadgeLabel(tool.type)} | OnchainAI`;
  const description = toolDescription(tool);
  const canonical = `/tools/${tool.slug}`;
  const ogImage = tool.logo_url?.startsWith("http")
    ? tool.logo_url
    : DEFAULT_OG_IMAGE_PATH;

  return {
    title,
    description,
    alternates: {
      canonical,
    },
    openGraph: {
      title,
      description,
      url: canonical,
      siteName: "OnchainAI",
      type: "website",
      images: [
        {
          url: ogImage,
          width: 1200,
          height: 630,
          alt: `${tool.name} on OnchainAI`,
        },
      ],
    },
    twitter: {
      card: "summary_large_image",
      title,
      description,
      images: [ogImage],
    },
  };
}

export default async function ToolDetailPage({ params }: PageProps) {
  const { slug } = await params;
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: SEO_REVALIDATE_SECONDS * 1000,
      },
    },
  });

  const [tool] = await Promise.all([
    fetchToolForPage(slug),
    queryClient.prefetchQuery({
      queryKey: ["tool", slug],
      queryFn: () => fetchToolForPage(slug),
    }),
    queryClient.prefetchQuery({
      queryKey: ["comment-count", slug],
      queryFn: () => getToolCommentCountServer(slug),
    }),
  ]);

  const jsonLd = buildSoftwareApplicationJsonLd(tool);

  return (
    <HydrationBoundary state={dehydrate(queryClient)}>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: serializeJsonLd(jsonLd) }}
      />
      <ToolDetailPageShell slug={slug} />
    </HydrationBoundary>
  );
}