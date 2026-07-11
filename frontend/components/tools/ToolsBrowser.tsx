"use client";

import { useCallback, useEffect, useMemo } from "react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import {
  loadBrowserData,
  getCategories,
  getToolBySlug,
  normalizeCategoryRows,
  type CategoryWithCount,
} from "@/lib/api";
import { FUNCTION_CATEGORY_FALLBACK } from "@/lib/function-categories";
import {
  type BrowserBase,
  ADD_MCP_INTENT,
  paramsFromSearchParams,
  buildQueryBase,
  forFilterNavigation,
  forSort,
  forStatusFilter,
  forPricingFilter,
  forTypeFilter,
  isX402FilterActive,
  forNextPage,
  withSelected,
  withoutSelected,
  shouldShowLoadMore,
  buildToolFilters,
  buildFilterRevision,
  compareReturnHref,
  stripPreviewParams,
} from "@/lib/browser-query";
import { Sidebar } from "@/components/layout/Sidebar";
import { ChainStrip } from "@/components/tools/ChainStrip";
import { ToolCard } from "@/components/tools/ToolCard";
import { PreviewPanel } from "@/components/tools/PreviewPanel";
import { BottomSheet } from "@/components/tools/BottomSheet";
import { EmptyState } from "@/components/ui/EmptyState";
import { ErrorState } from "@/components/ui/ErrorState";
import { ToolListSkeleton } from "@/components/ui/Skeleton";
import { ToolSearchCombobox } from "@/components/tools/ToolSearchCombobox";
import {
  buildEmptyIntersectionMessage,
  describeActiveFilters,
} from "@/lib/describe-active-filters";

function mergeCategoryCounts(
  labels: CategoryWithCount[],
  scoped: CategoryWithCount[] | null,
): CategoryWithCount[] {
  if (!scoped?.length) return labels;
  const countById = new Map(scoped.map((row) => [row.category.id, row.count]));
  return labels.map((row) => ({
    category: row.category,
    count: countById.get(row.category.id) ?? 0,
  }));
}

interface ToolbarSearchProps {
  base: BrowserBase;
  initialQ: string;
}

const SORT_OPTIONS = [
  { value: "hot", label: "HOT ↓" },
  { value: "new", label: "New" },
  { value: "comments", label: "Comments" },
] as const;

const TYPE_CHIPS = [
  { id: "mcp", label: "MCP" },
  { id: "cli", label: "CLI" },
  { id: "api", label: "API" },
  { id: "sdk", label: "SDK" },
  { id: "skill", label: "Skill" },
  { id: "x402", label: "x402" },
] as const;

const STATUS_CHIPS = [
  { id: "verified", label: "Verified" },
  { id: "official", label: "Official" },
] as const;

interface MobileToolbarStripProps {
  sort: string;
  sortHot: string;
  sortNew: string;
  sortComments: string;
  typeHrefs: Record<string, string>;
  statusHrefs: Record<string, string>;
  activeType?: string;
  x402Active?: boolean;
  activeStatus?: string;
  toolCount: number;
}

function MobileToolbarStrip({
  sort,
  sortHot,
  sortNew,
  sortComments,
  typeHrefs,
  statusHrefs,
  activeType,
  x402Active = false,
  activeStatus,
  toolCount,
}: MobileToolbarStripProps) {
  const router = useRouter();
  const sortHrefs: Record<string, string> = {
    hot: sortHot,
    new: sortNew,
    comments: sortComments,
  };

  return (
    <div className="toolbar-mobile">
      <span className="tool-count tool-count-mobile toolbar-mobile-count">{toolCount} tools</span>
      <div className="toolbar-mobile-strip sticky-toolbar">
        <label className="toolbar-sort-label">
          <span className="sr-only">Sort tools</span>
          <select
            className="toolbar-sort-select"
            value={sort}
            aria-label="Sort tools"
            onChange={(ev) => {
              const href = sortHrefs[ev.target.value];
              if (href) router.push(href, { scroll: false });
            }}
          >
            {SORT_OPTIONS.map(({ value, label }) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
        <div className="toolbar-type-chips" role="group" aria-label="Filter by type or status" tabIndex={0}>
          {TYPE_CHIPS.map(({ id, label }) => (
            <Link
              key={id}
              href={typeHrefs[id]}
              scroll={false}
              prefetch={false}
              className={
                (id === "x402" ? x402Active : activeType === id)
                  ? "toolbar-type-chip active"
                  : "toolbar-type-chip"
              }
            >
              {label}
            </Link>
          ))}
          {STATUS_CHIPS.map(({ id, label }) => (
            <Link
              key={id}
              href={statusHrefs[id]}
              scroll={false}
              prefetch={false}
              className={activeStatus === id ? "toolbar-type-chip active" : "toolbar-type-chip"}
            >
              {label}
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}

function ToolbarSearch({ base, initialQ }: ToolbarSearchProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const params = paramsFromSearchParams(base, searchParams);

  const navigateWithQuery = useCallback(
    (q: string) => {
      const trimmed = q.trim();
      const nextParams = {
        ...params,
        q: trimmed || undefined,
        selected: undefined,
        intent: undefined,
        page: 1,
      };
      const href = buildQueryBase(base, nextParams);
      const currentQ = params.q?.trim() ?? "";
      if (trimmed === currentQ) return;
      router.push(href, { scroll: false });
    },
    [base, params, router],
  );

  return (
    <div className="toolbar-search">
      <ToolSearchCombobox
        variant="toolbar"
        defaultValue={initialQ}
        onSubmitSearch={navigateWithQuery}
        data-testid="toolbar-search-bar"
      />
    </div>
  );
}

interface ToolsBrowserProps {
  base: BrowserBase;
  showToolbarSearch?: boolean;
  children?: React.ReactNode;
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return (
    tag === "INPUT" ||
    tag === "TEXTAREA" ||
    tag === "SELECT" ||
    target.isContentEditable
  );
}

export function ToolsBrowser({ base, showToolbarSearch = false, children }: ToolsBrowserProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const params = useMemo(
    () => paramsFromSearchParams(base, searchParams),
    [base, searchParams],
  );
  const filters = buildToolFilters(params);
  const queryBase = buildQueryBase(base, params);
  const filterQueryBase = buildQueryBase(base, forFilterNavigation(params));
  const filterRevision = buildFilterRevision(params);
  const addMode = params.intent === ADD_MCP_INTENT;
  const compareBackHref = compareReturnHref(params.compare_tools) ?? "";
  const cardQueryBase = stripPreviewParams(
    base === "home" ? "/" : base === "tools" ? "/tools" : `/categories/${(base as { category: string }).category}`,
    queryBase,
  );

  const catalogCategoriesQuery = useQuery({
    queryKey: ["catalog-categories"],
    queryFn: getCategories,
    staleTime: 5 * 60 * 1000,
  });

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
    // Match home ISR (120s): hydrated SSR data should not immediately re-POST.
    staleTime: 120 * 1000,
    refetchOnMount: false,
  });

  const selectedSlug = params.selected;
  const previewQuery = useQuery({
    queryKey: ["preview-tool", selectedSlug],
    queryFn: () => getToolBySlug(selectedSlug!),
    enabled: !!selectedSlug,
  });

  const scopedCategoryCounts = useMemo(
    () =>
      browserQuery.data
        ? normalizeCategoryRows(browserQuery.data.categories as unknown[])
        : null,
    [browserQuery.data],
  );

  const categories = useMemo(() => {
    const labelSource =
      catalogCategoriesQuery.data?.length
        ? catalogCategoriesQuery.data
        : scopedCategoryCounts?.length
          ? scopedCategoryCounts
          : FUNCTION_CATEGORY_FALLBACK;
    return mergeCategoryCounts(labelSource, scopedCategoryCounts);
  }, [catalogCategoriesQuery.data, scopedCategoryCounts]);

  // Clear query filters on the current base; preserve sort. Category routes stay in-category.
  const emptyClearHref = buildQueryBase(
    base === "home" ? "home" : base,
    { sort: params.sort, page: 1 },
  );
  const emptyFilterLines = useMemo(
    () => describeActiveFilters(params, categories),
    [params, categories],
  );
  const emptyMessage = useMemo(
    () => buildEmptyIntersectionMessage(params, categories),
    [params, categories],
  );

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
  const typeX402 = buildQueryBase(base, forPricingFilter(params, "x402"));
  const x402Active = isX402FilterActive(params);
  const loadMoreHref = buildQueryBase(base, forNextPage(params));
  const closePreviewHref = withoutSelected(base, queryBase);
  const previewOpen = Boolean(selectedSlug && previewQuery.data);
  const layoutClass = previewOpen ? "tools-layout tools-layout-preview-open" : "tools-layout";

  useEffect(() => {
    if (!selectedSlug || !browserQuery.data?.tools.length) return;

    function onKeyDown(ev: KeyboardEvent) {
      if (isEditableTarget(ev.target)) return;

      const tools = browserQuery.data!.tools;
      const currentIndex = tools.findIndex((tool) => tool.slug === selectedSlug);
      if (currentIndex < 0) return;

      let nextIndex = -1;
      if (ev.key === "ArrowDown" || ev.key === "j") {
        nextIndex = currentIndex + 1;
      } else if (ev.key === "ArrowUp" || ev.key === "k") {
        nextIndex = currentIndex - 1;
      } else {
        return;
      }

      if (nextIndex < 0 || nextIndex >= tools.length) return;

      ev.preventDefault();
      router.push(withSelected(base, queryBase, tools[nextIndex].slug), { scroll: false });
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [base, browserQuery.data, queryBase, router, selectedSlug]);

  return (
    <div className={layoutClass} data-tools-browser="">
      {browserQuery.isLoading && !browserQuery.data ? (
        <>
          <Sidebar
            base={base}
            categories={categories}
            queryBase={filterQueryBase}
            filterRevision={filterRevision}
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
            filterRevision={filterRevision}
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
            filterRevision={filterRevision}
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
            <div className="tools-toolbar">
              {showToolbarSearch && (
                <ToolbarSearch base={base} initialQ={params.q ?? ""} />
              )}
              <div className="toolbar-desktop sticky-toolbar">
                <div className="toolbar-rows">
                  <div className="toolbar-sort-row">
                    <Link href={sortHot} scroll={false} prefetch={false} className={params.sort === "hot" ? "sort-link active" : "sort-link"}>HOT ↓</Link>
                    <Link href={sortNew} scroll={false} prefetch={false} className={params.sort === "new" ? "sort-link active" : "sort-link"}>New</Link>
                    <Link href={sortComments} scroll={false} prefetch={false} className={params.sort === "comments" ? "sort-link active" : "sort-link"}>Comments</Link>
                  </div>
                  <div className="toolbar-filter-row">
                    <span className="toolbar-filter-label">Filter:</span>
                    <Link href={typeMcp} scroll={false} prefetch={false} className={params.type === "mcp" ? "sort-link active" : "sort-link"}>MCP</Link>
                    <Link href={typeCli} scroll={false} prefetch={false} className={params.type === "cli" ? "sort-link active" : "sort-link"}>CLI</Link>
                    <Link href={typeApi} scroll={false} prefetch={false} className={params.type === "api" ? "sort-link active" : "sort-link"}>API</Link>
                    <Link href={typeSdk} scroll={false} prefetch={false} className={params.type === "sdk" ? "sort-link active" : "sort-link"}>SDK</Link>
                    <Link href={typeSkill} scroll={false} prefetch={false} className={params.type === "skill" ? "sort-link active" : "sort-link"}>Skill</Link>
                    <Link href={typeX402} scroll={false} prefetch={false} className={x402Active ? "sort-link active" : "sort-link"}>x402</Link>
                    <Link href={statusVerified} scroll={false} prefetch={false} className={params.status === "verified" ? "sort-link active" : "sort-link"}>Verified</Link>
                    <Link href={statusOfficial} scroll={false} prefetch={false} className={params.status === "official" ? "sort-link active" : "sort-link"}>Official</Link>
                  </div>
                </div>
                <span className="tool-count">{browserQuery.data.total} tools</span>
              </div>
              <MobileToolbarStrip
                sort={params.sort}
                sortHot={sortHot}
                sortNew={sortNew}
                sortComments={sortComments}
                typeHrefs={{
                  mcp: typeMcp,
                  cli: typeCli,
                  api: typeApi,
                  sdk: typeSdk,
                  skill: typeSkill,
                  x402: typeX402,
                }}
                statusHrefs={{
                  verified: statusVerified,
                  official: statusOfficial,
                }}
                activeType={params.type}
                x402Active={x402Active}
                activeStatus={params.status}
                toolCount={browserQuery.data.total}
              />
            </div>

            {browserQuery.data.tools.length === 0 ? (
              <EmptyState
                message={emptyMessage}
                filterLines={emptyFilterLines}
                clearHref={emptyClearHref}
                showX402Cta={
                  (params.type ?? "").includes("x402") ||
                  (params.pricing ?? "").includes("x402")
                }
              />
            ) : (
              <>
                <div className="tool-list">
                  {browserQuery.data.tools.map((tool) => {
                    const isSelected = selectedSlug === tool.slug;
                    const cardPreviewOpen = isSelected && previewOpen;
                    return (
                      <div
                        key={tool.slug}
                        className={
                          cardPreviewOpen
                            ? "tool-card-host tool-card-host--preview-open"
                            : "tool-card-host"
                        }
                        data-preview-open={cardPreviewOpen ? "" : undefined}
                      >
                        <ToolCard
                          tool={tool}
                          previewHref={withSelected(base, queryBase, tool.slug)}
                          queryBase={cardQueryBase}
                          isSelected={isSelected}
                          commentCount={browserQuery.data!.comment_counts[tool.slug] ?? 0}
                        />
                      </div>
                    );
                  })}
                </div>
                {shouldShowLoadMore(
                  browserQuery.data.tools.length,
                  browserQuery.data.total,
                  params.page,
                ) && (
                  <div className="load-more-row">
                    <Link href={loadMoreHref} scroll={false} prefetch={false} className="load-more-btn">
                      Load more
                    </Link>
                    <span className="load-more-count">
                      Showing {browserQuery.data.tools.length} of {browserQuery.data.total}
                    </span>
                  </div>
                )}
              </>
            )}

            {selectedSlug && previewQuery.isError && !previewQuery.isLoading && (
              <div
                className="preview-load-error"
                role="alert"
                data-testid="preview-load-error"
              >
                <p className="empty-state-message">
                  {previewQuery.error?.message ?? "Could not load tool preview."}
                </p>
                <Link href={closePreviewHref} scroll={false} prefetch={false} className="empty-state-clear-btn">
                  Close preview
                </Link>
              </div>
            )}

            {previewQuery.data && (
              <div className="preview-mobile">
                <BottomSheet
                  tool={previewQuery.data}
                  closeHref={closePreviewHref}
                  fullPageHref={`/tools/${previewQuery.data.slug}`}
                  commentCount={browserQuery.data.comment_counts[previewQuery.data.slug] ?? 0}
                  addMode={addMode}
                  addMcpQueryBase={queryBase}
                  compareReturnHref={compareBackHref}
                />
              </div>
            )}
          </div>
          {previewQuery.data && (
            <div className="preview-desktop">
              <PreviewPanel
                tool={previewQuery.data}
                closeHref={closePreviewHref}
                fullPageHref={`/tools/${previewQuery.data.slug}`}
                commentCount={browserQuery.data.comment_counts[previewQuery.data.slug] ?? 0}
                addMode={addMode}
                addMcpQueryBase={queryBase}
                compareReturnHref={compareBackHref}
              />
            </div>
          )}
        </>
      ) : null}
    </div>
  );
}