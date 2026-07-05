const API_URL = process.env.NEXT_PUBLIC_API_URL || "";

/** Browser fetch base — same-origin on Vercel preview when env points at production. */
export function clientApiBase(): string {
  if (typeof window === "undefined") return API_URL;
  if (!API_URL) return "";
  try {
    const resolved = new URL(API_URL, window.location.origin);
    if (resolved.origin !== window.location.origin) return "";
  } catch {
    return API_URL;
  }
  return API_URL;
}

function requestBase(): string {
  return typeof window === "undefined" ? API_URL : clientApiBase();
}

export async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const headers: Record<string, string> = {};
  if (options?.body && !(options.body instanceof FormData)) {
    headers["Content-Type"] = "application/json";
  }
  const res = await fetch(`${requestBase()}${path}`, {
    ...options,
    credentials: "include",
    headers: { ...headers, ...(options?.headers as Record<string, string>) },
  });
  if (!res.ok) {
    const error = await res.json().catch(() => ({ error: { message: "Request failed" } }));
    throw new Error(error.error?.message || "Request failed");
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

// --- Types ---

export interface Category {
  id: string;
  label: string;
  icon: string;
  description: string;
  sort_order: number;
}

export type CategoryRow = [Category, number];

export interface CategoryWithCount {
  category: Category;
  count: number;
}

export interface SessionUser {
  id: string;
  nickname: string | null;
  avatar_url: string | null;
  is_admin: boolean;
  auth_method: string;
}

/** Minimum fields for install/MCP affordances on cards and detail. */
export interface InstallSurfaceTool {
  slug: string;
  type: string;
  install_command?: string | null;
  safe_copy_command?: string | null;
  mcp_endpoint?: string | null;
}

/** Slim list/card payload from browser, search, and list endpoints. */
export interface PublicToolSummary {
  slug: string;
  name: string;
  description: string | null;
  type: string;
  function: string;
  chains: string[];
  install_risk_level: string;
  status: string;
  stars: number;
  pricing: string;
  claim_state: string;
  payment_verified: boolean;
  x402_endpoint_verified: boolean;
  referral_enabled: boolean;
  logo_url: string | null;
  logo_monogram: string | null;
  install_command: string | null;
  safe_copy_command: string | null;
  official_team: string | null;
  source: string;
  license: string | null;
  x402_price: string | null;
  last_commit_at: string | null;
  updated_at: string;
}

/** Public detail payload — operator/moderation fields omitted. */
export interface PublicTool {
  slug: string;
  name: string;
  description: string | null;
  function: string;
  asset_class: string;
  actor: string;
  type: string;
  repo_url: string | null;
  homepage: string | null;
  npm_package: string | null;
  install_command: string | null;
  safe_copy_command: string | null;
  mcp_endpoint: string | null;
  chains: string[];
  status: string;
  official_team: string | null;
  install_risk_level: string;
  install_risk_reasons: string[];
  requires_secret: boolean;
  claim_state: string;
  license: string | null;
  pricing: string;
  x402_price: string | null;
  referral_enabled: boolean;
  referral_bps: number | null;
  referral_model: string | null;
  payment_verified: boolean;
  x402_endpoint_verified: boolean;
  price_verified: boolean;
  stars: number;
  last_commit_at: string | null;
  source: string;
  source_url: string | null;
  logo_url: string | null;
  logo_monogram: string | null;
  created_at: string;
  updated_at: string;
}

export interface Tool {
  id: string;
  name: string;
  slug: string;
  description: string | null;
  function: string;
  asset_class: string;
  actor: string;
  type: string;
  repo_url: string | null;
  homepage: string | null;
  npm_package: string | null;
  install_command: string | null;
  mcp_endpoint: string | null;
  chains: string[];
  status: string;
  official_team: string | null;
  trust_score: number;
  approval_status?: string;
  crypto_relevance_score: number;
  crypto_relevance_reasons: string[];
  relevance_status: string;
  install_risk_level: string;
  install_risk_reasons: string[];
  requires_secret: boolean;
  safe_copy_command: string | null;
  claim_state: string;
  license: string | null;
  pricing: string;
  x402_price: string | null;
  referral_enabled?: boolean;
  referral_bps?: number | null;
  referral_model?: string | null;
  payment_verified: boolean;
  x402_endpoint_verified: boolean;
  price_verified: boolean;
  stars: number;
  last_commit_at: string | null;
  source: string;
  source_url: string | null;
  logo_url: string | null;
  logo_monogram: string | null;
  created_at: string;
  updated_at: string;
}

export interface ToolFilters {
  function?: string[];
  asset_class?: string[];
  actor?: string[];
  tool_type?: string[];
  status?: string[];
  pricing?: string[];
  install_risk?: string[];
  chain?: string[];
}

export interface ToolListRequest {
  sort: string;
  offset: number;
  limit: number;
  filters: ToolFilters;
  query?: string | null;
}

export interface BrowserDataPayload {
  categories: CategoryRow[];
  chains: [string, number][];
  total: number;
  tools: PublicToolSummary[];
  comment_counts: Record<string, number>;
  preview_tool: PublicTool | null;
}

export interface LoadBrowserDataRequest {
  sort: string;
  filters: ToolFilters;
  search_q?: string | null;
  selected?: string | null;
  page: number;
}

export interface FeaturedCard {
  id: string;
  tool_id: string;
  tool_slug: string;
  tool_name: string;
  image_url: string;
  headline: string | null;
  subtitle: string | null;
  sort_order: number;
}

export interface FooterLink {
  label: string;
  url: string;
}

export interface SiteSettings {
  id: number;
  site_name: string;
  slogan: string;
  description: string;
  mcp_endpoint: string;
  search_keywords: string[];
  allow_free_registration: boolean;
  require_tool_approval: boolean;
  allow_x402_registration: boolean;
  default_referral_bps?: number | null;
  default_referral_payout_address?: string | null;
  x402_builder_code?: string | null;
  mcp_premium_enabled: boolean;
  mcp_premium_pay_to_address?: string | null;
  mcp_premium_price?: string | null;
  mcp_premium_network: string;
  mcp_premium_asset?: string | null;
  mcp_premium_display_price?: string | null;
  hero_title?: string | null;
  hero_subtitle?: string | null;
  about_content?: string | null;
  footer_links: FooterLink[];
  updated_at: string;
}

export interface UpdateSiteSettingsPayload {
  site_name: string;
  slogan: string;
  description: string;
  mcp_endpoint: string;
  search_keywords_raw: string;
  allow_free_registration: boolean;
  require_tool_approval: boolean;
  allow_x402_registration: boolean;
  default_referral_bps?: number | null;
  default_referral_payout_address?: string | null;
  x402_builder_code?: string | null;
  mcp_premium_enabled: boolean;
  mcp_premium_pay_to_address?: string | null;
  mcp_premium_price?: string | null;
  mcp_premium_network: string;
  mcp_premium_asset?: string | null;
  mcp_premium_display_price?: string | null;
  hero_title?: string | null;
  hero_subtitle?: string | null;
  about_content?: string | null;
  footer_links: FooterLink[];
}

export interface TrustFact {
  label: string;
  detail: string;
  severity: string;
}

export interface ToolOfficialLink {
  id: string;
  tool_id: string;
  url: string;
  link_type: string;
  verification_status: string;
  evidence_strength: string;
  official_badge_allowed: boolean;
  verification_method: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface ToolTrustView {
  tool: Tool;
  official_links: ToolOfficialLink[];
  trust_facts: TrustFact[];
}

export interface CommentView {
  id: string;
  tool_id: string;
  parent_id: string | null;
  user_id: string;
  content: string;
  created_at: string;
  author_nickname: string | null;
  author_auth_method: string | null;
  author_is_admin: boolean;
  upvote_count: number;
  viewer_upvoted: boolean;
}

export interface DashboardBucket {
  id: string;
  label: string;
  count: number;
  href: string;
}

export interface DashboardMetrics {
  public_tools: number;
  mcp_tools: number;
  cli_tools: number;
  sdk_tools: number;
  api_tools: number;
  x402_tools: number;
  official_tools: number;
  verified_tools: number;
  updated_recently: number;
}

export interface PublicDashboardSnapshot {
  metrics: DashboardMetrics;
  type_counts: DashboardBucket[];
  function_counts: DashboardBucket[];
  chain_counts: DashboardBucket[];
  trust_counts: DashboardBucket[];
  pricing_counts: DashboardBucket[];
  new_tools: PublicToolSummary[];
  popular_tools: PublicToolSummary[];
  x402_tools: PublicToolSummary[];
  high_trust_tools: PublicToolSummary[];
  as_of: string;
}

export interface ToolkitToolView {
  tool: PublicTool;
  note: string | null;
  tags: string[];
  source?: string;
  source_client?: string | null;
  starred?: boolean;
  saved_at?: string;
  created_at?: string;
  updated_at: string;
}

export interface AgentTokenListItem {
  id: string;
  label: string;
  token_prefix: string;
  client: string;
  last_used_at: string | null;
  expires_at: string;
  revoked_at: string | null;
  created_at: string;
}

export interface AgentLinkStatus {
  linked: boolean;
}

export interface AgentDeviceApproveResult {
  ok: boolean;
  id: string;
  token_prefix: string;
  expires_at: string;
  message: string;
}

export interface MyToolkitPayload {
  items: ToolkitToolView[];
  exports: { claude_desktop: { filename: string; body: string }; cursor: { filename: string; body: string } };
}

export interface ToolComparisonView {
  tool: PublicTool;
  official_links: ToolOfficialLink[];
  trust_facts: TrustFact[];
  viewer_bookmarked: boolean;
}

export interface ToolSubmission {
  id: string;
  name: string;
  slug: string;
  status: string;
  created_at: string;
}

export interface AdminDashboardStats {
  pending_candidates: number;
  known_updates: number;
  high_risk_installs: number;
  open_reports: number;
  needs_manual_research: number;
  low_relevance: number;
  public_tools: number;
  crawler_sources: CrawlerSourceView[];
}

export interface CrawlerSourceView {
  id?: string | null;
  name: string;
  url: string;
  schedule: string;
  schedule_minutes: number;
  enabled: boolean;
  last_crawled_at: string | null;
  crawl_status: string;
  items_found: number;
  error_message: string | null;
}

export interface UpdateCrawlerSourcePayload {
  schedule_minutes: number;
  enabled: boolean;
}

export interface ReviewQueueItem {
  tool: Tool;
  queue_reason: string;
  priority: number;
}

export interface AdminUserView {
  id: string;
  nickname: string | null;
  auth_method: string;
  is_admin: boolean;
  is_banned: boolean;
  comment_count: number;
  bookmark_count: number;
  created_at: string;
}

export interface AdminCommentView {
  id: string;
  content: string;
  created_at: string;
  tool_slug: string;
  tool_name: string;
  author_nickname: string | null;
  author_auth_method: string | null;
}

export interface AdminCategoryView {
  category: Category;
  count: number;
}

// --- Helpers ---

export function normalizeCategoryRow(row: unknown): CategoryWithCount | null {
  if (Array.isArray(row) && row.length >= 2 && row[0] && typeof row[0] === "object") {
    const count = Number(row[1]);
    return { category: row[0] as Category, count: Number.isFinite(count) ? count : 0 };
  }
  if (row && typeof row === "object" && "id" in row && "label" in row && "count" in row) {
    const flat = row as Category & { count: number };
    const { count, ...category } = flat;
    const n = Number(count);
    return { category: category as Category, count: Number.isFinite(n) ? n : 0 };
  }
  return null;
}

function normalizeCategory(row: CategoryRow): CategoryWithCount {
  return normalizeCategoryRow(row) ?? { category: row[0], count: row[1] };
}

export function normalizeCategoryRows(rows: unknown[]): CategoryWithCount[] {
  return rows
    .map((row) => normalizeCategoryRow(row))
    .filter((row): row is CategoryWithCount => row !== null && Boolean(row.category.id));
}

function filtersToQuery(filters: ToolFilters): URLSearchParams {
  const search = new URLSearchParams();
  const mapping: [string, string[] | undefined][] = [
    ["function", filters.function],
    ["asset_class", filters.asset_class],
    ["actor", filters.actor],
    ["type", filters.tool_type],
    ["status", filters.status],
    ["pricing", filters.pricing],
    ["install_risk", filters.install_risk],
    ["chain", filters.chain],
  ];
  for (const [key, values] of mapping) {
    if (values?.length) search.set(key, values.join(","));
  }
  return search;
}

// --- Auth ---

export async function getMe(): Promise<SessionUser | null> {
  return apiFetch<SessionUser | null>("/api/v2/me");
}

export async function checkAdminAccess(): Promise<boolean> {
  try {
    await apiFetch<{ ok: boolean }>("/api/v2/admin/check");
    return true;
  } catch {
    return false;
  }
}

// --- Public catalog ---

export async function getCategories(): Promise<CategoryWithCount[]> {
  const rows = await apiFetch<CategoryRow[]>("/api/v2/categories");
  return rows.map(normalizeCategory);
}

export async function getToolBySlug(slug: string): Promise<PublicTool> {
  return apiFetch<PublicTool>(`/api/v2/tools/${encodeURIComponent(slug)}`);
}

/** N1 related tools — returns [] until API is available. */
export async function getRelatedTools(slug: string, limit = 4): Promise<Tool[]> {
  try {
    const tools = await apiFetch<Tool[]>(
      `/api/v2/tools/${encodeURIComponent(slug)}/related?limit=${limit}`,
    );
    return Array.isArray(tools) ? tools : [];
  } catch {
    return [];
  }
}

export async function getToolTrustView(slug: string): Promise<ToolTrustView> {
  return apiFetch<ToolTrustView>(`/api/v2/admin/trust/${encodeURIComponent(slug)}`);
}

export async function searchTools(params: {
  query: string;
  function?: string;
  chain?: string;
  page_size?: number;
}): Promise<PublicToolSummary[]> {
  const search = new URLSearchParams({ query: params.query });
  if (params.function) search.set("function", params.function);
  if (params.chain) search.set("chain", params.chain);
  if (params.page_size != null) search.set("page_size", String(params.page_size));
  const tools = await apiFetch<PublicToolSummary[]>(`/api/v2/tools/search?${search.toString()}`);
  if (params.page_size != null && params.page_size > 0) {
    return tools.slice(0, params.page_size);
  }
  return tools;
}

export async function getRecentTools(limit = 10): Promise<Tool[]> {
  return apiFetch<Tool[]>(`/api/v2/tools/recent?limit=${limit}`);
}

export async function listTools(req: ToolListRequest): Promise<PublicToolSummary[]> {
  return apiFetch<PublicToolSummary[]>("/api/v2/tools/list", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export async function countTools(filters: ToolFilters = {}): Promise<number> {
  const qs = filtersToQuery(filters).toString();
  return apiFetch<number>(`/api/v2/tools/count${qs ? `?${qs}` : ""}`);
}

export async function getChainCounts(limit = 12): Promise<[string, number][]> {
  return apiFetch<[string, number][]>(`/api/v2/chains?limit=${limit}`);
}

export async function loadBrowserData(req: LoadBrowserDataRequest): Promise<BrowserDataPayload> {
  return apiFetch<BrowserDataPayload>("/api/v2/browser-data", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export async function getFeaturedCards(): Promise<FeaturedCard[]> {
  return apiFetch<FeaturedCard[]>("/api/v2/featured");
}

export async function getSiteSettings(): Promise<SiteSettings> {
  return apiFetch<SiteSettings>("/api/v2/settings");
}

export async function getPublicDashboard(limit = 12): Promise<PublicDashboardSnapshot> {
  return apiFetch<PublicDashboardSnapshot>(`/api/v2/dashboard?limit=${limit}`);
}

export async function compareTools(slugs: string[]): Promise<ToolComparisonView[]> {
  return apiFetch<ToolComparisonView[]>(
    `/api/v2/tools/compare?slugs=${encodeURIComponent(slugs.join(","))}`,
  );
}

export async function getToolCommentCounts(slugs: string[]): Promise<Record<string, number>> {
  if (!slugs.length) return {};
  return apiFetch<Record<string, number>>(
    `/api/v2/tools/comment-counts?slugs=${encodeURIComponent(slugs.join(","))}`,
  );
}

export async function getToolCommentCount(slug: string): Promise<number> {
  return apiFetch<number>(`/api/v2/tools/${encodeURIComponent(slug)}/comment-count`);
}

// --- Comments & bookmarks ---

export async function getToolComments(slug: string, sort = "new"): Promise<CommentView[]> {
  return apiFetch<CommentView[]>(
    `/api/v2/tools/${encodeURIComponent(slug)}/comments?sort=${sort}`,
  );
}

export async function createComment(
  slug: string,
  content: string,
  parentId?: string | null,
): Promise<CommentView> {
  return apiFetch<CommentView>(`/api/v2/tools/${encodeURIComponent(slug)}/comments`, {
    method: "POST",
    body: JSON.stringify({ content, parent_id: parentId ?? null }),
  });
}

export async function toggleUpvote(commentId: string): Promise<{ upvoted: boolean }> {
  return apiFetch<{ upvoted: boolean }>(`/api/v2/comments/${commentId}/upvote`, {
    method: "POST",
  });
}

export async function isBookmarked(slug: string): Promise<boolean> {
  const res = await apiFetch<{ starred: boolean }>(
    `/api/v2/tools/${encodeURIComponent(slug)}/bookmark`,
  );
  return res.starred;
}

export async function setBookmark(slug: string, starred: boolean): Promise<void> {
  await apiFetch(`/api/v2/tools/${encodeURIComponent(slug)}/bookmark`, {
    method: "PUT",
    body: JSON.stringify({ starred }),
  });
}

export async function toggleBookmark(slug: string): Promise<{ starred: boolean }> {
  return apiFetch<{ starred: boolean }>(
    `/api/v2/tools/${encodeURIComponent(slug)}/bookmark`,
    { method: "POST" },
  );
}

// --- Blueprints ---

export interface BlueprintNode {
  id: string;
  kind: "tool" | "note" | "chain";
  slug?: string;
  chainId?: string;
  /** Tool nodes only: chains selected for this blueprint annotation. */
  chains?: string[];
  text?: string;
  x: number;
  y: number;
}

export interface BlueprintEdge {
  id: string;
  fromId: string;
  toId: string;
  style: "solid" | "arrow";
  color: string;
}

export interface BlueprintListItem {
  id: string;
  title: string;
  node_count: number;
  updated_at: string;
}

export interface Blueprint {
  id: string;
  title: string;
  nodes: BlueprintNode[];
  edges: BlueprintEdge[];
  created_at: string;
  updated_at: string;
}

export async function listBlueprints(): Promise<BlueprintListItem[]> {
  return apiFetch<BlueprintListItem[]>("/api/v2/blueprints");
}

export async function createBlueprint(payload: {
  title?: string;
  nodes?: BlueprintNode[];
  edges?: BlueprintEdge[];
}): Promise<Blueprint> {
  return apiFetch<Blueprint>("/api/v2/blueprints", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function getBlueprint(id: string): Promise<Blueprint> {
  return apiFetch<Blueprint>(`/api/v2/blueprints/${encodeURIComponent(id)}`);
}

export async function updateBlueprint(
  id: string,
  payload: { title?: string; nodes?: BlueprintNode[]; edges?: BlueprintEdge[] },
): Promise<Blueprint> {
  return apiFetch<Blueprint>(`/api/v2/blueprints/${encodeURIComponent(id)}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteBlueprint(id: string): Promise<void> {
  await apiFetch(`/api/v2/blueprints/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });
}

export interface BlueprintAgentExport {
  title: string;
  markdown: string;
  slugs: string[];
  filename: string;
}

export async function getBlueprintAgentExport(id: string): Promise<BlueprintAgentExport> {
  return apiFetch<BlueprintAgentExport>(
    `/api/v2/blueprints/${encodeURIComponent(id)}/agent-export`,
  );
}

// --- Agent Sync ---

export async function getAgentLinkStatus(): Promise<AgentLinkStatus> {
  return apiFetch<AgentLinkStatus>("/api/v2/agent/link-status");
}

export async function listAgentTokens(): Promise<{ items: AgentTokenListItem[] }> {
  return apiFetch<{ items: AgentTokenListItem[] }>("/api/v2/agent/tokens");
}

export async function approveAgentDevice(userCode: string, label?: string): Promise<AgentDeviceApproveResult> {
  return apiFetch<AgentDeviceApproveResult>("/api/v2/agent/device/approve", {
    method: "POST",
    body: JSON.stringify({ user_code: userCode, label }),
  });
}

export async function revokeAgentToken(id: string): Promise<void> {
  await apiFetch(`/api/v2/agent/tokens/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });
}

// --- Toolkit ---

export async function listMyToolkit(): Promise<MyToolkitPayload> {
  return apiFetch<MyToolkitPayload>("/api/v2/toolkit");
}

export async function updateToolkitItem(
  slug: string,
  payload: { note?: string | null; tags?: string[]; starred?: boolean },
): Promise<void> {
  await apiFetch(`/api/v2/toolkit/${encodeURIComponent(slug)}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

// --- Submissions ---

export async function submitTool(payload: Record<string, unknown>): Promise<ToolSubmission> {
  return apiFetch<ToolSubmission>("/api/v2/submit", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listMySubmissions(): Promise<ToolSubmission[]> {
  return apiFetch<ToolSubmission[]>("/api/v2/my-submissions");
}

// --- x402 open listing ---

export const X402_LISTING_TERMS_VERSION = "x402-open-listing-v1";

export const X402_REFERRAL_DISCLOSURE =
  "OnchainAI may receive referral fees from tools discovered through this directory. We never process payments or hold funds — attribution metadata only.";

export interface X402ProbeDetails {
  amount: string | null;
  asset: string | null;
  network: string | null;
  pay_to: string | null;
  description: string | null;
}

export interface X402ProbeResponse {
  live: boolean;
  status: string;
  details: X402ProbeDetails | null;
  reason: string | null;
}

export interface X402SubmitResponse {
  published: boolean;
  slug: string | null;
  tool_id: string | null;
  submission_id: string | null;
  probe: X402ProbeResponse;
}

export async function probeX402Endpoint(url: string): Promise<X402ProbeResponse> {
  return apiFetch<X402ProbeResponse>("/api/v2/x402/probe", {
    method: "POST",
    body: JSON.stringify({ url }),
  });
}

export async function submitX402Listing(payload: {
  name: string;
  description: string;
  endpoint_url: string;
  homepage?: string | null;
  repo_url?: string | null;
}): Promise<X402SubmitResponse> {
  return apiFetch<X402SubmitResponse>("/api/v2/x402/submit", {
    method: "POST",
    body: JSON.stringify({
      ...payload,
      terms_version: X402_LISTING_TERMS_VERSION,
      terms_accepted: true,
    }),
  });
}

export async function reportTool(slug: string, reason: string): Promise<void> {
  await apiFetch(`/api/v2/tools/${encodeURIComponent(slug)}/report`, {
    method: "POST",
    body: JSON.stringify({ reason }),
  });
}

export async function requestToolClaim(slug: string, payload: Record<string, unknown>): Promise<void> {
  await apiFetch(`/api/v2/tools/${encodeURIComponent(slug)}/claim`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

// --- Admin ---

export async function getAdminStats(): Promise<AdminDashboardStats> {
  return apiFetch<AdminDashboardStats>("/api/v2/admin/stats");
}

export async function getReviewQueue(queue: string, limit = 50): Promise<ReviewQueueItem[]> {
  return apiFetch<ReviewQueueItem[]>(
    `/api/v2/admin/review-queue?queue=${encodeURIComponent(queue)}&limit=${limit}`,
  );
}

export async function reviewTool(payload: Record<string, unknown>): Promise<void> {
  await apiFetch("/api/v2/admin/review", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listAdminUsers(): Promise<AdminUserView[]> {
  return apiFetch<AdminUserView[]>("/api/v2/admin/users");
}

export async function setUserBanned(userId: string, banned: boolean): Promise<void> {
  await apiFetch(`/api/v2/admin/users/${userId}/ban`, {
    method: "PUT",
    body: JSON.stringify({ banned }),
  });
}

export async function setUserAdmin(userId: string, isAdmin: boolean): Promise<void> {
  await apiFetch(`/api/v2/admin/users/${userId}/admin`, {
    method: "PUT",
    body: JSON.stringify({ is_admin: isAdmin }),
  });
}

export async function deleteUser(userId: string): Promise<void> {
  await apiFetch(`/api/v2/admin/users/${userId}`, { method: "DELETE" });
}

export async function listAdminComments(): Promise<AdminCommentView[]> {
  return apiFetch<AdminCommentView[]>("/api/v2/admin/comments");
}

export async function deleteAdminComment(commentId: string): Promise<void> {
  await apiFetch(`/api/v2/admin/comments/${commentId}`, { method: "DELETE" });
}

export async function deleteCommentAndBanUser(commentId: string): Promise<void> {
  await apiFetch(`/api/v2/admin/comments/${commentId}/ban-author`, { method: "DELETE" });
}

export async function listAdminCategories(): Promise<AdminCategoryView[]> {
  const rows = await apiFetch<CategoryRow[]>("/api/v2/admin/categories");
  return rows.map((row) => ({ category: row[0], count: row[1] }));
}

export async function createCategory(payload: { id: string; label: string; icon: string }): Promise<Category> {
  return apiFetch<Category>("/api/v2/admin/categories", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateCategory(id: string, payload: Partial<Category>): Promise<Category> {
  return apiFetch<Category>(`/api/v2/admin/categories/${encodeURIComponent(id)}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteCategory(id: string): Promise<void> {
  await apiFetch(`/api/v2/admin/categories/${encodeURIComponent(id)}`, { method: "DELETE" });
}

export async function listFeaturedCardsAdmin(): Promise<FeaturedCard[]> {
  return apiFetch<FeaturedCard[]>("/api/v2/admin/featured");
}

export async function getAdminSiteSettings(): Promise<SiteSettings> {
  return apiFetch<SiteSettings>("/api/v2/admin/settings");
}

export async function updateSiteSettings(
  payload: UpdateSiteSettingsPayload,
): Promise<SiteSettings> {
  return apiFetch<SiteSettings>("/api/v2/admin/settings", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function listCrawlerSources(): Promise<CrawlerSourceView[]> {
  return apiFetch<CrawlerSourceView[]>("/api/v2/admin/crawler/sources");
}

export async function updateCrawlerSource(
  id: string,
  payload: UpdateCrawlerSourcePayload,
): Promise<unknown> {
  return apiFetch(`/api/v2/admin/crawler/sources/${encodeURIComponent(id)}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function triggerCrawlerSource(source: string): Promise<{ ok: boolean }> {
  return apiFetch<{ ok: boolean }>("/api/v2/admin/crawler/trigger", {
    method: "POST",
    body: JSON.stringify({ source }),
  });
}

export const API_BASE = API_URL;