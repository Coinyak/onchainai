"use client";

import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { getAdminStats } from "@/lib/api";
import { timeAgo } from "@/lib/format";

export default function AdminDashboardPage() {
  const statsQuery = useQuery({
    queryKey: ["admin-stats"],
    queryFn: getAdminStats,
  });

  if (statsQuery.isLoading) {
    return <p className="px-6 py-8 text-secondary">Loading dashboard...</p>;
  }

  const data = statsQuery.data;
  if (!data) return null;

  return (
    <div className="px-gutter md:px-6 py-8 max-w-[1100px] mx-auto">
      <h1 className="text-h2 mb-1">Operator Dashboard</h1>
      <p className="text-secondary text-body-md mb-8">
        Review queue pressure, publication health, and crawler source status.
      </p>

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4 mb-8">
        <StatCard label="Pending candidates" value={data.pending_candidates} href="/admin/tools?queue=new_candidate" />
        <StatCard label="Known updates" value={data.known_updates} href="/admin/tools?queue=known_update" />
        <StatCard label="High risk installs" value={data.high_risk_installs} href="/admin/tools?queue=high_risk_install" accent />
        <StatCard label="Open reports" value={data.open_reports} href="/admin/tools?queue=reported" />
        <StatCard label="Needs research" value={data.needs_manual_research} href="/admin/tools?queue=needs_manual_research" />
        <StatCard label="Low relevance" value={data.low_relevance} href="/admin/tools?queue=low_relevance" />
        <StatCard label="Public tools" value={data.public_tools} href="/tools" />
      </div>

      <section>
        <h2 className="text-h2 mb-4">Crawler status</h2>
        <div className="border border-border rounded-md divide-y divide-border">
          {data.crawler_sources.map((source) => (
            <div key={source.id} className="p-4 flex justify-between gap-4 text-body-md">
              <span>{source.name}</span>
              <span className="text-secondary">
                {source.crawl_status} · {source.items_found} items · {source.last_crawled_at ? timeAgo(source.last_crawled_at) : "never"}
              </span>
            </div>
          ))}
        </div>
        <Link href="/admin/crawler" className="inline-block mt-4 text-tertiary">
          Manage crawler
        </Link>
      </section>
    </div>
  );
}

function StatCard({
  label,
  value,
  href,
  accent = false,
}: {
  label: string;
  value: number;
  href: string;
  accent?: boolean;
}) {
  return (
    <Link
      href={href}
      className={`stat-card no-underline text-inherit border rounded-md p-lg hover:border-border-strong ${
        accent ? "border-error/30" : "border-border"
      }`}
    >
      <div className="text-label-caps uppercase text-secondary">{label}</div>
      <div className="text-h2 mt-1">{value}</div>
    </Link>
  );
}