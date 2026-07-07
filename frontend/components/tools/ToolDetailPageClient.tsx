"use client";

import { Suspense } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import { getToolBySlug, getToolCommentCount } from "@/lib/api";
import { SiteShell } from "@/components/layout/SiteShell";
import { ToolDetail } from "@/components/tools/ToolDetail";
import { RelatedToolsSection } from "@/components/tools/RelatedToolsSection";
import { ToolListingActions } from "@/components/tools/ToolListingActions";
import { CommentsSection } from "@/components/comments/CommentsSection";
import { ErrorState } from "@/components/ui/ErrorState";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

interface ToolDetailPageClientProps {
  slug: string;
}

export function ToolDetailPageClient({ slug }: ToolDetailPageClientProps) {
  const searchParams = useSearchParams();
  const backParams = new URLSearchParams();
  searchParams.forEach((v, k) => {
    if (k !== "selected") backParams.set(k, v);
  });
  const backHref = backParams.toString() ? `/tools?${backParams}` : "/tools";

  const toolQuery = useQuery({
    queryKey: ["tool", slug],
    queryFn: () => getToolBySlug(slug),
  });

  const countQuery = useQuery({
    queryKey: ["comment-count", slug],
    queryFn: () => getToolCommentCount(slug),
    enabled: !!slug,
  });

  if (toolQuery.isLoading && !toolQuery.data) {
    return <ToolListSkeleton count={1} />;
  }

  if (toolQuery.isError) {
    return (
      <ErrorState
        message={toolQuery.error?.message ?? "Tool not found"}
        onRetry={() => toolQuery.refetch()}
      />
    );
  }

  const tool = toolQuery.data!;

  return (
    <>
      <Link href={backHref} className="back-link no-underline text-primary mb-6 inline-block">
        ← All Tools
      </Link>
      <ToolDetail
        tool={tool}
        trustProbe={tool.trust_probe ?? null}
        commentCount={countQuery.data ?? 0}
      />
      <div className="detail-page-tail">
        <ToolListingActions tool={tool} />
        <RelatedToolsSection slug={slug} />
        <CommentsSection slug={slug} toolName={tool.name} />
      </div>
    </>
  );
}

export function ToolDetailPageShell({ slug }: ToolDetailPageClientProps) {
  return (
    <SiteShell>
      <div className="detail-page px-gutter md:px-8 py-8 max-w-[1120px] mx-auto">
        <Suspense fallback={<ToolListSkeleton count={1} />}>
          <ToolDetailPageClient slug={slug} />
        </Suspense>
      </div>
    </SiteShell>
  );
}