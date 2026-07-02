"use client";

import { Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { getReviewQueue } from "@/lib/api";
import { AdminReviewDecisionPanel } from "@/components/admin/AdminReviewDecisionPanel";
import { ToolLogo } from "@/components/tools/ToolLogo";

function AdminToolsContent() {
  const searchParams = useSearchParams();
  const queue = searchParams.get("queue") ?? "new_candidate";
  const slug = searchParams.get("slug");

  const queueQuery = useQuery({
    queryKey: ["review-queue", queue],
    queryFn: () => getReviewQueue(queue, 50),
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

      {queueQuery.isLoading && <p className="text-secondary">Loading queue...</p>}
      <div className="space-y-4">
        {queueQuery.data?.map((item) => (
          <article key={item.tool.slug} className="border border-border rounded-md p-lg">
            <div className="flex gap-4">
              <ToolLogo name={item.tool.name} logoUrl={item.tool.logo_url} logoMonogram={item.tool.logo_monogram} />
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
                <AdminReviewDecisionPanel slug={item.tool.slug} onReviewed={() => queueQuery.refetch()} />
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