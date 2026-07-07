"use client";

import { Suspense, useCallback, useMemo } from "react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import { SiteShell } from "@/components/layout/SiteShell";
import { CompareMatrix } from "@/components/compare/CompareMatrix";
import { CompareInstallSections } from "@/components/compare/CompareInstallSections";
import { CompareAddToolTypeahead } from "@/components/compare/CompareAddToolTypeahead";
import { compareTools, getToolBySlug } from "@/lib/api";
import {
  MAX_COMPARE_TOOLS,
  MIN_COMPARE_TOOLS,
  normalizeCompareSlugs,
} from "@/lib/compare";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

function CompareContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const raw = searchParams.get("tools") ?? searchParams.get("slugs") ?? "";
  const slugs = useMemo(() => normalizeCompareSlugs(raw), [raw]);

  const updateSlugs = useCallback(
    (next: string[]) => {
      const normalized = normalizeCompareSlugs(next.join(","));
      if (normalized.length === 0) {
        router.replace("/compare", { scroll: false });
        return;
      }
      router.replace(`/compare?tools=${encodeURIComponent(normalized.join(","))}`, {
        scroll: false,
      });
    },
    [router],
  );

  const compareQuery = useQuery({
    queryKey: ["compare", slugs.join(",")],
    queryFn: () => compareTools(slugs),
    enabled: slugs.length >= MIN_COMPARE_TOOLS,
  });

  const singleToolQuery = useQuery({
    queryKey: ["compare-single", slugs[0]],
    queryFn: () => getToolBySlug(slugs[0]!),
    enabled: slugs.length === 1,
  });

  const entries = useMemo(() => {
    if (slugs.length >= MIN_COMPARE_TOOLS && compareQuery.data) {
      return slugs
        .map((slug) => compareQuery.data?.find((entry) => entry.tool.slug === slug))
        .filter((entry): entry is NonNullable<typeof entry> => Boolean(entry));
    }
    if (slugs.length === 1 && singleToolQuery.data) {
      return [
        {
          tool: singleToolQuery.data,
          official_links: [],
          trust_facts: [],
          viewer_bookmarked: false,
          trust_probe: singleToolQuery.data.trust_probe ?? null,
        },
      ];
    }
    return [];
  }, [compareQuery.data, singleToolQuery.data, slugs]);

  const tools = entries.map((entry) => entry.tool);
  const isLoading =
    (slugs.length >= MIN_COMPARE_TOOLS && compareQuery.isLoading) ||
    (slugs.length === 1 && singleToolQuery.isLoading);

  function addSlug(slug: string) {
    if (slugs.includes(slug) || slugs.length >= MAX_COMPARE_TOOLS) return;
    updateSlugs([...slugs, slug]);
  }

  function removeSlug(slug: string) {
    updateSlugs(slugs.filter((value) => value !== slug));
  }

  if (slugs.length === 0) {
    return (
      <div className="compare-empty" data-testid="compare-empty">
        <h2 className="text-h2">Compare tools side by side</h2>
        <p>
          Add two to four tools to compare type, trust signals, chains, pricing, and install
          steps. Share the URL to revisit the same comparison.
        </p>
        <CompareAddToolTypeahead selectedSlugs={[]} onSelect={addSlug} />
        <p className="compare-empty-hint">
          Or open a comparison from a tool card or detail page, or use{" "}
          <code>?tools=tool-a,tool-b</code> in the URL.
        </p>
        <Link href="/tools" className="compare-browse-link">
          Browse tools
        </Link>
      </div>
    );
  }

  if (isLoading) {
    return <ToolListSkeleton count={Math.max(slugs.length, MIN_COMPARE_TOOLS)} />;
  }

  if (slugs.length === 1 && singleToolQuery.isError) {
    return (
      <div className="compare-empty">
        <h2 className="text-h2">Tool not found</h2>
        <p>
          <code>{slugs[0]}</code> could not be loaded. Try another slug or browse the directory.
        </p>
        <Link href="/tools" className="compare-browse-link">
          Browse tools
        </Link>
      </div>
    );
  }

  if (slugs.length >= MIN_COMPARE_TOOLS && compareQuery.isError) {
    return (
      <div className="compare-empty">
        <h2 className="text-h2">Comparison unavailable</h2>
        <p>{compareQuery.error?.message ?? "Failed to load comparison."}</p>
        <Link href="/tools" className="compare-browse-link">
          Browse tools
        </Link>
      </div>
    );
  }

  return (
    <>
      {slugs.length < MIN_COMPARE_TOOLS && (
        <p className="compare-prompt" role="status">
          Add at least one more tool to compare attributes.
        </p>
      )}
      <CompareMatrix entries={entries} onRemove={removeSlug} onAdd={addSlug} />
      {entries.length >= MIN_COMPARE_TOOLS && (
        <CompareInstallSections entries={entries} slugs={slugs} />
      )}
    </>
  );
}

export default function ComparePage() {
  return (
    <SiteShell>
      <div className="compare-page" data-testid="compare-page">
        <header className="compare-header">
          <div>
            <h1>Compare tools</h1>
            <p>
              Scan differences across type, trust, chains, and pricing. Install guides stay below
              in collapsible sections.
            </p>
          </div>
          <Link href="/tools" className="compare-browse-link">
            Browse tools
          </Link>
        </header>
        <Suspense fallback={<ToolListSkeleton count={2} />}>
          <CompareContent />
        </Suspense>
      </div>
    </SiteShell>
  );
}