"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { listCrawlerSources, triggerCrawlerSource } from "@/lib/api";
import { timeAgo } from "@/lib/format";

export default function AdminCrawlerPage() {
  const queryClient = useQueryClient();
  const sourcesQuery = useQuery({
    queryKey: ["crawler-sources"],
    queryFn: listCrawlerSources,
  });

  const triggerMut = useMutation({
    mutationFn: triggerCrawlerSource,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["crawler-sources"] }),
  });

  return (
    <div className="px-gutter md:px-6 py-8 max-w-[900px] mx-auto">
      <h1 className="text-h2 mb-6">Crawler control</h1>
      <div className="border border-border rounded-md divide-y divide-border">
        {sourcesQuery.data?.map((source) => (
          <div key={source.id ?? source.name} className="p-4 flex flex-wrap justify-between gap-3 items-center">
            <div>
              <div className="font-medium">{source.name}</div>
              <div className="text-body-sm text-secondary">
                {source.crawl_status} · {source.items_found} items · {source.last_crawled_at ? timeAgo(source.last_crawled_at) : "never"}
              </div>
              {source.error_message && (
                <p className="text-body-sm text-error mt-1">{source.error_message}</p>
              )}
            </div>
            <button
              type="button"
              className="min-h-touch px-4 rounded-md border border-border-strong"
              disabled={triggerMut.isPending || !source.id}
              onClick={() => source.id && triggerMut.mutate(source.id)}
            >
              Run now
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}