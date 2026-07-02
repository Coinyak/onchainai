"use client";

import { Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import { SiteShell } from "@/components/layout/SiteShell";
import { ToolDetail } from "@/components/tools/ToolDetail";
import { compareTools } from "@/lib/api";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

function CompareContent() {
  const searchParams = useSearchParams();
  const slugs = (searchParams.get("slugs") ?? "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);

  const compareQuery = useQuery({
    queryKey: ["compare", slugs.join(",")],
    queryFn: () => compareTools(slugs),
    enabled: slugs.length >= 2,
  });

  if (slugs.length < 2) {
    return (
      <p className="text-secondary">
        Add at least two tool slugs via <code>?slugs=tool-a,tool-b</code>
      </p>
    );
  }

  if (compareQuery.isLoading) return <ToolListSkeleton count={2} />;

  return (
    <div className="compare-grid grid gap-8 md:grid-cols-2">
      {compareQuery.data?.tools.map((tool) => (
        <ToolDetail key={tool.slug} tool={tool} compact />
      ))}
    </div>
  );
}

export default function ComparePage() {
  return (
    <SiteShell>
      <div className="px-gutter md:px-8 py-8 max-w-[1100px] mx-auto">
        <h1 className="text-h1 mb-6">Compare tools</h1>
        <Suspense fallback={<ToolListSkeleton count={2} />}>
          <CompareContent />
        </Suspense>
      </div>
    </SiteShell>
  );
}