"use client";

import { Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { SiteShell } from "@/components/layout/SiteShell";
import { InstallGuidePanel } from "@/components/tools/InstallGuidePanel";
import { AddMcpAction } from "@/components/tools/AddMcpAction";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { compareTools } from "@/lib/api";
import { toolHasInstallPath } from "@/lib/install-guide";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

function normalizeCompareSlugs(raw: string): string[] {
  const seen = new Set<string>();
  return raw
    .split(",")
    .map((part) => {
      try {
        return decodeURIComponent(part.trim()).toLowerCase();
      } catch {
        return part.trim().toLowerCase();
      }
    })
    .filter((part) => part && !seen.has(part) && seen.add(part))
    .slice(0, 3);
}

function CompareContent() {
  const searchParams = useSearchParams();
  const raw = searchParams.get("tools") ?? searchParams.get("slugs") ?? "";
  const slugs = normalizeCompareSlugs(raw);

  const compareQuery = useQuery({
    queryKey: ["compare", slugs.join(",")],
    queryFn: () => compareTools(slugs),
    enabled: slugs.length >= 2,
  });

  if (slugs.length < 2) {
    return (
      <p className="text-secondary">
        Add at least two tool slugs via <code>?tools=tool-a,tool-b</code>
      </p>
    );
  }

  if (compareQuery.isLoading) return <ToolListSkeleton count={2} />;

  return (
    <div className="compare-grid grid gap-8 md:grid-cols-2">
      {compareQuery.data?.map((entry) => {
        const tool = entry.tool;
        return (
        <article key={tool.slug} className="compare-card">
          <header className="compare-card-header">
            <ToolLogo
              name={tool.name}
              logoUrl={tool.logo_url}
              logoMonogram={tool.logo_monogram}
              size={48}
            />
            <div>
              <h2 className="text-h2">{tool.name}</h2>
              <p className="text-body-sm text-secondary">
                {tool.description ?? "No description."}
              </p>
            </div>
          </header>
          <InstallGuidePanel tool={tool} compact />
          <div className="compare-card-actions">
            <Link href={`/tools/${tool.slug}`}>Open details</Link>
            {toolHasInstallPath(tool) && (
              <AddMcpAction
                tool={tool}
                hrefSource={{ kind: "compare_slugs", slugs }}
                variant="inline_button"
              />
            )}
            <Link href="/toolkit">Open toolkit</Link>
          </div>
        </article>
        );
      })}
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