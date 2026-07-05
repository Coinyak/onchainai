use super::*;

/// Returns all function categories with live **approved** tool counts.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_categories(pool: &sqlx::PgPool) -> Result<Vec<(Category, i64)>, FnError> {
    fetch_filtered_category_counts(pool, &ToolFilters::default()).await
}

/// Per-function counts for the sidebar, respecting active filters (function axis excluded).
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_filtered_category_counts(
    pool: &sqlx::PgPool,
    filters: &ToolFilters,
) -> Result<Vec<(Category, i64)>, FnError> {
    let mut facet_filters = filters.clone();
    facet_filters.function.clear();

    let mut q = sqlx::QueryBuilder::new(
        "SELECT c.id, c.label, c.icon, c.description, c.sort_order, COUNT(t.id)::bigint AS count \
         FROM categories c \
         LEFT JOIN tools t ON t.function = c.id AND ",
    );
    q.push(PUBLIC_TOOL_WHERE);
    append_tool_filters(&mut q, &facet_filters);
    q.push(
        " GROUP BY c.id, c.label, c.icon, c.description, c.sort_order \
          ORDER BY c.sort_order ASC",
    );

    let rows = q
        .build_query_as::<CategoryWithCount>()
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("failed to load filtered category counts: {e}")))?;

    Ok(rows.into_iter().map(CategoryWithCount::into_pair).collect())
}

/// Fetch a single **approved** tool by slug, if present.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_tool_by_slug(
    pool: &sqlx::PgPool,
    slug: &str,
) -> Result<Option<Tool>, FnError> {
    let tool = sqlx::query_as::<_, Tool>(APPROVED_TOOL_BY_SLUG_SQL)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(|e| FnError::new(format!("failed to load tool: {e}")))?;

    Ok(tool.map(sanitize_tool_for_public_response))
}

/// Maximum length for a tool slug accepted at server boundaries.
const MAX_SLUG_LEN: usize = 128;

/// Validate an externally-supplied slug at the server boundary: non-empty,
/// bounded length, allowed charset (URL-safe slug characters only).
fn validate_slug(slug: &str) -> Result<(), FnError> {
    let slug = slug.trim();
    if slug.is_empty() {
        return Err(FnError::new("slug must not be empty"));
    }
    if slug.len() > MAX_SLUG_LEN {
        return Err(FnError::new(format!(
            "slug must be at most {MAX_SLUG_LEN} characters"
        )));
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(FnError::new(
            "slug contains invalid characters; only a-z, 0-9, '-', '_', '.' allowed",
        ));
    }
    Ok(())
}

/// Fetch an approved tool and build its public install guide (shared server/MCP path).
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_public_install_guide(
    pool: &sqlx::PgPool,
    slug: &str,
    platform: &str,
) -> Result<crate::public_install_guide::PublicInstallGuide, FnError> {
    validate_slug(slug)?;
    let tool = fetch_tool_by_slug(pool, slug)
        .await?
        .ok_or_else(|| FnError::new(format!("tool not found: {slug}")))?;
    crate::public_install_guide::build_install_guide_for_platform(&tool, slug, platform)
        .map_err(FnError::new)
}

/// Maximum tools returned by `list_tools` / browser "load more" (matches UI cap).
pub const MAX_LIST_TOOLS_LIMIT: i64 = 500;

/// Clamp list-tools `limit` to the browser cap (500), never the legacy 100 ceiling.
pub(crate) fn clamp_list_tools_limit(limit: i64) -> i64 {
    limit.clamp(1, MAX_LIST_TOOLS_LIMIT)
}

const MAX_TOOL_FILTER_VALUES: usize = 20;
const MAX_TOOL_FILTER_VALUE_LEN: usize = 64;
const MAX_TOOL_LIST_QUERY_LEN: usize = 200;

pub(crate) fn validate_search_tools_input(
    query: &str,
    function: &Option<String>,
    chain: &Option<String>,
) -> Result<(), FnError> {
    if query.trim().is_empty() {
        return Err(FnError::new("query must not be empty"));
    }
    if query.len() > MAX_TOOL_LIST_QUERY_LEN {
        return Err(FnError::new(format!(
            "query must be at most {MAX_TOOL_LIST_QUERY_LEN} characters"
        )));
    }
    if let Some(filter) = function {
        validate_tool_filter_values("function", std::slice::from_ref(filter))?;
    }
    if let Some(filter) = chain {
        validate_tool_filter_values("chain", std::slice::from_ref(filter))?;
    }
    Ok(())
}

fn validate_tool_filter_values(axis: &str, values: &[String]) -> Result<(), FnError> {
    if values.len() > MAX_TOOL_FILTER_VALUES {
        return Err(FnError::new(format!(
            "filter `{axis}` accepts at most {MAX_TOOL_FILTER_VALUES} values"
        )));
    }
    for value in values {
        if value.len() > MAX_TOOL_FILTER_VALUE_LEN {
            return Err(FnError::new(format!(
                "filter `{axis}` values must be at most {MAX_TOOL_FILTER_VALUE_LEN} characters"
            )));
        }
    }
    Ok(())
}

/// Validates multi-axis tool filters for list/count queries.
pub fn validate_tool_filters(filters: &ToolFilters) -> Result<(), FnError> {
    validate_tool_filter_values("function", &filters.function)?;
    validate_tool_filter_values("asset_class", &filters.asset_class)?;
    validate_tool_filter_values("actor", &filters.actor)?;
    validate_tool_filter_values("tool_type", &filters.tool_type)?;
    validate_tool_filter_values("status", &filters.status)?;
    validate_tool_filter_values("pricing", &filters.pricing)?;
    validate_tool_filter_values("install_risk", &filters.install_risk)?;
    validate_tool_filter_values("chain", &filters.chain)?;
    Ok(())
}

fn validate_tool_sort(sort: &str) -> Result<(), FnError> {
    matches!(sort, "hot" | "new" | "comments")
        .then_some(())
        .ok_or_else(|| FnError::new("sort must be one of: hot, new, comments"))
}

/// Validates browser tool-list request bounds (rejects out-of-range instead of clamping).
pub fn validate_tool_list_request(req: &ToolListRequest) -> Result<(), FnError> {
    validate_tool_sort(&req.sort)?;
    validate_tool_filters(&req.filters)?;
    if req.offset < 0 {
        return Err(FnError::new("offset must be >= 0"));
    }
    if !(1..=MAX_LIST_TOOLS_LIMIT).contains(&req.limit) {
        return Err(FnError::new(format!(
            "limit must be between 1 and {MAX_LIST_TOOLS_LIMIT}"
        )));
    }
    if let Some(query) = req.query.as_ref() {
        if query.len() > MAX_TOOL_LIST_QUERY_LEN {
            return Err(FnError::new(format!(
                "query must be at most {MAX_TOOL_LIST_QUERY_LEN} characters"
            )));
        }
    }
    Ok(())
}

/// Stable request payload for browser tool-list queries (avoids positional arg drift).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolListRequest {
    pub sort: String,
    pub offset: i64,
    pub limit: i64,
    pub filters: ToolFilters,
    pub query: Option<String>,
}

/// Optional axis filters for tool list/count queries (AND across axes and within each axis).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolFilters {
    #[serde(default)]
    pub function: Vec<String>,
    #[serde(default)]
    pub asset_class: Vec<String>,
    #[serde(default)]
    pub actor: Vec<String>,
    #[serde(default)]
    pub tool_type: Vec<String>,
    #[serde(default)]
    pub status: Vec<String>,
    #[serde(default)]
    pub pricing: Vec<String>,
    #[serde(default)]
    pub install_risk: Vec<String>,
    #[serde(default)]
    pub chain: Vec<String>,
}

#[cfg(feature = "ssr")]
fn append_scalar_union<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    column: &str,
    values: &'qb [String],
) {
    if values.is_empty() {
        return;
    }
    query.push(" AND ");
    if values.len() == 1 {
        query.push(column).push(" = ").push_bind(&values[0]);
        return;
    }
    query.push("(");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            query.push(" OR ");
        }
        query.push(column).push(" = ").push_bind(value);
    }
    query.push(")");
}

/// x402 catalog slice: matches dashboard snapshot semantics (not only `type`/`pricing` = x402).
#[cfg(feature = "ssr")]
fn append_x402_catalog_predicate(query: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>) {
    query.push(
        " AND (type = 'x402' OR pricing = 'x402' OR x402_price IS NOT NULL OR referral_enabled = true)",
    );
}

#[cfg(feature = "ssr")]
fn append_scalar_union_except<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    column: &str,
    values: &'qb [String],
    skip: &str,
) {
    let kept: Vec<&'qb String> = values
        .iter()
        .filter(|value| value.as_str() != skip)
        .collect();
    if kept.is_empty() {
        return;
    }
    query.push(" AND ");
    if kept.len() == 1 {
        query.push(column).push(" = ").push_bind(kept[0]);
        return;
    }
    query.push("(");
    for (index, value) in kept.iter().enumerate() {
        if index > 0 {
            query.push(" OR ");
        }
        query.push(column).push(" = ").push_bind(*value);
    }
    query.push(")");
}

#[cfg(feature = "ssr")]
fn filters_include_x402(filters: &ToolFilters) -> bool {
    filters.tool_type.iter().any(|value| value == "x402")
        || filters.pricing.iter().any(|value| value == "x402")
}

#[cfg(feature = "ssr")]
pub(crate) fn append_tool_filters<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    filters: &'qb ToolFilters,
) {
    append_scalar_union(query, "function", &filters.function);
    append_scalar_union(query, "asset_class", &filters.asset_class);
    append_scalar_union(query, "actor", &filters.actor);
    append_scalar_union_except(query, "type", &filters.tool_type, "x402");
    append_scalar_union(query, "status", &filters.status);
    append_scalar_union_except(query, "pricing", &filters.pricing, "x402");
    append_scalar_union(query, "install_risk_level", &filters.install_risk);
    if filters_include_x402(filters) {
        append_x402_catalog_predicate(query);
    }
    if !filters.chain.is_empty() {
        // Intersection: tool must support every selected chain.
        query.push(" AND chains @> ").push_bind(&filters.chain);
    }
}

#[cfg(feature = "ssr")]
fn push_list_query_filter<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    search: Option<&'qb str>,
    match_mode: crate::server::tool_search::ToolSearchMatch,
) {
    let Some(text) = search.filter(|q| !q.trim().is_empty()) else {
        return;
    };
    use crate::server::tool_search::{ToolSearchMatch, TOOL_SEARCH_VECTOR};
    query.push(" AND ").push(TOOL_SEARCH_VECTOR).push(" @@ ");
    match match_mode {
        ToolSearchMatch::And => {
            query.push("plainto_tsquery('english', ");
            query.push_bind(text);
            query.push(")");
        }
        ToolSearchMatch::Or => {
            query.push("to_tsquery('english', replace(plainto_tsquery('english', ");
            query.push_bind(text);
            query.push(")::text, ' & ', ' | '))");
        }
    }
}

#[cfg(feature = "ssr")]
fn push_list_order_offset_limit(
    query: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>,
    sort: &str,
    offset: i64,
    limit: i64,
) {
    match sort {
        "new" => {
            query.push(" ORDER BY created_at DESC");
        }
        "comments" => {
            query.push(
                " ORDER BY \
                 (SELECT COUNT(*)::bigint FROM comments cm WHERE cm.tool_id = tools.id) DESC, \
                 created_at DESC",
            );
        }
        _ => {
            query.push(" ORDER BY stars DESC, created_at DESC");
        }
    }
    query.push(" OFFSET ").push_bind(offset);
    query.push(" LIMIT ").push_bind(limit);
}

/// Count approved tools with optional multi-axis filters.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_count_tools(
    pool: &sqlx::PgPool,
    filters: &ToolFilters,
) -> Result<i64, FnError> {
    let mut q = sqlx::QueryBuilder::new(COUNT_APPROVED_TOOLS_SQL);
    append_tool_filters(&mut q, filters);

    let count = q
        .build_query_as::<(i64,)>()
        .fetch_one(pool)
        .await
        .map_err(|e| FnError::new(format!("count failed: {e}")))?;

    Ok(count.0)
}

/// Top chains by approved-tool count for sidebar filters.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_chain_counts(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<Vec<(String, i64)>, FnError> {
    let limit = limit.clamp(1, 100);
    let rows = sqlx::query_as::<_, (String, i64)>(CHAIN_COUNTS_SQL)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("chain counts failed: {e}")))?;

    Ok(rows)
}

/// List approved tools with sort, pagination, FTS query, and optional filters.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_list_tools(
    pool: &sqlx::PgPool,
    sort: &str,
    offset: i64,
    limit: i64,
    filters: &ToolFilters,
    query: Option<&str>,
) -> Result<Vec<Tool>, FnError> {
    use crate::server::queries::{MCP_SEARCH_TOOLS_COUNT_OR_SQL, MCP_SEARCH_TOOLS_COUNT_SQL};
    use crate::server::tool_search::{resolve_search_match, ToolSearchMatch};

    let offset = offset.max(0);
    let limit = clamp_list_tools_limit(limit);
    let search_text = query.filter(|q| !q.trim().is_empty());
    let match_mode = if let Some(text) = search_text {
        resolve_search_match(
            pool,
            MCP_SEARCH_TOOLS_COUNT_SQL,
            MCP_SEARCH_TOOLS_COUNT_OR_SQL,
            text,
            None,
            None,
        )
        .await
        .map_err(|e| FnError::new(format!("search match resolve failed: {e}")))?
    } else {
        ToolSearchMatch::And
    };
    let mut q = sqlx::QueryBuilder::new(LIST_APPROVED_TOOLS_SQL);
    push_list_query_filter(&mut q, query, match_mode);
    append_tool_filters(&mut q, filters);
    push_list_order_offset_limit(&mut q, sort, offset, limit);

    let tools = q
        .build_query_as::<Tool>()
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("list tools failed: {e}")))?;

    Ok(sanitize_tools_for_public_response(tools))
}

/// Tools browser page size (must match `tools_browser::TOOL_PAGE_SIZE`).
pub const BROWSER_TOOL_PAGE_SIZE: u32 = 50;

/// Clamp browser `page` query param to the UI-visible window.
pub fn clamp_browser_page_param(page: u32) -> u32 {
    let max_page = (MAX_LIST_TOOLS_LIMIT as u32) / BROWSER_TOOL_PAGE_SIZE;
    page.max(1).min(max_page)
}

/// Cumulative list limit for browser pagination (`offset` always 0).
pub fn browser_visible_limit_for_page(page: u32) -> i64 {
    let page = clamp_browser_page_param(page);
    let limit = page
        .saturating_mul(BROWSER_TOOL_PAGE_SIZE)
        .min(MAX_LIST_TOOLS_LIMIT as u32);
    i64::from(limit)
}

/// Single RPC payload for `ToolsBrowser` — avoids client-side fan-out into 5–6 `/api` calls.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowserDataPayload {
    pub categories: Vec<(Category, i64)>,
    pub chains: Vec<(String, i64)>,
    pub total: i64,
    pub tools: Vec<Tool>,
    pub comment_counts: HashMap<String, i64>,
    pub preview_tool: Option<Tool>,
}

pub const MAX_DASHBOARD_LIST_LIMIT: i64 = 12;

pub fn clamp_dashboard_list_limit(limit: i64) -> i64 {
    limit.clamp(1, MAX_DASHBOARD_LIST_LIMIT)
}

fn encoded_query_value(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

pub fn dashboard_filter_href(axis: &str, value: &str) -> String {
    let value = encoded_query_value(value);
    match axis {
        "function" => format!("/tools?function={value}"),
        "type" => format!("/tools?type={value}"),
        "chain" => format!("/tools?chain={value}"),
        "status" => format!("/tools?status={value}"),
        "pricing" => format!("/tools?pricing={value}"),
        _ => "/tools".into(),
    }
}

fn dashboard_label(axis: &str, value: &str) -> String {
    match axis {
        "type" if value.eq_ignore_ascii_case("mcp") => "MCP".into(),
        "type" if value.eq_ignore_ascii_case("cli") => "CLI".into(),
        "type" if value.eq_ignore_ascii_case("sdk") => "SDK".into(),
        "type" if value.eq_ignore_ascii_case("api") => "API".into(),
        "type" if value.eq_ignore_ascii_case("x402") => "x402".into(),
        "status" if value.eq_ignore_ascii_case("official") => "Official".into(),
        "status" if value.eq_ignore_ascii_case("verified") => "Verified".into(),
        "status" if value.eq_ignore_ascii_case("community") => "Community".into(),
        "pricing" if value.eq_ignore_ascii_case("x402") => "x402".into(),
        _ => value
            .split(['-', '_'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn dashboard_bucket(axis: &str, id: String, label: Option<String>, count: i64) -> DashboardBucket {
    let label = label.unwrap_or_else(|| dashboard_label(axis, &id));
    let href = dashboard_filter_href(axis, &id);
    DashboardBucket {
        id,
        label,
        count,
        href,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DashboardBucket {
    pub id: String,
    pub label: String,
    pub count: i64,
    pub href: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DashboardMetrics {
    pub public_tools: i64,
    pub mcp_tools: i64,
    pub cli_tools: i64,
    pub sdk_tools: i64,
    pub api_tools: i64,
    pub x402_tools: i64,
    pub official_tools: i64,
    pub verified_tools: i64,
    pub updated_recently: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PublicDashboardSnapshot {
    pub metrics: DashboardMetrics,
    pub type_counts: Vec<DashboardBucket>,
    pub function_counts: Vec<DashboardBucket>,
    pub chain_counts: Vec<DashboardBucket>,
    pub trust_counts: Vec<DashboardBucket>,
    pub pricing_counts: Vec<DashboardBucket>,
    pub new_tools: Vec<crate::models::tool::PublicToolSummary>,
    pub popular_tools: Vec<crate::models::tool::PublicToolSummary>,
    pub x402_tools: Vec<crate::models::tool::PublicToolSummary>,
    pub high_trust_tools: Vec<crate::models::tool::PublicToolSummary>,
    pub as_of: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolkitExportPayload {
    pub format: String,
    pub filename: String,
    pub body: String,
}

/// Public-safe tool shape for JSON export — omits internal ids and operator fields.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolkitExportTool {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub function: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub chains: Vec<String>,
    pub status: String,
    pub official_team: Option<String>,
    pub pricing: String,
    pub x402_price: Option<String>,
    pub install_command: Option<String>,
    pub safe_copy_command: Option<String>,
    pub mcp_endpoint: Option<String>,
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
    pub npm_package: Option<String>,
    pub license: Option<String>,
    pub stars: i32,
    pub claim_state: String,
    pub install_risk_level: String,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolkitToolView {
    pub tool: Tool,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub source: String,
    pub source_client: Option<String>,
    pub saved_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ToolkitToolView {
    pub fn from_tool(tool: Tool) -> Self {
        let now = chrono::Utc::now();
        Self {
            tool,
            note: None,
            tags: Vec::new(),
            source: "web".into(),
            source_client: None,
            saved_at: now,
            updated_at: now,
        }
    }
}

fn tool_to_toolkit_export(item: &ToolkitToolView) -> ToolkitExportTool {
    let tool = &item.tool;
    ToolkitExportTool {
        slug: tool.slug.clone(),
        name: tool.name.clone(),
        description: tool.description.clone(),
        function: tool.function.clone(),
        tool_type: tool.tool_type.clone(),
        chains: tool.chains.clone(),
        status: tool.status.clone(),
        official_team: tool.official_team.clone(),
        pricing: tool.pricing.clone(),
        x402_price: tool.x402_price.clone(),
        install_command: tool.install_command.clone(),
        safe_copy_command: tool.safe_copy_command.clone(),
        mcp_endpoint: tool.mcp_endpoint.clone(),
        repo_url: tool.repo_url.clone(),
        homepage: tool.homepage.clone(),
        npm_package: tool.npm_package.clone(),
        license: tool.license.clone(),
        stars: tool.stars,
        claim_state: tool.claim_state.clone(),
        install_risk_level: tool.install_risk_level.clone(),
        note: item.note.clone(),
        tags: item.tags.clone(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MyToolkitPayload {
    pub total: i64,
    pub items: Vec<ToolkitToolView>,
    pub tools: Vec<Tool>,
    pub markdown_export: ToolkitExportPayload,
    pub json_export: ToolkitExportPayload,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateToolkitItemPayload {
    pub slug: String,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolComparisonView {
    pub tool: Tool,
    pub official_links: Vec<ToolOfficialLink>,
    pub trust_facts: Vec<TrustFact>,
    pub viewer_bookmarked: bool,
}

/// Request for bundled browser data load.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadBrowserDataRequest {
    pub sort: String,
    #[serde(default)]
    pub filters: ToolFilters,
    #[serde(default)]
    pub search_q: Option<String>,
    #[serde(default)]
    pub selected: Option<String>,
    pub page: u32,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
struct DashboardValueCountRow {
    id: String,
    count: i64,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
struct DashboardCategoryCountRow {
    id: String,
    label: String,
    count: i64,
}

#[cfg(feature = "ssr")]
async fn fetch_dashboard_value_counts(
    pool: &sqlx::PgPool,
    axis: DashboardCountAxis,
    limit: i64,
) -> Result<Vec<DashboardBucket>, FnError> {
    let rows = sqlx::query_as::<_, DashboardValueCountRow>(axis.count_sql())
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            FnError::new(format!(
                "{} dashboard counts failed: {e}",
                axis.bucket_axis()
            ))
        })?;

    Ok(rows
        .into_iter()
        .map(|row| dashboard_bucket(axis.bucket_axis(), row.id, None, row.count))
        .collect())
}

#[cfg(feature = "ssr")]
async fn fetch_dashboard_function_counts(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<Vec<DashboardBucket>, FnError> {
    let rows = sqlx::query_as::<_, DashboardCategoryCountRow>(DASHBOARD_FUNCTION_COUNTS_SQL)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("function dashboard counts failed: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|row| dashboard_bucket("function", row.id, Some(row.label), row.count))
        .collect())
}

#[cfg(feature = "ssr")]
async fn fetch_dashboard_x402_tools(pool: &sqlx::PgPool, limit: i64) -> Result<Vec<Tool>, FnError> {
    let tools = sqlx::query_as::<_, Tool>(DASHBOARD_X402_TOOLS_SQL)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("x402 dashboard tools failed: {e}")))?;
    Ok(sanitize_tools_for_public_response(tools))
}

#[cfg(feature = "ssr")]
async fn fetch_dashboard_metrics(pool: &sqlx::PgPool) -> Result<DashboardMetrics, FnError> {
    let row =
        sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, i64, i64, i64)>(DASHBOARD_METRICS_SQL)
            .fetch_one(pool)
            .await
            .map_err(|e| FnError::new(format!("dashboard metrics failed: {e}")))?;

    Ok(DashboardMetrics {
        public_tools: row.0,
        mcp_tools: row.1,
        cli_tools: row.2,
        sdk_tools: row.3,
        api_tools: row.4,
        x402_tools: row.5,
        official_tools: row.6,
        verified_tools: row.7,
        updated_recently: row.8,
    })
}

#[cfg(feature = "ssr")]
pub(crate) async fn fetch_public_dashboard_snapshot(
    pool: &sqlx::PgPool,
    list_limit: i64,
) -> Result<PublicDashboardSnapshot, FnError> {
    let limit = clamp_dashboard_list_limit(list_limit);
    let empty_filters = ToolFilters::default();
    let high_trust_filters = ToolFilters {
        status: vec!["official".into(), "verified".into()],
        ..Default::default()
    };

    let (metrics, type_counts, function_counts, chain_counts, trust_counts, pricing_counts) = futures::join!(
        fetch_dashboard_metrics(pool),
        fetch_dashboard_value_counts(pool, DashboardCountAxis::Type, limit),
        fetch_dashboard_function_counts(pool, limit),
        fetch_chain_counts(pool, limit),
        fetch_dashboard_value_counts(pool, DashboardCountAxis::Status, limit),
        fetch_dashboard_value_counts(pool, DashboardCountAxis::Pricing, limit),
    );
    let (new_tools, popular_tools, x402_tools, high_trust_tools) = futures::join!(
        fetch_list_tools(pool, "new", 0, limit, &empty_filters, None),
        fetch_list_tools(pool, "hot", 0, limit, &empty_filters, None),
        fetch_dashboard_x402_tools(pool, limit),
        fetch_list_tools(pool, "hot", 0, limit, &high_trust_filters, None),
    );

    use crate::models::tool::tools_to_public_summaries;

    Ok(PublicDashboardSnapshot {
        metrics: metrics?,
        type_counts: type_counts?,
        function_counts: function_counts?,
        chain_counts: chain_counts?
            .into_iter()
            .map(|(id, count)| dashboard_bucket("chain", id, None, count))
            .collect(),
        trust_counts: trust_counts?,
        pricing_counts: pricing_counts?,
        new_tools: tools_to_public_summaries(new_tools?),
        popular_tools: tools_to_public_summaries(popular_tools?),
        x402_tools: tools_to_public_summaries(x402_tools?),
        high_trust_tools: tools_to_public_summaries(high_trust_tools?),
        as_of: chrono::Utc::now(),
    })
}

fn sanitize_toolkit_items(items: Vec<ToolkitToolView>) -> Vec<ToolkitToolView> {
    items
        .into_iter()
        .map(|mut item| {
            let mut tool = sanitize_tool_for_public_response(item.tool);
            tool.name = redact_secrets(&tool.name);
            tool.description = tool.description.map(|value| redact_secrets(&value));
            tool.install_command = tool.install_command.map(|value| redact_secrets(&value));
            tool.safe_copy_command = tool.safe_copy_command.map(|value| redact_secrets(&value));
            tool.mcp_endpoint = tool.mcp_endpoint.map(|value| redact_secrets(&value));
            item.tool = tool;
            item.note = item.note.map(|value| redact_secrets(&value));
            item.tags = item
                .tags
                .into_iter()
                .map(|value| redact_secrets(&value))
                .collect();
            item
        })
        .collect()
}

fn toolkit_markdown_for_items(items: &[ToolkitToolView]) -> String {
    let mut body = String::from("# My OnchainAI Toolkit\n\n");
    if items.is_empty() {
        body.push_str("No saved tools yet.\n");
        return body;
    }

    body.push_str("Saved tools exported from OnchainAI.\n\n");
    for item in items {
        let tool = &item.tool;
        let chains = if tool.chains.is_empty() {
            "Not listed".into()
        } else {
            tool.chains.join(", ")
        };
        let install = tool
            .safe_copy_command
            .as_deref()
            .or(tool.install_command.as_deref())
            .unwrap_or("No install command listed");
        let endpoint = tool
            .mcp_endpoint
            .as_deref()
            .unwrap_or("No MCP endpoint listed");
        let _ = writeln!(body, "## {}", tool.name);
        let _ = writeln!(body, "- Slug: {}", tool.slug);
        let _ = writeln!(body, "- Type: {}", tool.tool_type);
        let _ = writeln!(body, "- Function: {}", tool.function);
        let _ = writeln!(body, "- Chains: {chains}");
        let _ = writeln!(body, "- Trust: {}", tool.status);
        let _ = writeln!(body, "- Pricing: {}", tool.pricing);
        if let Some(note) = item
            .note
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let _ = writeln!(body, "- Note: {note}");
        }
        if !item.tags.is_empty() {
            let _ = writeln!(body, "- Tags: {}", item.tags.join(", "));
        }
        if let Some(price) = tool.x402_price.as_deref().filter(|value| !value.is_empty()) {
            let _ = writeln!(body, "- x402 price: {price}");
        }
        let _ = writeln!(body, "- Install: `{install}`");
        let _ = writeln!(body, "- MCP endpoint: {endpoint}");
        let _ = writeln!(body, "- OnchainAI: /tools/{}\n", tool.slug);
    }
    body
}

pub fn build_toolkit_payload(items: Vec<ToolkitToolView>) -> Result<MyToolkitPayload, FnError> {
    let items = sanitize_toolkit_items(items);
    let tools: Vec<Tool> = items.iter().map(|item| item.tool.clone()).collect();
    let markdown_body = toolkit_markdown_for_items(&items);
    let export_tools: Vec<ToolkitExportTool> = items.iter().map(tool_to_toolkit_export).collect();
    let json_body = serde_json::to_string_pretty(&export_tools)
        .map_err(|e| FnError::new(format!("failed to serialize toolkit: {e}")))?;

    Ok(MyToolkitPayload {
        total: items.len() as i64,
        items,
        tools,
        markdown_export: ToolkitExportPayload {
            format: "markdown".into(),
            filename: "onchainai-toolkit.md".into(),
            body: markdown_body,
        },
        json_export: ToolkitExportPayload {
            format: "json".into(),
            filename: "onchainai-toolkit.json".into(),
            body: json_body,
        },
    })
}

#[cfg(feature = "ssr")]
async fn fetch_user_toolkit(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<MyToolkitPayload, FnError> {
    use sqlx::{FromRow, Row};

    let rows = sqlx::query(USER_TOOLKIT_SQL)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("failed to load toolkit: {e}")))?;

    let mut items = Vec::new();
    for row in rows {
        let tool = Tool::from_row(&row)
            .map_err(|e| FnError::new(format!("failed to decode toolkit tool: {e}")))?;
        let note = row
            .try_get::<Option<String>, _>("bookmark_note")
            .map_err(|e| FnError::new(format!("failed to decode toolkit note: {e}")))?;
        let tags = row
            .try_get::<Vec<String>, _>("bookmark_tags")
            .map_err(|e| FnError::new(format!("failed to decode toolkit tags: {e}")))?;
        let saved_at = row
            .try_get::<chrono::DateTime<chrono::Utc>, _>("bookmark_created_at")
            .map_err(|e| FnError::new(format!("failed to decode toolkit saved_at: {e}")))?;
        let updated_at = row
            .try_get::<chrono::DateTime<chrono::Utc>, _>("bookmark_updated_at")
            .map_err(|e| FnError::new(format!("failed to decode toolkit updated_at: {e}")))?;
        let source = row
            .try_get::<String, _>("bookmark_source")
            .unwrap_or_else(|_| "web".into());
        let source_client = row
            .try_get::<Option<String>, _>("bookmark_source_client")
            .ok()
            .flatten();
        items.push(ToolkitToolView {
            tool,
            note,
            tags,
            source,
            source_client,
            saved_at,
            updated_at,
        });
    }

    build_toolkit_payload(items)
}

pub(crate) fn validate_toolkit_tags(tags: &[String]) -> Result<Vec<String>, FnError> {
    if tags.len() > 8 {
        return Err(FnError::new("toolkit tags accept at most 8 values"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.trim().trim_start_matches('#').to_ascii_lowercase();
        if tag.is_empty() {
            continue;
        }
        if tag.len() > 32 {
            return Err(FnError::new("toolkit tags must be at most 32 characters"));
        }
        if tag
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))
        {
            return Err(FnError::new(
                "toolkit tags may contain letters, numbers, hyphens, and underscores",
            ));
        }
        if seen.insert(tag.clone()) {
            normalized.push(tag);
        }
    }
    Ok(normalized)
}

fn validate_toolkit_note(note: Option<String>) -> Result<Option<String>, FnError> {
    let note = note.map(|value| value.trim().to_string());
    let note = note.filter(|value| !value.is_empty());
    if note.as_ref().is_some_and(|value| value.len() > 500) {
        return Err(FnError::new("toolkit note must be at most 500 characters"));
    }
    Ok(note)
}

/// Batch comment counts for approved tools by slug.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_tool_comment_counts(
    pool: &sqlx::PgPool,
    slugs: &[String],
) -> Result<Vec<(String, i64)>, FnError> {
    if slugs.is_empty() {
        return Ok(Vec::new());
    }

    let rows = sqlx::query_as::<_, (String, i64)>(TOOL_COMMENT_COUNTS_BY_SLUGS_SQL)
        .bind(slugs)
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("comment counts failed: {e}")))?;

    Ok(rows)
}

#[cfg(all(test, feature = "ssr"))]
mod fetch_install_guide_tests {
    use super::*;

    fn require_db_tests() -> bool {
        std::env::var("ONCHAINAI_REQUIRE_DB_TESTS")
            .ok()
            .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
    }

    async fn test_pool() -> Option<sqlx::PgPool> {
        let database_url = std::env::var("SUPABASE_URL_TEST")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .ok()?;
        sqlx::PgPool::connect(&database_url).await.ok()
    }

    #[tokio::test]
    async fn fetch_public_install_guide_loads_approved_tool_from_db() {
        let Some(pool) = test_pool().await else {
            if require_db_tests() {
                panic!("ONCHAINAI_REQUIRE_DB_TESTS=1 but DATABASE_URL is unavailable");
            }
            eprintln!("SKIP: DATABASE_URL not set — fetch_public_install_guide DB test");
            return;
        };

        let slug: Option<String> = sqlx::query_scalar(
            "SELECT slug FROM tools \
             WHERE approval_status = 'approved' \
               AND quarantined_at IS NULL \
               AND install_risk_level <> 'critical' \
               AND (install_command IS NOT NULL OR mcp_endpoint IS NOT NULL) \
             LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("query approved tool slug");

        let Some(slug) = slug else {
            if require_db_tests() {
                panic!("ONCHAINAI_REQUIRE_DB_TESTS=1 but no eligible approved tool found");
            }
            eprintln!("SKIP: no eligible approved tool in database");
            return;
        };

        let guide = fetch_public_install_guide(&pool, &slug, "claude")
            .await
            .unwrap_or_else(|error| {
                panic!("fetch_public_install_guide failed for {slug}: {error}")
            });

        assert_eq!(guide.slug, slug);
        assert_eq!(guide.platform, "claude");
        assert!(!guide.blocked);
        assert!(
            guide.copy_text.is_some() || guide.config_json.is_some(),
            "expected copy output for low/medium-risk tool"
        );

        let local = crate::public_install_guide::build_install_guide_for_platform(
            &fetch_tool_by_slug(&pool, &slug)
                .await
                .expect("reload tool")
                .expect("tool exists"),
            &slug,
            "claude",
        )
        .expect("local builder");
        assert_eq!(guide.copy_text, local.copy_text);
        assert_eq!(guide.config_json, local.config_json);
    }

    #[tokio::test]
    async fn fetch_public_install_guide_returns_not_found_for_missing_slug() {
        let Some(pool) = test_pool().await else {
            eprintln!("SKIP: DATABASE_URL not set — missing slug test");
            return;
        };

        let missing = format!("missing-mcp-tool-{}", uuid::Uuid::new_v4());
        let result = fetch_public_install_guide(&pool, &missing, "claude").await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("tool not found"),
            "expected not-found error"
        );
    }
}

/// Install-guide integration test helpers (direct fetch path, no Leptos RPC).
#[cfg(all(feature = "ssr", any(test, feature = "test-helpers")))]
pub mod server_fn_context_tests {
    use super::{fetch_public_install_guide, fetch_tool_by_slug};
    use crate::public_install_guide::{
        build_install_guide_for_platform, build_public_install_guide, resolve_install_guide,
        InstallPlatform,
    };
    use sqlx::postgres::PgPoolOptions;
    use std::fmt::Display;

    pub fn db_tests_required() -> bool {
        std::env::var("ONCHAINAI_REQUIRE_DB_TESTS")
            .ok()
            .is_some_and(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
    }

    pub fn skip_or_panic(context: &str, err: impl Display) {
        if db_tests_required() {
            panic!("{context}: {err}");
        }
        eprintln!("SKIP: {context}: {err}");
    }

    pub async fn test_pool() -> Result<sqlx::PgPool, String> {
        let _ = dotenvy::dotenv();
        let database_url = std::env::var("SUPABASE_URL_TEST")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::env::var("DATABASE_URL")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .ok_or_else(|| "missing SUPABASE_URL_TEST or DATABASE_URL".to_string())?;
        PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(15))
            .connect(&database_url)
            .await
            .map_err(|error| format!("failed to connect test database: {error}"))
    }

    pub async fn call_fetch_public_install_guide(
        pool: &sqlx::PgPool,
        slug: &str,
        platform: &str,
    ) -> Result<crate::public_install_guide::PublicInstallGuide, crate::server::fn_error::FnError>
    {
        fetch_public_install_guide(pool, slug, platform).await
    }

    pub async fn eligible_approved_slug(pool: &sqlx::PgPool) -> Option<String> {
        sqlx::query_scalar(
            "SELECT slug FROM tools \
             WHERE approval_status = 'approved' \
               AND quarantined_at IS NULL \
               AND install_risk_level <> 'critical' \
               AND (install_command IS NOT NULL OR mcp_endpoint IS NOT NULL) \
             LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .ok()?
    }

    pub async fn run_get_public_install_guide_server_fn_loads_approved_tool() {
        let pool = match test_pool().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("get_public_install_guide server fn DB setup failed", err);
                return;
            }
        };

        let Some(slug) = eligible_approved_slug(&pool).await else {
            skip_or_panic(
                "get_public_install_guide server fn DB setup failed",
                "no eligible approved tool found",
            );
            return;
        };

        let guide = call_fetch_public_install_guide(&pool, &slug, "claude")
            .await
            .unwrap_or_else(|error| {
                panic!("fetch_public_install_guide() failed for {slug}: {error}")
            });

        assert_eq!(guide.slug, slug);
        assert_eq!(guide.platform, "claude");
        assert!(!guide.blocked);
        assert!(
            guide.copy_text.is_some() || guide.config_json.is_some(),
            "expected copy output for eligible tool"
        );
    }

    #[cfg(test)]
    #[tokio::test(flavor = "multi_thread")]
    async fn get_public_install_guide_server_fn_loads_approved_tool() {
        run_get_public_install_guide_server_fn_loads_approved_tool().await;
    }

    pub async fn run_get_public_install_guide_server_fn_returns_not_found_for_missing_slug() {
        let pool = match test_pool().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("get_public_install_guide missing slug test", err);
                return;
            }
        };

        let missing = format!("missing-mcp-tool-{}", uuid::Uuid::new_v4());
        let result = call_fetch_public_install_guide(&pool, &missing, "claude").await;

        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("tool not found"),
            "expected not-found error for {missing}"
        );
    }

    #[cfg(test)]
    #[tokio::test(flavor = "multi_thread")]
    async fn get_public_install_guide_server_fn_returns_not_found_for_missing_slug() {
        run_get_public_install_guide_server_fn_returns_not_found_for_missing_slug().await;
    }

    pub async fn run_install_guide_panel_chain_matches_server_fn_for_approved_tool() {
        let pool = match test_pool().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("install guide panel chain DB setup failed", err);
                return;
            }
        };

        let Some(slug) = eligible_approved_slug(&pool).await else {
            skip_or_panic(
                "install guide panel chain DB setup failed",
                "no eligible approved tool found",
            );
            return;
        };

        let tool = fetch_tool_by_slug(&pool, &slug)
            .await
            .expect("fetch_tool_by_slug must succeed for approved tool")
            .expect("approved tool must exist");

        let remote = call_fetch_public_install_guide(&pool, &slug, "claude")
            .await
            .expect("fetch must succeed for approved tool");

        let local = build_public_install_guide(&tool, &slug, InstallPlatform::Claude);
        let resolved = resolve_install_guide(Some(Ok(remote.clone())), local.clone());

        assert_eq!(resolved, remote);
        assert_eq!(resolved.copy_text, local.copy_text);
        assert_eq!(resolved.config_json, local.config_json);

        let direct = build_install_guide_for_platform(
            &fetch_tool_by_slug(&pool, &slug)
                .await
                .expect("reload tool")
                .expect("tool exists"),
            &slug,
            "claude",
        )
        .expect("platform builder must match server fn body");
        assert_eq!(resolved.copy_text, direct.copy_text);
        assert_eq!(resolved.config_json, direct.config_json);
    }

    #[cfg(test)]
    #[tokio::test(flavor = "multi_thread")]
    async fn install_guide_panel_chain_matches_server_fn_for_approved_tool() {
        run_install_guide_panel_chain_matches_server_fn_for_approved_tool().await;
    }
}
