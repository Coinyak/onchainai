"use client";

import { Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { getAdminToolWorkbench, getReviewQueue } from "@/lib/api";
import { AdminReviewDecisionPanel } from "@/components/admin/AdminReviewDecisionPanel";
import { ToolLogo } from "@/components/tools/ToolLogo";

function AdminToolsContent() {
  const searchParams = useSearchParams();
  const queue = searchParams.get("queue") ?? "new_candidate";
  const slug = searchParams.get("slug");
  const forceLookup = searchParams.get("lookup") === "1";

  const queueQuery = useQuery({
    queryKey: ["review-queue", queue],
    queryFn: () => getReviewQueue(queue, 50),
  });

  const queueLoaded = !queueQuery.isLoading && queueQuery.data !== undefined;
  const queueItem = slug ? queueQuery.data?.find((item) => item.tool.slug === slug) : undefined;
  const shouldLookup = Boolean(
    slug && (forceLookup || queueQuery.isError || (queueLoaded && !queueItem)),
  );

  const workbenchQuery = useQuery({
    queryKey: ["admin-tool-workbench", slug],
    queryFn: () => getAdminToolWorkbench(slug!),
    enabled: shouldLookup,
    retry: false,
  });

  return (
    <div className="px-gutter md:px-6 py-8 max-w-[1100px] mx-auto">
      <h1 className="text-h2 mb-6">Tool management</h1>

      <div className="flex flex-wrap gap-2 mb-6">
        {["new_candidate", "known_update", "high_risk_install", "reported", "needs_manual_research", "low_relevance"].map((q) => (
          <Link
            key={q}
            href={`/admin/tools?queue=${q}`}
            className={queue === q ? "sort-link active" : "sort-link"}
          >
            {q.replace(/_/g, " ")}
          </Link>
        ))}
      </div>

      {queueQuery.isError && (
        <p className="text-body-sm text-error mb-4" role="alert">
          Could not load the review queue. Tool lookup by slug still works when available.
        </p>
      )}

      {shouldLookup && workbenchQuery.isLoading && (
        <p className="text-secondary mb-4">Loading tool...</p>
      )}
      {shouldLookup && workbenchQuery.isError && (
        <article className="border border-border rounded-md p-lg mb-6">
          <p className="text-body-md text-error">Tool not found: {slug}</p>
        </article>
      )}
      {shouldLookup && workbenchQuery.data && (
        <article className="border border-border rounded-md p-lg mb-6">
          <div className="flex gap-4 mb-4">
            <ToolLogo
              name={workbenchQuery.data.tool.name}
              logoUrl={workbenchQuery.data.tool.logo_url}
              logoMonogram={workbenchQuery.data.tool.logo_monogram}
              status={workbenchQuery.data.tool.status}
            />
            <div className="flex-1 min-w-0">
              <h3 className="text-h3">{workbenchQuery.data.tool.name}</h3>
              <p className="text-body-sm text-secondary">
                {workbenchQuery.data.tool.slug} · {workbenchQuery.data.tool.status}
                {workbenchQuery.data.tool.approval_status
                  ? ` · ${workbenchQuery.data.tool.approval_status}`
                  : ""}
              </p>
              {workbenchQuery.data.tool.official_team && (
                <p className="text-body-sm text-secondary">
                  Team: {workbenchQuery.data.tool.official_team}
                </p>
              )}
            </div>
          </div>
          <AdminReviewDecisionPanel
            slug={workbenchQuery.data.tool.slug}
            tool={workbenchQuery.data.tool}
            onReviewed={() => {
              workbenchQuery.refetch();
              queueQuery.refetch();
            }}
          />
        </article>
      )}

      {queueQuery.isLoading && <p className="text-secondary">Loading queue...</p>}
      <div className="space-y-4">
        {queueQuery.data?.map((item) => (
          <article key={item.tool.slug} className="border border-border rounded-md p-lg">
            <div className="flex gap-4">
              <ToolLogo
                name={item.tool.name}
                logoUrl={item.tool.logo_url}
                logoMonogram={item.tool.logo_monogram}
                status={item.tool.status}
              />
              <div className="flex-1 min-w-0">
                <h3 className="text-h3">{item.tool.name}</h3>
                <p className="text-body-sm text-secondary">{item.queue_reason}</p>
                <Link href={`/admin/tools?queue=${queue}&slug=${item.tool.slug}`} className="text-tertiary text-body-sm">
                  Review
                </Link>
              </div>
            </div>
            {slug === item.tool.slug && (
              <div className="mt-4 border-t border-border pt-4">
                <AdminReviewDecisionPanel
                  slug={item.tool.slug}
                  tool={item.tool}
                  onReviewed={() => queueQuery.refetch()}
                />
              </div>
            )}
          </article>
        ))}
      </div>
    </div>
  );
}

export default function AdminToolsPage() {
  return (
    <Suspense fallback={<p className="p-8 text-secondary">Loading...</p>}>
      <AdminToolsContent />
    </Suspense>
  );
}