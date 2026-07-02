"use client";

import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  listCrawlerSources,
  triggerCrawlerSource,
  updateCrawlerSource,
  type CrawlerSourceView,
} from "@/lib/api";
import { timeAgo } from "@/lib/format";

function CrawlerSourceRow({ source }: { source: CrawlerSourceView }) {
  const queryClient = useQueryClient();
  const [scheduleMinutes, setScheduleMinutes] = useState(source.schedule_minutes);
  const [enabled, setEnabled] = useState(source.enabled);
  const [synced, setSynced] = useState({
    id: source.id,
    schedule_minutes: source.schedule_minutes,
    enabled: source.enabled,
  });

  if (
    source.id !== synced.id ||
    source.schedule_minutes !== synced.schedule_minutes ||
    source.enabled !== synced.enabled
  ) {
    setSynced({
      id: source.id,
      schedule_minutes: source.schedule_minutes,
      enabled: source.enabled,
    });
    setScheduleMinutes(source.schedule_minutes);
    setEnabled(source.enabled);
  }

  const updateMut = useMutation({
    mutationFn: () =>
      updateCrawlerSource(source.id!, {
        schedule_minutes: scheduleMinutes,
        enabled,
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["crawler-sources"] }),
  });

  const triggerMut = useMutation({
    mutationFn: () => triggerCrawlerSource(source.id!),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["crawler-sources"] }),
  });

  const dirty =
    scheduleMinutes !== source.schedule_minutes || enabled !== source.enabled;
  const validSchedule = Number.isFinite(scheduleMinutes) && scheduleMinutes >= 1;
  const busy = updateMut.isPending || triggerMut.isPending;

  return (
    <div className="p-4 flex flex-wrap justify-between gap-4 items-start">
      <div className="min-w-0 flex-1">
        <div className="font-medium">{source.name}</div>
        <div className="text-body-sm text-secondary">
          {source.crawl_status} · {source.items_found} items ·{" "}
          {source.last_crawled_at ? timeAgo(source.last_crawled_at) : "never"}
        </div>
        <div className="text-body-sm text-secondary mt-2">
          Schedule: every {source.schedule_minutes} min · {source.enabled ? "enabled" : "disabled"}
        </div>
        {source.error_message && (
          <p className="text-body-sm text-error mt-2">{source.error_message}</p>
        )}
      </div>

      <div className="flex flex-wrap items-end gap-4">
        <label className="block">
          <span className="text-body-sm text-secondary">Interval (minutes)</span>
          <input
            type="number"
            min={1}
            className="mt-2 w-full min-h-touch px-4 rounded-md border border-border"
            value={scheduleMinutes}
            onChange={(e) => setScheduleMinutes(Number(e.target.value))}
            disabled={!source.id || busy}
          />
        </label>

        <label className="flex min-h-touch items-center gap-2 text-body-sm">
          <input
            type="checkbox"
            className="size-4 rounded border-border"
            checked={enabled}
            onChange={(e) => setEnabled(e.target.checked)}
            disabled={!source.id || busy}
          />
          Enabled
        </label>

        <button
          type="button"
          className="min-h-touch px-4 rounded-md bg-tertiary text-on-tertiary font-medium disabled:opacity-50"
          disabled={!source.id || busy || !dirty || !validSchedule}
          onClick={() => source.id && updateMut.mutate()}
        >
          {updateMut.isPending ? "Saving..." : "Save"}
        </button>

        <button
          type="button"
          className="min-h-touch px-4 rounded-md border border-border-strong disabled:opacity-50"
          disabled={!source.id || busy}
          onClick={() => source.id && triggerMut.mutate()}
        >
          {triggerMut.isPending ? "Running..." : "Run now"}
        </button>
      </div>

      {updateMut.isSuccess && !dirty && (
        <p className="w-full text-body-sm text-success">Schedule saved.</p>
      )}
      {updateMut.isError && (
        <p className="w-full text-body-sm text-error">
          {updateMut.error instanceof Error ? updateMut.error.message : "Failed to save schedule"}
        </p>
      )}
    </div>
  );
}

export default function AdminCrawlerPage() {
  const sourcesQuery = useQuery({
    queryKey: ["crawler-sources"],
    queryFn: listCrawlerSources,
  });

  return (
    <div className="px-gutter md:px-6 py-8 max-w-[900px] mx-auto">
      <h1 className="text-h2 mb-6">Crawler control</h1>
      <p className="text-secondary text-body-md mb-6">
        Set crawl intervals, enable or disable sources, and trigger manual runs.
      </p>

      {sourcesQuery.isLoading && <p className="text-secondary">Loading sources...</p>}
      {sourcesQuery.isError && (
        <p className="text-error text-body-md">
          {sourcesQuery.error instanceof Error
            ? sourcesQuery.error.message
            : "Failed to load crawler sources"}
        </p>
      )}

      {sourcesQuery.data && (
        <div className="border border-border rounded-md divide-y divide-border">
          {sourcesQuery.data.map((source) => (
            <CrawlerSourceRow key={source.id ?? source.name} source={source} />
          ))}
          {sourcesQuery.data.length === 0 && (
            <p className="p-4 text-secondary">No crawler sources configured.</p>
          )}
        </div>
      )}
    </div>
  );
}