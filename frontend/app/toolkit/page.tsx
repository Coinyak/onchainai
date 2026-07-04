"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { SiteShell } from "@/components/layout/SiteShell";
import { ToolCard } from "@/components/tools/ToolCard";
import { GuestSignInPrompt } from "@/components/auth/GuestSignInPrompt";
import { CopyButton } from "@/components/ui/CopyButton";
import { ToolkitEmptyState } from "@/components/toolkit/ToolkitEmptyState";
import { useAuth } from "@/lib/auth";
import { getAgentLinkStatus, listMyToolkit } from "@/lib/api";
import { Badge } from "@/components/ui/Badge";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

type ToolkitFilter = "all" | "agent";

export default function ToolkitPage() {
  const { isAuthenticated } = useAuth();
  const [filter, setFilter] = useState<ToolkitFilter>("all");

  const toolkitQuery = useQuery({
    queryKey: ["toolkit"],
    queryFn: listMyToolkit,
    enabled: isAuthenticated,
  });

  const linkQuery = useQuery({
    queryKey: ["agent-link-status"],
    queryFn: getAgentLinkStatus,
    enabled: isAuthenticated,
  });

  const items = useMemo(
    () => toolkitQuery.data?.items ?? [],
    [toolkitQuery.data?.items],
  );
  const agentCount = useMemo(
    () => items.filter((item) => item.source === "agent").length,
    [items],
  );
  const visibleItems = useMemo(
    () =>
      filter === "agent"
        ? items.filter((item) => item.source === "agent")
        : items,
    [filter, items],
  );

  if (!isAuthenticated) {
    return (
      <SiteShell>
        <GuestSignInPrompt
          title="My Toolkit"
          description="Sign in to save tools and export bundles."
          testId="toolkit-sign-in"
        />
      </SiteShell>
    );
  }

  const linked = linkQuery.data?.linked ?? false;

  return (
    <SiteShell>
      <div className="px-gutter md:px-8 py-8 max-w-[960px] mx-auto">
        <div className="toolkit-page-header mb-8">
          <div>
            <h1 className="text-h1 mb-2">My Toolkit</h1>
            <p className="text-secondary text-body-md">Saved tools and export bundles.</p>
          </div>
          <Link href="/blueprints" className="toolkit-browse-link whitespace-nowrap">
            Plan on a blueprint →
          </Link>
        </div>

        {linkQuery.data && (
          <section
            className="toolkit-agent-sync-card mb-6"
            aria-labelledby="toolkit-agent-sync-heading"
            data-testid={linked ? "toolkit-agent-linked" : "toolkit-agent-not-linked"}
          >
            <h2 id="toolkit-agent-sync-heading" className="toolkit-agent-sync-label">
              Coding app connection
            </h2>
            <span className="text-secondary text-body-sm" role="status">
              {linked
                ? "Tools you save from Claude Code or Cursor will appear here."
                : "Connect your coding app on the Connect page to save tools automatically."}
            </span>
            {!linked && (
              <Link
                href="/connect#agent-sync"
                className="toolkit-agent-sync-cta"
                data-testid="toolkit-agent-sync-cta"
              >
                Link your agent
              </Link>
            )}
          </section>
        )}

        {toolkitQuery.isLoading && <ToolListSkeleton count={3} />}
        {toolkitQuery.data && (
          <>
            {items.length > 0 && (
              <div
                className="toolkit-filter-bar mb-4"
                role="group"
                aria-label="Filter toolkit"
                data-testid="toolkit-filter-bar"
              >
                <button
                  type="button"
                  className={`toolkit-filter-btn${filter === "all" ? " toolkit-filter-btn-active" : ""}`}
                  aria-pressed={filter === "all"}
                  onClick={() => setFilter("all")}
                  data-testid="toolkit-filter-all"
                >
                  All ({items.length})
                </button>
                <button
                  type="button"
                  className={`toolkit-filter-btn${filter === "agent" ? " toolkit-filter-btn-active" : ""}`}
                  aria-pressed={filter === "agent"}
                  onClick={() => setFilter("agent")}
                  data-testid="toolkit-filter-agent"
                >
                  From agent ({agentCount})
                </button>
              </div>
            )}
            {visibleItems.length > 0 ? (
              <div className="tool-list mb-8">
                {visibleItems.map((item) => (
                  <div key={item.tool.slug} className="toolkit-item-wrap">
                    {item.source === "agent" && (
                      <Badge variant="community" className="toolkit-from-agent-badge">
                        From agent
                      </Badge>
                    )}
                    <ToolCard
                      tool={item.tool}
                      previewHref={`/tools?selected=${item.tool.slug}`}
                    />
                  </div>
                ))}
              </div>
            ) : filter === "agent" && items.length > 0 ? (
              <p className="toolkit-filter-empty text-secondary text-body-md mb-8" role="status">
                No tools saved from your coding app yet. Link your agent on{" "}
                <Link href="/connect#agent-sync">Connect</Link> and save a tool from Claude Code or
                Cursor.
              </p>
            ) : (
              <ToolkitEmptyState linked={linked} />
            )}
            {toolkitQuery.data.exports?.claude_desktop && (
              <section className="toolkit-export-panel border border-border rounded-md p-lg">
                <h2 className="text-h2 mb-2">Claude Desktop export</h2>
                <CopyButton
                  text={toolkitQuery.data.exports.claude_desktop.body}
                  label="Copy export"
                />
                <pre className="toolkit-export-body mt-3 p-4 bg-neutral-surface rounded-sm text-code overflow-x-auto">
                  {toolkitQuery.data.exports.claude_desktop.body}
                </pre>
              </section>
            )}
          </>
        )}
      </div>
    </SiteShell>
  );
}