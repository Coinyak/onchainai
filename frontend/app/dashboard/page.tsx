"use client";

import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { SiteShell } from "@/components/layout/SiteShell";
import { ToolCard } from "@/components/tools/ToolCard";
import { getPublicDashboard } from "@/lib/api";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

export default function DashboardPage() {
  const dashQuery = useQuery({
    queryKey: ["dashboard"],
    queryFn: () => getPublicDashboard(12),
  });

  if (dashQuery.isLoading) return <SiteShell><ToolListSkeleton count={4} /></SiteShell>;

  const data = dashQuery.data;
  if (!data) return null;

  return (
    <SiteShell>
      <div className="px-gutter md:px-8 py-8 max-w-[1100px] mx-auto">
        <h1 className="text-h1 mb-2">Catalog dashboard</h1>
        <p className="text-secondary text-body-md mb-8">
          Operator view of public catalog metrics as of {new Date(data.as_of).toLocaleString()}
        </p>

        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4 mb-8">
          <StatCard label="Public tools" value={data.metrics.public_tools} href="/tools" />
          <StatCard label="MCP tools" value={data.metrics.mcp_tools} href="/tools?type=mcp" />
          <StatCard label="Verified" value={data.metrics.verified_tools} href="/tools?status=verified" />
          <StatCard label="Official" value={data.metrics.official_tools} href="/tools?status=official" />
        </div>

        <section className="mb-8">
          <h2 className="text-h2 mb-4">New tools</h2>
          <div className="tool-list">
            {data.new_tools.map((tool) => (
              <ToolCard key={tool.slug} tool={tool} previewHref={`/tools?selected=${tool.slug}`} />
            ))}
          </div>
        </section>
      </div>
    </SiteShell>
  );
}

function StatCard({ label, value, href }: { label: string; value: number; href: string }) {
  return (
    <Link href={href} className="stat-card no-underline text-inherit border border-border rounded-md p-lg hover:border-border-strong">
      <div className="text-label-caps uppercase text-secondary">{label}</div>
      <div className="text-h1 mt-1">{value}</div>
    </Link>
  );
}