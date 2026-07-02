use super::*;

/// Returns the most recently added **approved** tools.
///
/// HOT order = higher `stars` first, then more recent `created_at`.
#[server(GetRecentTools, "/api")]
pub async fn get_recent_tools(limit: i64) -> Result<Vec<Tool>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let limit = limit.clamp(1, 100);
    let tools = sqlx::query_as::<_, Tool>(RECENT_APPROVED_TOOLS_SQL)
        .bind(limit)
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load tools: {e}")))?;

    Ok(sanitize_tools_for_public_response(tools))
}

/// Returns all function categories with live **approved** tool counts.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_categories(
    pool: &sqlx::PgPool,
) -> Result<Vec<(Category, i64)>, ServerFnError> {
    let rows = sqlx::query_as::<_, CategoryWithCount>(CATEGORIES_WITH_COUNTS_SQL)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load categories: {e}")))?;

    Ok(rows.into_iter().map(CategoryWithCount::into_pair).collect())
}

/// Returns all function categories with live **approved** tool counts.
#[server(GetCategories, "/api")]
pub async fn get_categories() -> Result<Vec<(Category, i64)>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;
    fetch_categories(&pool).await
}

/// Searches **approved** tools using Postgres full-text search.
///
/// Optional filters narrow by `function` and any chain in `chains`.
#[server(SearchTools, "/api")]
pub async fn search_tools(
    query: String,
    function: Option<String>,
    chain: Option<String>,
) -> Result<Vec<Tool>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    validate_search_tools_input(&query, &function, &chain)?;

    let tools = sqlx::query_as::<_, Tool>(SEARCH_APPROVED_TOOLS_SQL)
        .bind(&query)
        .bind(function.as_deref())
        .bind(chain.as_deref())
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("search failed: {e}")))?;

    Ok(sanitize_tools_for_public_response(tools))
}

/// Fetch a single **approved** tool by slug, if present.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_tool_by_slug(
    pool: &sqlx::PgPool,
    slug: &str,
) -> Result<Option<Tool>, ServerFnError> {
    let tool = sqlx::query_as::<_, Tool>(APPROVED_TOOL_BY_SLUG_SQL)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load tool: {e}")))?;

    Ok(tool.map(sanitize_tool_for_public_response))
}

/// Fetch a single **approved** tool by slug (404-style error if missing or not approved).
#[server(GetToolBySlug, "/api")]
pub async fn get_tool_by_slug(slug: String) -> Result<Tool, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;
    fetch_tool_by_slug(&pool, &slug)
        .await?
        .ok_or_else(|| ServerFnError::new(format!("tool not found: {slug}")))
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
) -> Result<(), ServerFnError> {
    if query.trim().is_empty() {
        return Err(ServerFnError::new("query must not be empty"));
    }
    if query.len() > MAX_TOOL_LIST_QUERY_LEN {
        return Err(ServerFnError::new(format!(
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

fn validate_tool_filter_values(axis: &str, values: &[String]) -> Result<(), ServerFnError> {
    if values.len() > MAX_TOOL_FILTER_VALUES {
        return Err(ServerFnError::new(format!(
            "filter `{axis}` accepts at most {MAX_TOOL_FILTER_VALUES} values"
        )));
    }
    for value in values {
        if value.len() > MAX_TOOL_FILTER_VALUE_LEN {
            return Err(ServerFnError::new(format!(
                "filter `{axis}` values must be at most {MAX_TOOL_FILTER_VALUE_LEN} characters"
            )));
        }
    }
    Ok(())
}

/// Validates multi-axis tool filters for list/count queries.
pub fn validate_tool_filters(filters: &ToolFilters) -> Result<(), ServerFnError> {
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

fn validate_tool_sort(sort: &str) -> Result<(), ServerFnError> {
    matches!(sort, "hot" | "new" | "comments")
        .then_some(())
        .ok_or_else(|| ServerFnError::new("sort must be one of: hot, new, comments"))
}

/// Validates browser tool-list request bounds (rejects out-of-range instead of clamping).
pub fn validate_tool_list_request(req: &ToolListRequest) -> Result<(), ServerFnError> {
    validate_tool_sort(&req.sort)?;
    validate_tool_filters(&req.filters)?;
    if req.offset < 0 {
        return Err(ServerFnError::new("offset must be >= 0"));
    }
    if !(1..=MAX_LIST_TOOLS_LIMIT).contains(&req.limit) {
        return Err(ServerFnError::new(format!(
            "limit must be between 1 and {MAX_LIST_TOOLS_LIMIT}"
        )));
    }
    if let Some(query) = req.query.as_ref() {
        if query.len() > MAX_TOOL_LIST_QUERY_LEN {
            return Err(ServerFnError::new(format!(
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

/// Optional axis filters for tool list/count queries (AND across axes; OR within axis via ANY).
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
pub(crate) fn append_tool_filters<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    filters: &'qb ToolFilters,
) {
    if !filters.function.is_empty() {
        query
            .push(" AND function = ANY(")
            .push_bind(&filters.function)
            .push(")");
    }
    if !filters.asset_class.is_empty() {
        query
            .push(" AND asset_class = ANY(")
            .push_bind(&filters.asset_class)
            .push(")");
    }
    if !filters.actor.is_empty() {
        query
            .push(" AND actor = ANY(")
            .push_bind(&filters.actor)
            .push(")");
    }
    if !filters.tool_type.is_empty() {
        query
            .push(" AND type = ANY(")
            .push_bind(&filters.tool_type)
            .push(")");
    }
    if !filters.status.is_empty() {
        query
            .push(" AND status = ANY(")
            .push_bind(&filters.status)
            .push(")");
    }
    if !filters.pricing.is_empty() {
        query
            .push(" AND pricing = ANY(")
            .push_bind(&filters.pricing)
            .push(")");
    }
    if !filters.install_risk.is_empty() {
        query
            .push(" AND install_risk_level = ANY(")
            .push_bind(&filters.install_risk)
            .push(")");
    }
    if !filters.chain.is_empty() {
        query.push(" AND chains && ").push_bind(&filters.chain);
    }
}

#[cfg(feature = "ssr")]
fn push_list_query_filter<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    search: Option<&'qb str>,
) {
    if let Some(text) = search.filter(|q| !q.trim().is_empty()) {
        query.push(
            " AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')) \
             @@ plainto_tsquery('english', ",
        );
        query.push_bind(text);
        query.push(")");
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
) -> Result<i64, ServerFnError> {
    let mut q = sqlx::QueryBuilder::new(COUNT_APPROVED_TOOLS_SQL);
    append_tool_filters(&mut q, filters);

    let count = q
        .build_query_as::<(i64,)>()
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("count failed: {e}")))?;

    Ok(count.0)
}

/// Count approved tools with optional multi-axis filters.
#[server(CountTools, "/api")]
pub async fn count_tools(filters: ToolFilters) -> Result<i64, ServerFnError> {
    validate_tool_filters(&filters)?;
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;
    fetch_count_tools(&pool, &filters).await
}

/// Top chains by approved-tool count for sidebar filters.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_chain_counts(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<Vec<(String, i64)>, ServerFnError> {
    let limit = limit.clamp(1, 100);
    let rows = sqlx::query_as::<_, (String, i64)>(CHAIN_COUNTS_SQL)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("chain counts failed: {e}")))?;

    Ok(rows)
}

/// Top chains by approved-tool count for sidebar filters.
#[server(GetChainCounts, "/api")]
pub async fn get_chain_counts(limit: i64) -> Result<Vec<(String, i64)>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;
    fetch_chain_counts(&pool, limit).await
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
) -> Result<Vec<Tool>, ServerFnError> {
    let offset = offset.max(0);
    let limit = clamp_list_tools_limit(limit);
    let mut q = sqlx::QueryBuilder::new(LIST_APPROVED_TOOLS_SQL);
    push_list_query_filter(&mut q, query);
    append_tool_filters(&mut q, filters);
    push_list_order_offset_limit(&mut q, sort, offset, limit);

    let tools = q
        .build_query_as::<Tool>()
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("list tools failed: {e}")))?;

    Ok(sanitize_tools_for_public_response(tools))
}

/// List approved tools with sort, pagination, FTS query, and optional filters.
#[server(ListTools, "/api")]
pub async fn list_tools(
    sort: String,
    offset: i64,
    limit: i64,
    filters: ToolFilters,
    query: Option<String>,
) -> Result<Vec<Tool>, ServerFnError> {
    let req = ToolListRequest {
        sort: sort.clone(),
        offset,
        limit,
        filters: filters.clone(),
        query: query.clone(),
    };
    validate_tool_list_request(&req)?;
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;
    fetch_list_tools(&pool, &sort, offset, limit, &filters, query.as_deref()).await
}

/// Stable browser-facing tool list — wraps positional `list_tools` with a struct payload.
#[server(ListToolsV1, "/api")]
pub async fn list_tools_v1(req: ToolListRequest) -> Result<Vec<Tool>, ServerFnError> {
    validate_tool_list_request(&req)?;
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;
    fetch_list_tools(
        &pool,
        &req.sort,
        req.offset,
        req.limit,
        &req.filters,
        req.query.as_deref(),
    )
    .await
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
    pub new_tools: Vec<Tool>,
    pub popular_tools: Vec<Tool>,
    pub x402_tools: Vec<Tool>,
    pub high_trust_tools: Vec<Tool>,
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

/// Load all data required by `ToolsBrowser` in **one** server round-trip (one DB pool checkout
/// sequence per HTTP request; internal queries still run concurrently on the server).
#[server(LoadBrowserData, "/api")]
pub async fn load_browser_data(
    req: LoadBrowserDataRequest,
) -> Result<BrowserDataPayload, ServerFnError> {
    validate_tool_filters(&req.filters)?;
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let page = clamp_browser_page_param(req.page);
    let list_req = ToolListRequest {
        sort: req.sort.clone(),
        offset: 0,
        limit: browser_visible_limit_for_page(page),
        filters: req.filters.clone(),
        query: req.search_q.clone(),
    };
    validate_tool_list_request(&list_req)?;

    let preview_slug = req.selected.filter(|s| !s.is_empty());

    let (categories, chains, total, tools, preview_tool) = futures::join!(
        fetch_categories(&pool),
        fetch_chain_counts(&pool, 12),
        fetch_count_tools(&pool, &req.filters),
        fetch_list_tools(
            &pool,
            &list_req.sort,
            list_req.offset,
            list_req.limit,
            &list_req.filters,
            list_req.query.as_deref(),
        ),
        async {
            match preview_slug.as_deref() {
                Some(s) => fetch_tool_by_slug(&pool, s).await.ok().flatten(),
                None => None,
            }
        },
    );
    let categories = categories?;
    let chains = chains?;
    let total = total?;
    let tools = tools?;

    let slugs: Vec<String> = tools.iter().map(|t| t.slug.clone()).collect();
    let comment_counts: HashMap<String, i64> = if slugs.is_empty() {
        HashMap::new()
    } else {
        fetch_tool_comment_counts(&pool, &slugs)
            .await?
            .into_iter()
            .collect()
    };

    Ok(BrowserDataPayload {
        categories,
        chains,
        total,
        tools,
        comment_counts,
        preview_tool,
    })
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
) -> Result<Vec<DashboardBucket>, ServerFnError> {
    let rows = sqlx::query_as::<_, DashboardValueCountRow>(axis.count_sql())
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            ServerFnError::new(format!(
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
) -> Result<Vec<DashboardBucket>, ServerFnError> {
    let rows = sqlx::query_as::<_, DashboardCategoryCountRow>(DASHBOARD_FUNCTION_COUNTS_SQL)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("function dashboard counts failed: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|row| dashboard_bucket("function", row.id, Some(row.label), row.count))
        .collect())
}

#[cfg(feature = "ssr")]
async fn fetch_dashboard_x402_tools(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<Vec<Tool>, ServerFnError> {
    let tools = sqlx::query_as::<_, Tool>(DASHBOARD_X402_TOOLS_SQL)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("x402 dashboard tools failed: {e}")))?;
    Ok(sanitize_tools_for_public_response(tools))
}

#[cfg(feature = "ssr")]
async fn fetch_dashboard_metrics(pool: &sqlx::PgPool) -> Result<DashboardMetrics, ServerFnError> {
    let row =
        sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, i64, i64, i64)>(DASHBOARD_METRICS_SQL)
            .fetch_one(pool)
            .await
            .map_err(|e| ServerFnError::new(format!("dashboard metrics failed: {e}")))?;

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
) -> Result<PublicDashboardSnapshot, ServerFnError> {
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
        new_tools: new_tools?,
        popular_tools: popular_tools?,
        x402_tools: x402_tools?,
        high_trust_tools: high_trust_tools?,
        as_of: chrono::Utc::now(),
    })
}

#[server(GetPublicDashboardSnapshot, "/api")]
pub async fn get_public_dashboard_snapshot(
    list_limit: i64,
) -> Result<PublicDashboardSnapshot, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;
    fetch_public_dashboard_snapshot(&pool, list_limit).await
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

pub fn build_toolkit_payload(
    items: Vec<ToolkitToolView>,
) -> Result<MyToolkitPayload, ServerFnError> {
    let items = sanitize_toolkit_items(items);
    let tools: Vec<Tool> = items.iter().map(|item| item.tool.clone()).collect();
    let markdown_body = toolkit_markdown_for_items(&items);
    let export_tools: Vec<ToolkitExportTool> = items.iter().map(tool_to_toolkit_export).collect();
    let json_body = serde_json::to_string_pretty(&export_tools)
        .map_err(|e| ServerFnError::new(format!("failed to serialize toolkit: {e}")))?;

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
) -> Result<MyToolkitPayload, ServerFnError> {
    use sqlx::{FromRow, Row};

    let rows = sqlx::query(USER_TOOLKIT_SQL)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load toolkit: {e}")))?;

    let mut items = Vec::new();
    for row in rows {
        let tool = Tool::from_row(&row)
            .map_err(|e| ServerFnError::new(format!("failed to decode toolkit tool: {e}")))?;
        let note = row
            .try_get::<Option<String>, _>("bookmark_note")
            .map_err(|e| ServerFnError::new(format!("failed to decode toolkit note: {e}")))?;
        let tags = row
            .try_get::<Vec<String>, _>("bookmark_tags")
            .map_err(|e| ServerFnError::new(format!("failed to decode toolkit tags: {e}")))?;
        let saved_at = row
            .try_get::<chrono::DateTime<chrono::Utc>, _>("bookmark_created_at")
            .map_err(|e| ServerFnError::new(format!("failed to decode toolkit saved_at: {e}")))?;
        let updated_at = row
            .try_get::<chrono::DateTime<chrono::Utc>, _>("bookmark_updated_at")
            .map_err(|e| ServerFnError::new(format!("failed to decode toolkit updated_at: {e}")))?;
        items.push(ToolkitToolView {
            tool,
            note,
            tags,
            saved_at,
            updated_at,
        });
    }

    build_toolkit_payload(items)
}

#[server(ListMyToolkit, "/api")]
pub async fn list_my_toolkit() -> Result<MyToolkitPayload, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let user = require_user(&parts, &pool, &config.jwt_secret, &config.jwt_issuer()).await?;
    fetch_user_toolkit(&pool, user.id).await
}

fn validate_toolkit_tags(tags: &[String]) -> Result<Vec<String>, ServerFnError> {
    if tags.len() > 8 {
        return Err(ServerFnError::new("toolkit tags accept at most 8 values"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.trim().trim_start_matches('#').to_ascii_lowercase();
        if tag.is_empty() {
            continue;
        }
        if tag.len() > 32 {
            return Err(ServerFnError::new(
                "toolkit tags must be at most 32 characters",
            ));
        }
        if tag
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))
        {
            return Err(ServerFnError::new(
                "toolkit tags may contain letters, numbers, hyphens, and underscores",
            ));
        }
        if seen.insert(tag.clone()) {
            normalized.push(tag);
        }
    }
    Ok(normalized)
}

fn validate_toolkit_note(note: Option<String>) -> Result<Option<String>, ServerFnError> {
    let note = note.map(|value| value.trim().to_string());
    let note = note.filter(|value| !value.is_empty());
    if note.as_ref().is_some_and(|value| value.len() > 500) {
        return Err(ServerFnError::new(
            "toolkit note must be at most 500 characters",
        ));
    }
    Ok(note)
}

#[server(UpdateToolkitItem, "/api")]
pub async fn update_toolkit_item(input: UpdateToolkitItemPayload) -> Result<(), ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let user = require_user(&parts, &pool, &config.jwt_secret, &config.jwt_issuer()).await?;
    let note = validate_toolkit_note(input.note)?;
    let tags = validate_toolkit_tags(&input.tags)?;
    let tool_id = resolve_bookmark_tool_id(&pool, &input.slug).await?;

    let result = sqlx::query(
        r#"
        UPDATE bookmarks
        SET note = $3, tags = $4, updated_at = now()
        WHERE tool_id = $1 AND user_id = $2
        "#,
    )
    .bind(tool_id)
    .bind(user.id)
    .bind(note)
    .bind(tags)
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to update toolkit item: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new(
            "save the tool before editing toolkit metadata",
        ));
    }

    Ok(())
}

/// Compare up to 3 public tools by slug, preserving the requested order.
#[server(CompareTools, "/api")]
pub async fn compare_tools(slugs: Vec<String>) -> Result<Vec<ToolComparisonView>, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let viewer = optional_session_result(
        session_from_parts(&parts, &pool, &config.jwt_secret, &config.jwt_issuer()).await,
    )?;
    let normalized = crate::discovery::normalize_compare_slugs(&slugs.join(","));
    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    // Batch-fetch all approved tools by slug in one query.
    let tools = sqlx::query_as::<_, Tool>(APPROVED_TOOLS_BY_SLUGS_SQL)
        .bind(&normalized)
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load tools: {e}")))?;
    let tools = sanitize_tools_for_public_response(tools);

    // Batch-check bookmarks for all slugs at once (if viewer is signed in).
    let bookmarked_slugs: std::collections::HashSet<String> = if let Some(user) = viewer.as_ref() {
        sqlx::query_scalar::<_, String>(BOOKMARKED_SLUGS_SQL)
            .bind(&normalized)
            .bind(user.id)
            .fetch_all(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("bookmark lookup failed: {e}")))?
            .into_iter()
            .collect()
    } else {
        std::collections::HashSet::new()
    };

    // Build a slug -> tool map for order-preserving assembly.
    let tool_map: std::collections::HashMap<String, Tool> =
        tools.into_iter().map(|t| (t.slug.clone(), t)).collect();

    let mut rows = Vec::new();
    for slug in &normalized {
        let Some(tool) = tool_map.get(slug) else {
            continue;
        };
        let official_links = list_public_official_links(&pool, tool.id).await?;
        let trust = verify_tool_trust(tool, &official_links);
        let viewer_bookmarked = bookmarked_slugs.contains(&tool.slug);
        rows.push(ToolComparisonView {
            tool: tool.clone(),
            official_links,
            trust_facts: trust.trust_facts,
            viewer_bookmarked,
        });
    }

    Ok(rows)
}

/// Batch comment counts for approved tools by slug.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_tool_comment_counts(
    pool: &sqlx::PgPool,
    slugs: &[String],
) -> Result<Vec<(String, i64)>, ServerFnError> {
    if slugs.is_empty() {
        return Ok(Vec::new());
    }

    let rows = sqlx::query_as::<_, (String, i64)>(TOOL_COMMENT_COUNTS_BY_SLUGS_SQL)
        .bind(slugs)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("comment counts failed: {e}")))?;

    Ok(rows)
}

/// Batch comment counts for approved tools by slug.
#[server(GetToolCommentCounts, "/api")]
pub async fn get_tool_comment_counts(
    slugs: Vec<String>,
) -> Result<Vec<(String, i64)>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;
    fetch_tool_comment_counts(&pool, &slugs).await
}
