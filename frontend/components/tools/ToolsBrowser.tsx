"use client";

import { useMemo } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import {
  loadBrowserData,
  getToolBySlug,
  type CategoryWithCount,
} from "@/lib/api";
import {
  type BrowserBase,
  paramsFromSearchParams,
  buildQueryBase,
  forFilterNavigation,
  forSort,
  forStatusFilter,
  forTypeFilter,
  forNextPage,
  withSelected,
  withoutSelected,
  shouldShowLoadMore,
  buildToolFilters,
} from "@/lib/browser-query";
import { Sidebar } from "@/components/layout/Sidebar";
import { ChainStrip } from "@/components/tools/ChainStrip";
import { ToolCard } from "@/components/tools/ToolCard";
import { PreviewPanel } from "@/components/tools/PreviewPanel";
import { BottomSheet } from "@/components/tools/BottomSheet";
import { EmptyState } from "@/components/ui/EmptyState";
import { ErrorState } from "@/components/ui/ErrorState";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

function normalizeCategories(rows: [import("@/lib/api").Category, number][]): CategoryWithCount[] {
  return rows.map(([category, count]) => ({ category, count }));
}

interface ToolbarSearchProps {
  base: BrowserBase;
  initialQ: string;
}

function ToolbarSearch({ base, initialQ }: ToolbarSearchProps) {
  const searchParams = useSearchParams();
  const params = paramsFromSearchParams(base, searchParams);

  return (
    <form className="toolbar-search" action={buildQueryBase(base, { ...params, page: 1 })} method="get">
      <input
        type="search"
        name="q"
        placeholder="Search tools..."
        defaultValue={initialQ}
        className="toolbar-search-input"
      />
      {params.sort !== "hot" && <input type="hidden" name="sort" value={params.sort} />}
      {params.function && <input type="hidden" name="function" value={params.function} />}
      {params.chain && <input type="hidden" name="chain" value={params.chain} />}
      {params.type && <input type="hidden" name="type" value={params.type} />}
    </form>
  );
}

interface ToolsBrowserProps {
  base: BrowserBase;
  showToolbarSearch?: boolean;
  children?: React.ReactNode;
}

export function ToolsBrowser({ base, showToolbarSearch = false, children }: ToolsBrowserProps) {
  const searchParams = useSearchParams();
  const params = useMemo(
    () => paramsFromSearchParams(base, searchParams),
    [base, searchParams],
  );
  const filters = buildToolFilters(params);
  const queryBase = buildQueryBase(base, params);
  const filterQueryBase = buildQueryBase(base, forFilterNavigation(params));

  const browserQuery = useQuery({
    queryKey: ["browser-data", base, params.sort, filters, params.q, params.page],
    queryFn: () =>
      loadBrowserData({
        sort: params.sort,
        filters,
        search_q: params.q ?? null,
        selected: null,
        page: params.page,
      }),
  });

  const selectedSlug = params.selected;
  const previewQuery = useQuery({
    queryKey: ["preview-tool", selectedSlug],
    queryFn: () => getToolBySlug(selectedSlug!),
    enabled: !!selectedSlug,
  });

  const categories = browserQuery.data
    ? normalizeCategories(browserQuery.data.categories)
    : [];

  const sortHot = buildQueryBase(base, forSort(params, "hot"));
  const sortNew = buildQueryBase(base, forSort(params, "new"));
  const sortComments = buildQueryBase(base, forSort(params, "comments"));
  const statusVerified = buildQueryBase(base, forStatusFilter(params, "verified"));
  const statusOfficial = buildQueryBase(base, forStatusFilter(params, "official"));
  const typeMcp = buildQueryBase(base, forTypeFilter(params, "mcp"));
  const typeCli = buildQueryBase(base, forTypeFilter(params, "cli"));
  const typeApi = buildQueryBase(base, forTypeFilter(params, "api"));
  const typeSdk = buildQueryBase(base, forTypeFilter(params, "sdk"));
  const typeSkill = buildQueryBase(base, forTypeFilter(params, "skill"));
  const typeX402 = buildQueryBase(base, forTypeFilter(params, "x402"));
  const loadMoreHref = buildQueryBase(base, forNextPage(params));
  const closePreviewHref = withoutSelected(base, queryBase);

  return (
    <div className="tools-layout" data-tools-browser="">
      {browserQuery.isLoading && !browserQuery.data ? (
        <>
          <Sidebar
            base={base}
            categories={[]}
            queryBase={filterQueryBase}
            defaultFunctionOpen={base === "tools"}
          />
          <div className="tools-main">
            {children && <div className="tools-prepend">{children}</div>}
            <ToolListSkeleton count={6} />
          </div>
        </>
      ) : browserQuery.isError ? (
        <>
          <Sidebar
            base={base}
            categories={categories}
            queryBase={filterQueryBase}
            activeFunction={params.function}
            activeAssetClass={params.asset_class}
            activeActor={params.actor}
            activeType={params.type}
            activeStatus={params.status}
            activePricing={params.pricing}
            activeInstallRisk={params.install_risk}
            defaultFunctionOpen={base === "tools"}
          />
          <div className="tools-main">
            {children && <div className="tools-prepend">{children}</div>}
            <ErrorState
              message={browserQuery.error?.message ?? "Unknown error"}
              onRetry={() => browserQuery.refetch()}
            />
          </div>
        </>
      ) : browserQuery.data ? (
        <>
          <Sidebar
            base={base}
            categories={categories}
            queryBase={filterQueryBase}
            activeFunction={params.function}
            activeAssetClass={params.asset_class}
            activeActor={params.actor}
            activeType={params.type}
            activeStatus={params.status}
            activePricing={params.pricing}
            activeInstallRisk={params.install_risk}
            defaultFunctionOpen={base === "tools"}
          />
          <div className="tools-main">
            {children && <div className="tools-prepend">{children}</div>}
            <ChainStrip
              base={base}
              queryBase={filterQueryBase}
              activeChain={params.chain}
              chainCounts={browserQuery.data.chains}
            />
            <div className="tools-toolbar sticky-toolbar">
              {showToolbarSearch && (
                <ToolbarSearch base={base} initialQ={params.q ?? ""} />
              )}
              <div className="toolbar-rows">
                <div className="toolbar-sort-row">
                  <Link href={sortHot} className={params.sort === "hot" ? "sort-link active" : "sort-link"}>HOT ↓</Link>
                  <Link href={sortNew} className={params.sort === "new" ? "sort-link active" : "sort-link"}>New</Link>
                  <Link href={sortComments} className={params.sort === "comments" ? "sort-link active" : "sort-link"}>Comments</Link>
                  <Link href={statusVerified} className={params.status === "verified" ? "sort-link active" : "sort-link"}>Verified</Link>
                  <Link href={statusOfficial} className={params.status === "official" ? "sort-link active" : "sort-link"}>Official</Link>
                </div>
                <div className="toolbar-filter-row">
                  <Link href={typeMcp} className={params.type === "mcp" ? "sort-link active" : "sort-link"}>MCP</Link>
                  <Link href={typeCli} className={params.type === "cli" ? "sort-link active" : "sort-link"}>CLI</Link>
                  <Link href={typeApi} className={params.type === "api" ? "sort-link active" : "sort-link"}>API</Link>
                  <Link href={typeSdk} className={params.type === "sdk" ? "sort-link active" : "sort-link"}>SDK</Link>
                  <Link href={typeSkill} className={params.type === "skill" ? "sort-link active" : "sort-link"}>Skill</Link>
                  <Link href={typeX402} className={params.type === "x402" ? "sort-link active" : "sort-link"}>x402</Link>
                </div>
              </div>
              <span className="tool-count">{browserQuery.data.total} tools</span>
            </div>

            {browserQuery.data.tools.length === 0 ? (
              <EmptyState clearHref={base === "home" ? "/" : base === "tools" ? "/tools" : `/categories/${(base as { category: string }).category}`} />
            ) : (
              <>
                <div className="tool-list">
                  {browserQuery.data.tools.map((tool) => (
                    <ToolCard
                      key={tool.slug}
                      tool={tool}
                      previewHref={withSelected(base, queryBase, tool.slug)}
                      isSelected={selectedSlug === tool.slug}
                      commentCount={browserQuery.data!.comment_counts[tool.slug] ?? 0}
                    />
                  ))}
                </div>
                {shouldShowLoadMore(
                  browserQuery.data.tools.length,
                  browserQuery.data.total,
                  params.page,
                ) && (
                  <div className="load-more-row">
                    <Link href={loadMoreHref} className="load-more-btn">
                      Load more
                    </Link>
                    <span className="load-more-count">
                      Showing {browserQuery.data.tools.length} of {browserQuery.data.total}
                    </span>
                  </div>
                )}
              </>
            )}

            {previewQuery.data && (
              <>
                <div className="preview-desktop">
                  <PreviewPanel
                    tool={previewQuery.data}
                    closeHref={closePreviewHref}
                    fullPageHref={`/tools/${previewQuery.data.slug}`}
                    commentCount={browserQuery.data.comment_counts[previewQuery.data.slug] ?? 0}
                  />
                </div>
                <div className="preview-mobile">
                  <BottomSheet
                    tool={previewQuery.data}
                    closeHref={closePreviewHref}
                    fullPageHref={`/tools/${previewQuery.data.slug}`}
                    commentCount={browserQuery.data.comment_counts[previewQuery.data.slug] ?? 0}
                  />
                </div>
              </>
            )}
          </div>
        </>
      ) : null}
    </div>
  );
}