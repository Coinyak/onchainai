import type { ToolFilters } from "@/lib/api";

export type BrowserBase = "home" | "tools" | { category: string };

export const ADD_MCP_INTENT = "add-mcp";

export interface BrowserQueryParams {
  function?: string;
  asset_class?: string;
  actor?: string;
  type?: string;
  status?: string;
  pricing?: string;
  install_risk?: string;
  chain?: string;
  sort: string;
  q?: string;
  selected?: string;
  intent?: string;
  compare_tools?: string;
  page: number;
}

const SCALAR_KEYS = new Set(["q", "sort", "selected", "intent", "compare_tools"]);
export const TOOL_PAGE_SIZE = 50;
export const MAX_VISIBLE_TOOLS = 500;
export const MAX_BROWSER_PAGE = MAX_VISIBLE_TOOLS / TOOL_PAGE_SIZE;

export function browserBasePath(base: BrowserBase): string {
  if (base === "home") return "/";
  if (base === "tools") return "/tools";
  return `/categories/${base.category}`;
}

export function parseMulti(raw?: string | null): string[] {
  if (!raw) return [];
  const seen = new Set<string>();
  return raw
    .split(",")
    .map((p) => p.trim())
    .filter((p) => p && !seen.has(p) && seen.add(p));
}

export function encodeMulti(values: string[]): string | undefined {
  return values.length ? values.join(",") : undefined;
}

function decodeParam(v: string): string {
  try {
    return decodeURIComponent(v.replace(/\+/g, " "));
  } catch {
    return v;
  }
}

export function toggleMulti(
  basePath: string,
  queryBase: string,
  key: string,
  value: string,
  active: string[],
): string {
  const query = queryBase.replace(basePath, "").replace(/^\?/, "");
  const map = new Map<string, string[]>();

  for (const part of query.split("&").filter(Boolean)) {
    const [k, v] = part.split("=");
    if (!k || k === key) continue;
    const decoded = decodeParam(v ?? "");
    map.set(k, SCALAR_KEYS.has(k) ? [decoded] : parseMulti(decoded));
  }

  const next = [...active];
  const pos = next.indexOf(value);
  if (pos >= 0) next.splice(pos, 1);
  else {
    next.push(value);
    next.sort();
  }

  if (next.length) map.set(key, next);
  return buildFromMap(basePath, map);
}

export function clearAxis(basePath: string, queryBase: string, key: string): string {
  const query = queryBase.replace(basePath, "").replace(/^\?/, "");
  const map = new Map<string, string[]>();

  for (const part of query.split("&").filter(Boolean)) {
    const [k, v] = part.split("=");
    if (!k || k === key) continue;
    const decoded = decodeParam(v ?? "");
    map.set(k, SCALAR_KEYS.has(k) ? [decoded] : parseMulti(decoded));
  }
  return buildFromMap(basePath, map);
}

function buildFromMap(basePath: string, map: Map<string, string[]>): string {
  const parts: string[] = [];
  for (const [k, vals] of [...map.entries()].sort(([a], [b]) => a.localeCompare(b))) {
    if (SCALAR_KEYS.has(k)) {
      const v = vals[0];
      if (v) parts.push(`${k}=${encodeURIComponent(v)}`);
    } else {
      const encoded = encodeMulti(vals);
      if (encoded) parts.push(`${k}=${encodeURIComponent(encoded)}`);
    }
  }
  return parts.length ? `${basePath}?${parts.join("&")}` : basePath;
}

export function parsePageParam(raw?: string | null): number {
  if (!raw) return 1;
  const n = parseInt(raw, 10);
  if (!Number.isFinite(n) || n < 1) return 1;
  return Math.min(n, MAX_BROWSER_PAGE);
}

export function visibleLimitForPage(page: number): number {
  const p = Math.max(1, page);
  return Math.min(p * TOOL_PAGE_SIZE, MAX_VISIBLE_TOOLS);
}

export function shouldShowLoadMore(shown: number, total: number, page: number): boolean {
  if (shown >= total || shown >= MAX_VISIBLE_TOOLS) return false;
  const currentLimit = visibleLimitForPage(page);
  const nextLimit = visibleLimitForPage(page + 1);
  if (currentLimit >= MAX_VISIBLE_TOOLS && nextLimit === currentLimit) return false;
  return true;
}

export function buildToolFilters(params: BrowserQueryParams): ToolFilters {
  return {
    function: parseMulti(params.function),
    asset_class: parseMulti(params.asset_class),
    actor: parseMulti(params.actor),
    tool_type: parseMulti(params.type),
    status: parseMulti(params.status),
    pricing: parseMulti(params.pricing),
    install_risk: parseMulti(params.install_risk),
    chain: parseMulti(params.chain),
  };
}

function categoryFunctionFilter(base: BrowserBase, params: BrowserQueryParams): string | undefined {
  if (typeof base === "object") return undefined;
  return params.function;
}

export function buildQueryBase(base: BrowserBase, params: BrowserQueryParams): string {
  const path = browserBasePath(base);
  const map = new Map<string, string[]>();

  const fn = categoryFunctionFilter(base, params);
  if (fn) map.set("function", parseMulti(fn));
  if (params.asset_class) map.set("asset_class", parseMulti(params.asset_class));
  if (params.actor) map.set("actor", parseMulti(params.actor));
  if (params.type) map.set("type", parseMulti(params.type));
  if (params.status) map.set("status", parseMulti(params.status));
  if (params.pricing) map.set("pricing", parseMulti(params.pricing));
  if (params.install_risk) map.set("install_risk", parseMulti(params.install_risk));
  if (params.chain) map.set("chain", parseMulti(params.chain));
  if (params.sort && params.sort !== "hot") map.set("sort", [params.sort]);
  if (params.q?.trim()) map.set("q", [params.q.trim()]);
  if (params.selected) map.set("selected", [params.selected]);
  if (params.intent === ADD_MCP_INTENT) map.set("intent", [ADD_MCP_INTENT]);
  if (params.compare_tools?.trim()) map.set("compare_tools", [params.compare_tools.trim()]);
  if (params.page > 1) map.set("page", [String(params.page)]);

  return buildFromMap(path, map);
}

export function paramsFromSearchParams(
  base: BrowserBase,
  searchParams: URLSearchParams,
): BrowserQueryParams {
  const categoryFn = typeof base === "object" ? base.category : undefined;
  const intent = searchParams.get("intent") ?? undefined;
  return {
    function: categoryFn ?? searchParams.get("function") ?? undefined,
    asset_class: searchParams.get("asset_class") ?? undefined,
    actor: searchParams.get("actor") ?? undefined,
    type: searchParams.get("type") ?? undefined,
    status: searchParams.get("status") ?? undefined,
    pricing: searchParams.get("pricing") ?? undefined,
    install_risk: searchParams.get("install_risk") ?? undefined,
    chain: searchParams.get("chain") ?? undefined,
    sort: searchParams.get("sort") ?? "hot",
    q: searchParams.get("q") ?? undefined,
    selected: searchParams.get("selected") ?? undefined,
    intent: intent === ADD_MCP_INTENT ? ADD_MCP_INTENT : undefined,
    compare_tools: searchParams.get("compare_tools") ?? undefined,
    page: parsePageParam(searchParams.get("page")),
  };
}

export function forFilterNavigation(params: BrowserQueryParams): BrowserQueryParams {
  return { ...params, selected: undefined, intent: undefined, page: 1 };
}

export function forSort(params: BrowserQueryParams, sort: string): BrowserQueryParams {
  return { ...params, sort, selected: undefined, intent: undefined, page: 1 };
}

export function forStatusFilter(params: BrowserQueryParams, status?: string): BrowserQueryParams {
  const next =
    status && params.status === status ? undefined : status;
  return { ...params, status: next, selected: undefined, intent: undefined, page: 1 };
}

export function forTypeFilter(params: BrowserQueryParams, toolType?: string): BrowserQueryParams {
  const next = toolType && params.type === toolType ? undefined : toolType;
  return { ...params, type: next, selected: undefined, intent: undefined, page: 1 };
}

export function forNextPage(params: BrowserQueryParams): BrowserQueryParams {
  return { ...params, selected: undefined, intent: undefined, page: params.page + 1 };
}

export function stripPreviewParams(basePath: string, queryBase: string): string {
  const query = queryBase.startsWith(basePath)
    ? queryBase.slice(basePath.length).replace(/^\?/, "")
    : queryBase.replace(/^\?/, "");
  const parts = query
    .split("&")
    .filter(
      (p) =>
        p &&
        !p.startsWith("selected=") &&
        !p.startsWith("intent="),
    );
  return parts.length ? `${basePath}?${parts.join("&")}` : basePath;
}

export function withSelected(base: BrowserBase, queryBase: string, slug: string): string {
  const root = browserBasePath(base);
  const cleaned = stripPreviewParams(root, queryBase);
  const separator = cleaned.includes("?") ? "&" : "?";
  return `${cleaned}${separator}selected=${encodeURIComponent(slug)}`;
}

export function withoutSelected(base: BrowserBase, queryBase: string): string {
  return stripPreviewParams(browserBasePath(base), queryBase);
}

export function stripAddModeParams(queryBase: string): string {
  const path = queryBase.split("?")[0] || queryBase;
  return stripPreviewParams(path, queryBase);
}

export function addMcpHref(queryBase: string, slug: string): string {
  const base = stripAddModeParams(queryBase);
  const separator = base.includes("?") ? "&" : "?";
  return `${base}${separator}selected=${encodeURIComponent(slug)}&intent=${ADD_MCP_INTENT}`;
}

export function addMcpHrefFromCompare(compareSlugs: string[], toolSlug: string): string {
  const base =
    compareSlugs.length === 0
      ? "/tools"
      : `/tools?compare_tools=${encodeURIComponent(compareSlugs.join(","))}`;
  return addMcpHref(base, toolSlug);
}

export function compareHref(slugs: string[]): string {
  if (slugs.length === 0) return "/compare";
  return `/compare?tools=${encodeURIComponent(slugs.join(","))}`;
}

export function compareReturnHref(compareTools?: string): string | undefined {
  if (!compareTools?.trim()) return undefined;
  return `/compare?tools=${encodeURIComponent(compareTools.trim())}`;
}

export function buildFilterRevision(params: BrowserQueryParams): string {
  return [
    `f=${params.function ?? ""}`,
    `ac=${params.asset_class ?? ""}`,
    `a=${params.actor ?? ""}`,
    `t=${params.type ?? ""}`,
    `st=${params.status ?? ""}`,
    `p=${params.pricing ?? ""}`,
    `r=${params.install_risk ?? ""}`,
    `c=${params.chain ?? ""}`,
  ].join("|");
}

export function sidebarDefaultCollapsedForViewport(): boolean {
  if (typeof window === "undefined") return false;
  return window.innerWidth < 1024;
}

export function shouldCollapseMobileSidebarOnRouteChange(
  prevRoute: string,
  nextRoute: string,
  mobileViewport: boolean,
): boolean {
  return mobileViewport && prevRoute.length > 0 && prevRoute !== nextRoute;
}

export function categoryHref(catId: string, queryBase: string): string {
  const query = queryBase.includes("?") ? queryBase.split("?")[1] : "";
  const parts = query.split("&").filter((p) => p && !p.startsWith("function="));
  return parts.length ? `/categories/${catId}?${parts.join("&")}` : `/categories/${catId}`;
}