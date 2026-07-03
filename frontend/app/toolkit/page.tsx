"use client";

import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { SiteShell } from "@/components/layout/SiteShell";
import { ToolCard } from "@/components/tools/ToolCard";
import { GuestSignInPrompt } from "@/components/auth/GuestSignInPrompt";
import { CopyButton } from "@/components/ui/CopyButton";
import { useAuth } from "@/lib/auth";
import { listMyToolkit } from "@/lib/api";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

export default function ToolkitPage() {
  const { isAuthenticated } = useAuth();

  const toolkitQuery = useQuery({
    queryKey: ["toolkit"],
    queryFn: listMyToolkit,
    enabled: isAuthenticated,
  });

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

        {toolkitQuery.isLoading && <ToolListSkeleton count={3} />}
        {toolkitQuery.data && (
          <>
            <div className="tool-list mb-8">
              {toolkitQuery.data.items.map((item) => (
                <ToolCard
                  key={item.tool.slug}
                  tool={item.tool}
                  previewHref={`/tools?selected=${item.tool.slug}`}
                />
              ))}
            </div>
            {toolkitQuery.data.items.length === 0 && (
              <p className="text-secondary">No saved tools yet. Bookmark tools from the directory.</p>
            )}
            <section className="toolkit-export-panel border border-border rounded-md p-lg">
              <h2 className="text-h2 mb-2">Claude Desktop export</h2>
              <CopyButton text={toolkitQuery.data.exports.claude_desktop.body} label="Copy export" />
              <pre className="toolkit-export-body mt-3 p-4 bg-neutral-surface rounded-sm text-code overflow-x-auto">
                {toolkitQuery.data.exports.claude_desktop.body}
              </pre>
            </section>
          </>
        )}
      </div>
    </SiteShell>
  );
}