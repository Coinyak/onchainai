//! Leptos server functions — public API used by pages and components.
// Goal harness deliverable AC2/AC5
// harness-round-7: 2026-06-25T19:10:00Z-functions
//!
//! These functions are auto-registered by the Leptos runtime and are
//! available to both server-rendered and hydrated components.

use crate::auth::session::{optional_session_result, SessionUser};
use crate::models::tool::{sanitize_tool_for_public_response, sanitize_tools_for_public_response};
use crate::models::{
    Category, Comment, FeaturedCard, SiteSettings, Source, Tool, ToolClaimRequest, ToolReport,
    ToolSubmission, ToolSubmissionPayload, TOOL_REPORT_REASONS,
};
use leptos::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::auth::guard::{require_admin, require_user};
#[cfg(feature = "ssr")]
use crate::auth::session::session_from_parts;
#[cfg(feature = "ssr")]
use crate::config::Config;
#[cfg(feature = "ssr")]
use crate::crawler::normalizer::base_slug;
#[cfg(feature = "ssr")]
use crate::crawler::relevance::{assess_relevance, RelevanceInput};
#[cfg(feature = "ssr")]
use crate::crawler::{self, default_source_registry_url};
#[cfg(feature = "ssr")]
use crate::install_safety::assess_install;
#[cfg(feature = "ssr")]
use crate::server::queries::TOOLS_APPROVED_WHERE;
#[cfg(feature = "ssr")]
use crate::server::rate_limit::{check_user_rate_limit, UserRateLimitAction};
use crate::server::secret_redaction::redact_secrets;
#[cfg(feature = "ssr")]
use crate::server::secret_redaction::redact_tool_for_admin;
#[cfg(feature = "ssr")]
use axum::http::request::Parts;

#[cfg(feature = "ssr")]
fn request_context() -> Result<(Parts, sqlx::PgPool, String, String), ServerFnError> {
    let parts = use_context::<Parts>()
        .ok_or_else(|| ServerFnError::new("request context not available"))?;
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;
    let config =
        use_context::<Config>().ok_or_else(|| ServerFnError::new("configuration not available"))?;
    let issuer = config.jwt_issuer();
    Ok((parts, pool, config.jwt_secret, issuer))
}

/// Current signed-in user, if any (from session cookie).
#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<Option<SessionUser>, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    optional_session_result(session_from_parts(&parts, &pool, &jwt_secret, &issuer).await)
}

/// Admin gate — returns the admin session or a generic "not found" error.
#[server(CheckAdminAccess, "/api")]
pub async fn check_admin_access() -> Result<SessionUser, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer)
        .await
        .map_err(ServerFnError::new)
}

/// Row shape for category listings with live approved-tool counts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct CategoryWithCount {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub description: String,
    pub sort_order: i32,
    pub count: i64,
}

impl CategoryWithCount {
    pub fn into_pair(self) -> (Category, i64) {
        (
            Category {
                id: self.id,
                label: self.label,
                icon: self.icon,
                description: self.description,
                sort_order: self.sort_order,
            },
            self.count,
        )
    }
}

/// Returns the public site settings singleton (slogan, description, MCP endpoint).
#[server(GetSiteSettings, "/api")]
pub async fn get_site_settings() -> Result<SiteSettings, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let settings = sqlx::query_as::<_, SiteSettings>("SELECT * FROM site_settings WHERE id = 1")
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load site settings: {e}")))?;

    Ok(settings)
}

/// Parse comma- or newline-separated crawler keywords.
pub(crate) fn parse_search_keywords(raw: &str) -> Vec<String> {
    raw.split([',', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Validate admin site settings input before persisting.
pub(crate) struct SiteSettingsValidationInput<'a> {
    pub site_name: &'a str,
    pub slogan: &'a str,
    pub description: &'a str,
    pub mcp_endpoint: &'a str,
    pub search_keywords: &'a [String],
    pub default_referral_bps: Option<i32>,
    pub default_referral_payout_address: Option<&'a str>,
    pub x402_builder_code: Option<&'a str>,
}

pub(crate) fn validate_update_site_settings_input(
    input: SiteSettingsValidationInput<'_>,
) -> Result<(), &'static str> {
    let name = input.site_name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err("site name must be 1–100 characters");
    }
    let slogan = input.slogan.trim();
    if slogan.is_empty() || slogan.len() > 200 {
        return Err("slogan must be 1–200 characters");
    }
    let description = input.description.trim();
    if description.is_empty() || description.len() > 500 {
        return Err("description must be 1–500 characters");
    }
    let mcp_endpoint = input.mcp_endpoint.trim();
    if mcp_endpoint.is_empty() || mcp_endpoint.len() > 200 {
        return Err("MCP endpoint must be 1–200 characters");
    }
    if input.search_keywords.is_empty() || input.search_keywords.len() > 50 {
        return Err("provide 1–50 search keywords");
    }
    for kw in input.search_keywords {
        let kw = kw.trim();
        if kw.is_empty() || kw.len() > 64 {
            return Err("each keyword must be 1–64 characters");
        }
        if !kw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err("keywords may only contain letters, numbers, hyphens, and underscores");
        }
    }
    if let Some(bps) = input.default_referral_bps {
        if !(0..=10_000).contains(&bps) {
            return Err("default referral bps must be 0–10000");
        }
    }
    if let Some(address) = input.default_referral_payout_address {
        let address = address.trim();
        if address.len() > 200 {
            return Err("default referral payout address must be 200 characters or fewer");
        }
    }
    if let Some(code) = input.x402_builder_code {
        let code = code.trim();
        if code.len() > 100 {
            return Err("x402 builder code must be 100 characters or fewer");
        }
    }
    Ok(())
}

/// Payload for admin site settings updates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateSiteSettingsPayload {
    pub site_name: String,
    pub slogan: String,
    pub description: String,
    pub mcp_endpoint: String,
    pub search_keywords_raw: String,
    pub allow_free_registration: bool,
    pub require_tool_approval: bool,
    pub allow_x402_registration: bool,
    pub default_referral_bps: Option<i32>,
    pub default_referral_payout_address: Option<String>,
    pub x402_builder_code: Option<String>,
}

/// Admin-only update of the `site_settings` singleton (id = 1).
#[server(UpdateSiteSettings, "/api")]
pub async fn update_site_settings(
    payload: UpdateSiteSettingsPayload,
) -> Result<SiteSettings, ServerFnError> {
    let keywords = parse_search_keywords(&payload.search_keywords_raw);
    if let Err(msg) = validate_update_site_settings_input(SiteSettingsValidationInput {
        site_name: &payload.site_name,
        slogan: &payload.slogan,
        description: &payload.description,
        mcp_endpoint: &payload.mcp_endpoint,
        search_keywords: &keywords,
        default_referral_bps: payload.default_referral_bps,
        default_referral_payout_address: payload.default_referral_payout_address.as_deref(),
        x402_builder_code: payload.x402_builder_code.as_deref(),
    }) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer)
        .await
        .map_err(ServerFnError::new)?;

    let settings = sqlx::query_as::<_, SiteSettings>(
        r#"
        UPDATE site_settings
        SET site_name = $1,
            slogan = $2,
            description = $3,
            mcp_endpoint = $4,
            search_keywords = $5,
            allow_free_registration = $6,
            require_tool_approval = $7,
            allow_x402_registration = $8,
            default_referral_bps = $9,
            default_referral_payout_address = $10,
            x402_builder_code = $11,
            updated_at = now()
        WHERE id = 1
        RETURNING *
        "#,
    )
    .bind(payload.site_name.trim())
    .bind(payload.slogan.trim())
    .bind(payload.description.trim())
    .bind(payload.mcp_endpoint.trim())
    .bind(&keywords)
    .bind(payload.allow_free_registration)
    .bind(payload.require_tool_approval)
    .bind(payload.allow_x402_registration)
    .bind(payload.default_referral_bps)
    .bind(
        payload
            .default_referral_payout_address
            .as_deref()
            .map(str::trim),
    )
    .bind(payload.x402_builder_code.as_deref().map(str::trim))
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to update site settings: {e}")))?;

    Ok(settings)
}

/// Returns the most recently added **approved** tools.
///
/// HOT order = higher `stars` first, then more recent `created_at`.
#[server(GetRecentTools, "/api")]
pub async fn get_recent_tools(limit: i64) -> Result<Vec<Tool>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let limit = limit.clamp(1, 100);
    let sql = format!(
        "SELECT * FROM tools WHERE {TOOLS_APPROVED_WHERE} ORDER BY stars DESC, created_at DESC LIMIT $1"
    );
    let tools = sqlx::query_as::<_, Tool>(&sql)
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
    let sql = format!(
        r#"
        SELECT c.id, c.label, c.icon, c.description, c.sort_order,
               COUNT(t.id) AS count
        FROM categories c
        LEFT JOIN tools t ON t.function = c.id AND t.{TOOLS_APPROVED_WHERE}
        GROUP BY c.id, c.label, c.icon, c.description, c.sort_order
        ORDER BY c.sort_order ASC
        "#
    );
    let rows = sqlx::query_as::<_, CategoryWithCount>(&sql)
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

    let mut sql = format!(
        r#"
        SELECT *
        FROM tools
        WHERE {TOOLS_APPROVED_WHERE}
          AND (
            to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
            @@ plainto_tsquery('english', $1)
          )
        "#
    );

    let mut idx = 2;
    if function.is_some() {
        sql.push_str(&format!(" AND function = ${idx}"));
        idx += 1;
    }
    if chain.is_some() {
        sql.push_str(&format!(" AND ${idx} = ANY(chains)"));
    }
    sql.push_str(" ORDER BY stars DESC, created_at DESC LIMIT 50");

    let mut q = sqlx::query_as::<_, Tool>(&sql).bind(&query);
    if let Some(f) = &function {
        q = q.bind(f);
    }
    if let Some(c) = &chain {
        q = q.bind(c);
    }

    let tools = q
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
    let sql = format!("SELECT * FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}");
    let tool = sqlx::query_as::<_, Tool>(&sql)
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
    validate_tool_filter_values("chain", &filters.chain)?;
    Ok(())
}

/// Validates browser tool-list request bounds (rejects out-of-range instead of clamping).
pub fn validate_tool_list_request(req: &ToolListRequest) -> Result<(), ServerFnError> {
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
    pub function: Vec<String>,
    pub asset_class: Vec<String>,
    pub actor: Vec<String>,
    pub tool_type: Vec<String>,
    pub status: Vec<String>,
    pub chain: Vec<String>,
}

fn append_tool_filters(sql: &mut String, filters: &ToolFilters, idx: &mut i32) {
    if !filters.function.is_empty() {
        sql.push_str(&format!(" AND function = ANY(${idx})"));
        *idx += 1;
    }
    if !filters.asset_class.is_empty() {
        sql.push_str(&format!(" AND asset_class = ANY(${idx})"));
        *idx += 1;
    }
    if !filters.actor.is_empty() {
        sql.push_str(&format!(" AND actor = ANY(${idx})"));
        *idx += 1;
    }
    if !filters.tool_type.is_empty() {
        sql.push_str(&format!(" AND type = ANY(${idx})"));
        *idx += 1;
    }
    if !filters.status.is_empty() {
        sql.push_str(&format!(" AND status = ANY(${idx})"));
        *idx += 1;
    }
    if !filters.chain.is_empty() {
        sql.push_str(&format!(" AND chains && ${idx}"));
        *idx += 1;
    }
}

/// Count approved tools with optional multi-axis filters.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_count_tools(
    pool: &sqlx::PgPool,
    filters: &ToolFilters,
) -> Result<i64, ServerFnError> {
    let mut sql = format!("SELECT COUNT(*) FROM tools WHERE {TOOLS_APPROVED_WHERE}");
    let mut idx = 1i32;
    append_tool_filters(&mut sql, filters, &mut idx);

    let mut q = sqlx::query_as::<_, (i64,)>(&sql);
    if !filters.function.is_empty() {
        q = q.bind(&filters.function);
    }
    if !filters.asset_class.is_empty() {
        q = q.bind(&filters.asset_class);
    }
    if !filters.actor.is_empty() {
        q = q.bind(&filters.actor);
    }
    if !filters.tool_type.is_empty() {
        q = q.bind(&filters.tool_type);
    }
    if !filters.status.is_empty() {
        q = q.bind(&filters.status);
    }
    if !filters.chain.is_empty() {
        q = q.bind(&filters.chain);
    }

    let count = q
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
    let sql = format!(
        r#"
        SELECT chain, COUNT(*) AS count
        FROM tools, UNNEST(chains) AS chain
        WHERE {TOOLS_APPROVED_WHERE}
        GROUP BY chain
        ORDER BY count DESC, chain ASC
        LIMIT $1
        "#
    );
    let rows = sqlx::query_as::<_, (String, i64)>(&sql)
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
    let order = match sort {
        "new" => "created_at DESC",
        "comments" => {
            "(SELECT COUNT(*)::bigint FROM comments cm WHERE cm.tool_id = tools.id) DESC, created_at DESC"
        }
        _ => "stars DESC, created_at DESC",
    };

    let has_query = query.is_some_and(|q| !q.trim().is_empty());
    let mut sql = format!("SELECT * FROM tools WHERE {TOOLS_APPROVED_WHERE}");
    let mut idx = 1i32;

    if has_query {
        sql.push_str(&format!(
            " AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')) @@ plainto_tsquery('english', ${idx})"
        ));
        idx += 1;
    }
    append_tool_filters(&mut sql, filters, &mut idx);
    sql.push_str(&format!(
        " ORDER BY {order} OFFSET ${idx} LIMIT ${}",
        idx + 1
    ));

    let mut q = sqlx::query_as::<_, Tool>(&sql);
    if let Some(text) = query.filter(|q| !q.trim().is_empty()) {
        q = q.bind(text);
    }
    if !filters.function.is_empty() {
        q = q.bind(&filters.function);
    }
    if !filters.asset_class.is_empty() {
        q = q.bind(&filters.asset_class);
    }
    if !filters.actor.is_empty() {
        q = q.bind(&filters.actor);
    }
    if !filters.tool_type.is_empty() {
        q = q.bind(&filters.tool_type);
    }
    if !filters.status.is_empty() {
        q = q.bind(&filters.status);
    }
    if !filters.chain.is_empty() {
        q = q.bind(&filters.chain);
    }
    q = q.bind(offset).bind(limit);

    let tools = q
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

/// Batch comment counts for approved tools by slug.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_tool_comment_counts(
    pool: &sqlx::PgPool,
    slugs: &[String],
) -> Result<Vec<(String, i64)>, ServerFnError> {
    if slugs.is_empty() {
        return Ok(Vec::new());
    }

    let sql = format!(
        r#"
        SELECT t.slug, COUNT(c.id)::bigint AS comment_count
        FROM tools t
        LEFT JOIN comments c ON c.tool_id = t.id
        WHERE t.slug = ANY($1) AND {TOOLS_APPROVED_WHERE}
        GROUP BY t.slug
        "#
    );
    let rows = sqlx::query_as::<_, (String, i64)>(&sql)
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

/// SQL for admin pending-tool review (AC5).
pub(crate) const LIST_PENDING_TOOLS_SQL: &str =
    "SELECT * FROM tools WHERE approval_status = 'pending' ORDER BY created_at DESC LIMIT $1";

/// List tools awaiting admin review (`approval_status = 'pending'`).
#[server(ListPendingTools, "/api")]
pub async fn list_pending_tools(limit: i64) -> Result<Vec<Tool>, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let tools = sqlx::query_as::<_, Tool>(LIST_PENDING_TOOLS_SQL)
        .bind(limit)
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to list pending tools: {e}")))?;

    Ok(tools)
}

/// Operator review queue identifiers.
pub const REVIEW_QUEUES: &[&str] = &[
    "new_candidate",
    "known_update",
    "needs_manual_research",
    "low_relevance",
    "reported",
    "high_risk_install",
];

/// SQL WHERE fragment for a review queue (testable without DB).
pub(crate) fn review_queue_where(queue: &str) -> Result<&'static str, &'static str> {
    match queue {
        "new_candidate" => Ok(
            "approval_status = 'pending' AND last_reviewed_at IS NULL AND quarantined_at IS NULL",
        ),
        "known_update" => Ok(
            "approval_status = 'approved' AND last_reviewed_at IS NOT NULL \
             AND updated_at > last_reviewed_at AND quarantined_at IS NULL",
        ),
        "needs_manual_research" => Ok(
            "approval_status IN ('pending', 'approved') AND relevance_status = 'needs_review' \
             AND crypto_relevance_score < 50 AND quarantined_at IS NULL",
        ),
        "low_relevance" => Ok(
            "approval_status = 'pending' AND relevance_status = 'rejected' AND quarantined_at IS NULL",
        ),
        "reported" => Ok(
            "id IN (SELECT DISTINCT tool_id FROM tool_reports WHERE status = 'open') \
             AND quarantined_at IS NULL",
        ),
        "high_risk_install" => Ok(
            "approval_status IN ('pending', 'approved') \
             AND install_risk_level IN ('high', 'critical') AND quarantined_at IS NULL",
        ),
        _ => Err("unknown review queue"),
    }
}

/// Stub duplicate candidate surfaced in review rows until dedupe table ships.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicateCandidateStub {
    pub slug: String,
    pub name: String,
}

/// Enriched review row for operator console queues.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewQueueItem {
    pub tool: Tool,
    pub duplicate_candidates: Vec<DuplicateCandidateStub>,
    pub lifecycle_state: String,
    pub claim_state: String,
}

/// Derive lifecycle label from tool fields (stub until lifecycle column exists).
pub(crate) fn derive_lifecycle_state(tool: &Tool) -> String {
    if tool.quarantined_at.is_some() {
        return "flagged".into();
    }
    match tool.approval_status.as_str() {
        "approved" => "public_unclaimed".into(),
        "pending" if tool.last_reviewed_at.is_none() => "candidate".into(),
        "pending" => "pending".into(),
        "rejected" => "delisted".into(),
        other => other.into(),
    }
}

/// Claim state from tool row (defaults to unclaimed when empty).
pub(crate) fn derive_claim_state(tool: &Tool) -> String {
    let state = tool.claim_state.trim();
    if state.is_empty() {
        "unclaimed".into()
    } else {
        state.to_string()
    }
}

/// Admin dashboard aggregate counts and crawler health.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminDashboardStats {
    pub pending_candidates: i64,
    pub known_updates: i64,
    pub high_risk_installs: i64,
    pub open_reports: i64,
    pub public_tool_count: i64,
    pub needs_manual_research: i64,
    pub low_relevance: i64,
    pub reported: i64,
    pub crawler_sources: Vec<CrawlerSourceView>,
}

/// Count open tool reports; returns 0 when the reports table is not migrated yet.
#[cfg(feature = "ssr")]
async fn count_open_reports(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM tool_reports WHERE status = 'open'")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

/// Operator dashboard stats — queue counts, public tools, crawler source health.
#[server(GetAdminDashboardStats, "/api")]
pub async fn get_admin_dashboard_stats() -> Result<AdminDashboardStats, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let counts = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, i64)>(
        r#"
        SELECT
          COUNT(*) FILTER (
            WHERE approval_status = 'pending'
              AND last_reviewed_at IS NULL
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status = 'approved'
              AND last_reviewed_at IS NOT NULL
              AND updated_at > last_reviewed_at
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status IN ('pending', 'approved')
              AND install_risk_level IN ('high', 'critical')
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status = 'approved'
              AND relevance_status = 'accepted'
              AND install_risk_level <> 'critical'
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status IN ('pending', 'approved')
              AND relevance_status = 'needs_review'
              AND crypto_relevance_score < 50
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status = 'pending'
              AND relevance_status = 'rejected'
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE id IN (SELECT DISTINCT tool_id FROM tool_reports WHERE status = 'open')
              AND quarantined_at IS NULL
          )::bigint
        FROM tools
        "#,
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to load dashboard counts: {e}")))?;

    let open_reports = count_open_reports(&pool).await;
    let crawler_sources = list_crawler_sources_inner(&pool).await?;

    Ok(AdminDashboardStats {
        pending_candidates: counts.0,
        known_updates: counts.1,
        high_risk_installs: counts.2,
        public_tool_count: counts.3,
        needs_manual_research: counts.4,
        low_relevance: counts.5,
        reported: counts.6,
        open_reports,
        crawler_sources,
    })
}

/// List tools in an operator review queue with enriched row metadata.
#[server(ListReviewQueue, "/api")]
pub async fn list_review_queue(
    queue: String,
    limit: i64,
) -> Result<Vec<ReviewQueueItem>, ServerFnError> {
    if let Err(msg) = review_queue_where(&queue) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let where_clause = review_queue_where(&queue).expect("validated above");
    let sql = format!("SELECT * FROM tools WHERE {where_clause} ORDER BY updated_at DESC LIMIT $1");
    let tools = sqlx::query_as::<_, Tool>(&sql)
        .bind(limit)
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to list review queue: {e}")))?;

    let mut items = Vec::with_capacity(tools.len());
    for tool in tools {
        let duplicates = fetch_duplicate_candidates(&pool, &tool).await?;
        items.push(ReviewQueueItem {
            lifecycle_state: derive_lifecycle_state(&tool),
            claim_state: derive_claim_state(&tool),
            duplicate_candidates: duplicates,
            tool: redact_tool_for_admin(tool),
        });
    }

    Ok(items)
}

#[cfg(feature = "ssr")]
async fn fetch_duplicate_candidates(
    pool: &sqlx::PgPool,
    tool: &Tool,
) -> Result<Vec<DuplicateCandidateStub>, ServerFnError> {
    let repo = tool.repo_url.as_deref().unwrap_or("");
    let rows = if repo.is_empty() {
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT slug, name
            FROM tools
            WHERE id <> $1
              AND approval_status = 'pending'
              AND lower(name) = lower($2)
            ORDER BY created_at DESC
            LIMIT 3
            "#,
        )
        .bind(tool.id)
        .bind(&tool.name)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT slug, name
            FROM tools
            WHERE id <> $1
              AND approval_status = 'pending'
              AND repo_url = $2
            ORDER BY created_at DESC
            LIMIT 3
            "#,
        )
        .bind(tool.id)
        .bind(repo)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| ServerFnError::new(format!("failed to load duplicate candidates: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|(slug, name)| DuplicateCandidateStub { slug, name })
        .collect())
}

/// Gated admin review payload — writes audit events and enforces publication gates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewToolPayload {
    pub slug: String,
    pub action: String,
    pub reason: String,
    pub override_reason: Option<String>,
    pub expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub snapshot_id: Option<uuid::Uuid>,
    pub recommendation_id: Option<uuid::Uuid>,
}

/// Whether a tool has at least one trustworthy publication URL or package id.
pub(crate) fn tool_has_trustworthy_url(tool: &Tool) -> bool {
    let valid_url = |value: &Option<String>| {
        value.as_ref().is_some_and(|u| {
            let trimmed = u.trim();
            trimmed.starts_with("https://") || trimmed.starts_with("http://")
        })
    };
    valid_url(&tool.repo_url)
        || valid_url(&tool.homepage)
        || tool
            .npm_package
            .as_ref()
            .is_some_and(|p| !p.trim().is_empty())
        || valid_url(&tool.mcp_endpoint)
}

/// Override reason is required when approving despite rejected relevance or critical install risk.
pub(crate) fn review_override_required(tool: &Tool) -> bool {
    tool.relevance_status == "rejected" || tool.install_risk_level == "critical"
}

/// Validate approval gates without touching the database.
pub(crate) fn validate_review_approval_gate(
    tool: &Tool,
    override_reason: Option<&str>,
) -> Result<(), &'static str> {
    if !tool_has_trustworthy_url(tool) {
        return Err("approval requires a repo, homepage, npm package, or MCP endpoint");
    }
    if review_override_required(tool) {
        if override_reason.map(str::trim).is_none_or(str::is_empty) {
            return Err("override reason required when overriding rejected relevance or critical install risk");
        }
        return Ok(());
    }
    if tool.relevance_status == "needs_review" {
        // Human approval in admin review implies relevance acceptance.
        return Ok(());
    }
    if tool.relevance_status != "accepted" {
        return Err("approval requires relevance status accepted");
    }
    Ok(())
}

/// Validate review action inputs without touching the database.
pub(crate) fn validate_review_action(action: &str, reason: &str) -> Result<(), &'static str> {
    const APPROVAL_ACTIONS: &[&str] = &[
        "approved",
        "rejected",
        "pending",
        "needs_info",
        "quarantine",
        "mark_verified",
        "mark_official",
    ];
    if !APPROVAL_ACTIONS.contains(&action) {
        return Err(
            "invalid review action (expected approved|rejected|pending|needs_info|quarantine|mark_verified|mark_official)",
        );
    }
    if action == "rejected" && reason.trim().is_empty() {
        return Err("rejection requires a non-empty reason");
    }
    if matches!(
        action,
        "needs_info" | "quarantine" | "mark_verified" | "mark_official"
    ) && reason.trim().is_empty()
    {
        return Err("review action requires a non-empty reason");
    }
    if action == "approved" && reason.trim().is_empty() {
        return Err("approval requires a non-empty reason");
    }
    Ok(())
}

/// Audit before/after labels for review events.
pub(crate) fn review_audit_statuses(tool: &Tool, action: &str) -> (String, String) {
    match action {
        "mark_verified" => (tool.status.clone(), "verified".into()),
        "mark_official" => (tool.status.clone(), "official".into()),
        "quarantine" => (
            if tool.quarantined_at.is_some() {
                "quarantined".into()
            } else {
                "active".into()
            },
            "quarantined".into(),
        ),
        "needs_info" => (tool.approval_status.clone(), "needs_info".into()),
        other => (tool.approval_status.clone(), other.to_string()),
    }
}

/// Validate admin approval inputs without touching the database.
pub(crate) fn validate_set_tool_approval_input(
    status: &str,
    reason: Option<&str>,
) -> Result<(), &'static str> {
    let reason_text = reason.map(str::trim).unwrap_or("");
    validate_review_action(
        status,
        if reason_text.is_empty() && status == "approved" {
            "legacy approval"
        } else {
            reason_text
        },
    )
}

/// Gated tool review — enforces publication gates, writes audit event, updates tool.
#[server(ReviewTool, "/api")]
pub async fn review_tool(payload: ReviewToolPayload) -> Result<(), ServerFnError> {
    if let Err(msg) = validate_review_action(&payload.action, &payload.reason) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let admin = require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("failed to start review transaction: {e}")))?;

    let tool = sqlx::query_as::<_, Tool>("SELECT * FROM tools WHERE slug = $1 FOR UPDATE")
        .bind(&payload.slug)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load tool: {e}")))?
        .ok_or_else(|| ServerFnError::new(format!("tool not found: {}", payload.slug)))?;

    if let Some(expected) = payload.expected_updated_at {
        if tool.updated_at != expected {
            return Err(ServerFnError::new(
                "tool was modified by another session; refresh and retry",
            ));
        }
    }

    if payload.action == "approved" {
        if let Err(msg) = validate_review_approval_gate(&tool, payload.override_reason.as_deref()) {
            return Err(ServerFnError::new(msg.to_string()));
        }
    }

    let (before_status, after_status) = review_audit_statuses(&tool, &payload.action);
    let rejection_reason = if payload.action == "rejected" {
        Some(payload.reason.trim().to_string())
    } else {
        None
    };

    let new_relevance = if payload.action == "approved"
        && (tool.relevance_status == "needs_review" || review_override_required(&tool))
    {
        "accepted"
    } else {
        tool.relevance_status.as_str()
    };

    sqlx::query(
        r#"
        INSERT INTO tool_review_events (
            tool_id, admin_id, action, reason, override_reason,
            before_status, after_status, snapshot_id, recommendation_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(tool.id)
    .bind(admin.id)
    .bind(&payload.action)
    .bind(payload.reason.trim())
    .bind(payload.override_reason.as_deref().map(str::trim))
    .bind(&before_status)
    .bind(&after_status)
    .bind(payload.snapshot_id)
    .bind(payload.recommendation_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to write review event: {e}")))?;

    let result = match payload.action.as_str() {
        "approved" | "rejected" | "pending" => {
            sqlx::query(
                r#"
                UPDATE tools
                SET approval_status = $1,
                    rejection_reason = $2,
                    relevance_status = $3,
                    last_reviewed_at = now(),
                    updated_at = now()
                WHERE slug = $4
                "#,
            )
            .bind(&payload.action)
            .bind(&rejection_reason)
            .bind(new_relevance)
            .bind(&payload.slug)
            .execute(&mut *tx)
            .await
        }
        "needs_info" => {
            sqlx::query(
                r#"
                UPDATE tools
                SET approval_status = 'pending',
                    rejection_reason = NULL,
                    last_reviewed_at = now(),
                    updated_at = now()
                WHERE slug = $1
                "#,
            )
            .bind(&payload.slug)
            .execute(&mut *tx)
            .await
        }
        "quarantine" => {
            sqlx::query(
                r#"
                UPDATE tools
                SET quarantined_at = now(),
                    last_reviewed_at = now(),
                    updated_at = now()
                WHERE slug = $1
                "#,
            )
            .bind(&payload.slug)
            .execute(&mut *tx)
            .await
        }
        "mark_verified" => {
            sqlx::query(
                r#"
                UPDATE tools
                SET status = 'verified',
                    last_reviewed_at = now(),
                    updated_at = now()
                WHERE slug = $1
                "#,
            )
            .bind(&payload.slug)
            .execute(&mut *tx)
            .await
        }
        "mark_official" => {
            sqlx::query(
                r#"
                UPDATE tools
                SET status = 'official',
                    last_reviewed_at = now(),
                    updated_at = now()
                WHERE slug = $1
                "#,
            )
            .bind(&payload.slug)
            .execute(&mut *tx)
            .await
        }
        _ => unreachable!("validated above"),
    }
    .map_err(|e| ServerFnError::new(format!("failed to update tool: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new(format!(
            "tool not found: {}",
            payload.slug
        )));
    }

    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("failed to commit review: {e}")))?;

    Ok(())
}

/// Approve or reject a tool by slug — legacy wrapper around gated `review_tool`.
#[server(SetToolApproval, "/api")]
pub async fn set_tool_approval(
    slug: String,
    status: String,
    reason: Option<String>,
) -> Result<(), ServerFnError> {
    if let Err(msg) = validate_set_tool_approval_input(&status, reason.as_deref()) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let review_reason = match reason {
        Some(r) if !r.trim().is_empty() => r,
        _ if status == "approved" => "Approved via legacy set_tool_approval".into(),
        _ => String::new(),
    };

    review_tool(ReviewToolPayload {
        slug,
        action: status,
        reason: review_reason,
        override_reason: None,
        expected_updated_at: None,
        snapshot_id: None,
        recommendation_id: None,
    })
    .await
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ReferralDashboardStats {
    pub x402_tools: i64,
    pub referral_enabled_tools: i64,
    pub attribution_events: i64,
    pub reported_settlements: i64,
}

#[server(GetReferralDashboardStats, "/api")]
pub async fn get_referral_dashboard_stats() -> Result<ReferralDashboardStats, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer)
        .await
        .map_err(ServerFnError::new)?;

    sqlx::query_as::<_, ReferralDashboardStats>(
        r#"
        SELECT
            (SELECT COUNT(*) FROM tools WHERE pricing = 'x402') AS x402_tools,
            (SELECT COUNT(*) FROM tools WHERE referral_enabled = true) AS referral_enabled_tools,
            (SELECT COUNT(*) FROM referral_events) AS attribution_events,
            (SELECT COUNT(*) FROM referral_events WHERE event_type = 'reported_settlement') AS reported_settlements
        "#,
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to load referral stats: {e}")))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateToolReferralPayload {
    pub slug: String,
    pub referral_enabled: bool,
    pub referral_bps: Option<i32>,
    pub referral_payout_address: Option<String>,
    pub referral_model: Option<String>,
    pub x402_pay_to_address: Option<String>,
    pub x402_builder_code: Option<String>,
    pub payment_verified: bool,
    pub x402_endpoint_verified: bool,
    pub price_verified: bool,
}

pub(crate) fn validate_tool_referral_payload(
    payload: &UpdateToolReferralPayload,
) -> Result<(), &'static str> {
    if payload.slug.trim().is_empty() {
        return Err("tool slug is required");
    }
    if let Some(bps) = payload.referral_bps {
        if !(0..=10_000).contains(&bps) {
            return Err("referral bps must be 0–10000");
        }
    }
    if let Some(model) = payload.referral_model.as_deref().map(str::trim) {
        if !model.is_empty() && model != "split" && model != "attribution" {
            return Err("referral model must be split or attribution");
        }
    }
    for value in [
        payload.referral_payout_address.as_deref(),
        payload.x402_pay_to_address.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.trim().len() > 200 {
            return Err("referral and pay-to addresses must be 200 characters or fewer");
        }
    }
    if let Some(code) = payload.x402_builder_code.as_deref() {
        if code.trim().len() > 100 {
            return Err("x402 builder code must be 100 characters or fewer");
        }
    }
    Ok(())
}

#[server(UpdateToolReferral, "/api")]
pub async fn update_tool_referral(
    payload: UpdateToolReferralPayload,
) -> Result<Tool, ServerFnError> {
    if let Err(msg) = validate_tool_referral_payload(&payload) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer)
        .await
        .map_err(ServerFnError::new)?;

    let tool = sqlx::query_as::<_, Tool>(
        r#"
        UPDATE tools
        SET referral_enabled = $1,
            referral_bps = $2,
            referral_payout_address = $3,
            referral_model = $4,
            x402_pay_to_address = $5,
            x402_builder_code = $6,
            payment_verified = $7,
            x402_endpoint_verified = $8,
            price_verified = $9,
            updated_at = now()
        WHERE slug = $10
        RETURNING *
        "#,
    )
    .bind(payload.referral_enabled)
    .bind(payload.referral_bps)
    .bind(normalize_optional_text(payload.referral_payout_address))
    .bind(normalize_optional_text(payload.referral_model))
    .bind(normalize_optional_text(payload.x402_pay_to_address))
    .bind(normalize_optional_text(payload.x402_builder_code))
    .bind(payload.payment_verified)
    .bind(payload.x402_endpoint_verified)
    .bind(payload.price_verified)
    .bind(payload.slug.trim())
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to update referral settings: {e}")))?
    .ok_or_else(|| ServerFnError::new(format!("tool not found: {}", payload.slug)))?;

    Ok(redact_tool_for_admin(tool))
}

/// Known crawler sources for the admin dashboard (merged with DB rows).
pub(crate) const CRAWLER_SOURCE_DEFS: &[(&str, &str)] = &[
    ("cryptoskill", "Every 6h"),
    ("github", "Hourly (+30m offset)"),
    ("npm", "Hourly"),
    ("web3-mcp-hub", "Every 12h"),
];

/// Admin crawler row — source status plus schedule hint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrawlerSourceView {
    pub name: String,
    pub url: String,
    pub schedule: String,
    pub last_crawled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub crawl_status: String,
    pub items_found: i32,
    pub error_message: Option<String>,
}

/// Build crawler source rows for admin views (shared by dashboard and crawler page).
#[cfg(feature = "ssr")]
async fn list_crawler_sources_inner(
    pool: &sqlx::PgPool,
) -> Result<Vec<CrawlerSourceView>, ServerFnError> {
    let rows = sqlx::query_as::<_, Source>("SELECT * FROM sources ORDER BY name ASC")
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to list sources: {e}")))?;

    let mut by_name: std::collections::HashMap<String, Source> =
        rows.into_iter().map(|r| (r.name.clone(), r)).collect();

    let mut views = Vec::with_capacity(CRAWLER_SOURCE_DEFS.len() + 1);
    for (name, schedule) in CRAWLER_SOURCE_DEFS {
        let url = default_source_registry_url(name).to_string();
        if let Some(row) = by_name.remove(*name) {
            views.push(CrawlerSourceView {
                name: row.name,
                url: row.url,
                schedule: (*schedule).into(),
                last_crawled_at: row.last_crawled_at,
                crawl_status: row.crawl_status,
                items_found: row.items_found,
                error_message: row.error_message,
            });
        } else {
            views.push(CrawlerSourceView {
                name: (*name).into(),
                url,
                schedule: (*schedule).into(),
                last_crawled_at: None,
                crawl_status: "pending".into(),
                items_found: 0,
                error_message: None,
            });
        }
    }

    for (_, row) in by_name {
        views.push(CrawlerSourceView {
            name: row.name,
            url: row.url,
            schedule: "—".into(),
            last_crawled_at: row.last_crawled_at,
            crawl_status: row.crawl_status,
            items_found: row.items_found,
            error_message: row.error_message,
        });
    }

    Ok(views)
}

/// List crawler source status (admin).
#[server(ListCrawlerSources, "/api")]
pub async fn list_crawler_sources() -> Result<Vec<CrawlerSourceView>, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;
    list_crawler_sources_inner(&pool).await
}

/// Validate manual crawler trigger input.
pub(crate) fn validate_trigger_crawler_source(source: &str) -> Result<(), &'static str> {
    match source {
        "npm" | "cryptoskill" | "web3-mcp-hub" | "github" | "sync_stars" => Ok(()),
        _ => Err("unknown crawler source"),
    }
}

/// Manually trigger a crawler job in the background (admin).
#[server(TriggerCrawlerSource, "/api")]
pub async fn trigger_crawler_source(source: String) -> Result<(), ServerFnError> {
    if let Err(msg) = validate_trigger_crawler_source(&source) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let pool_bg = pool.clone();
    let source_bg = source.clone();
    tokio::spawn(async move {
        crawler::trigger_source(&pool_bg, &source_bg).await;
    });

    Ok(())
}

/// Comment with author display fields and upvote count.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommentView {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub user_id: Uuid,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub author_nickname: Option<String>,
    pub author_auth_method: Option<String>,
    pub author_is_admin: bool,
    pub upvote_count: i64,
    pub viewer_upvoted: bool,
}

/// Validate comment body before insert.
pub(crate) fn validate_comment_content(content: &str) -> Result<(), &'static str> {
    let trimmed = content.trim();
    if trimmed.is_empty() || trimmed.len() > 2000 {
        return Err("comment must be 1–2000 characters");
    }
    Ok(())
}

/// List comments for an approved tool (`sort`: `new` | `top`).
#[server(GetToolComments, "/api")]
pub async fn get_tool_comments(
    slug: String,
    sort: String,
) -> Result<Vec<CommentView>, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let viewer = session_from_parts(&parts, &pool, &jwt_secret, &issuer)
        .await
        .ok()
        .flatten();

    let tool_id = sqlx::query_scalar::<_, Uuid>(&format!(
        "SELECT id FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}"
    ))
    .bind(&slug)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to resolve tool: {e}")))?
    .ok_or_else(|| ServerFnError::new(format!("tool not found: {slug}")))?;

    let order = match sort.as_str() {
        "top" => "COUNT(u.id) DESC, c.created_at DESC",
        "new" => "c.created_at DESC",
        _ => return Err(ServerFnError::new("sort must be 'new' or 'top'")),
    };
    let sql = format!(
        r#"
        SELECT
            c.id, c.tool_id, c.parent_id, c.user_id, c.content, c.created_at,
            p.nickname AS author_nickname,
            p.auth_method AS author_auth_method,
            p.is_admin AS author_is_admin,
            COUNT(u.id) AS upvote_count,
            BOOL_OR(u.user_id = $2) AS viewer_upvoted
        FROM comments c
        JOIN profiles p ON p.id = c.user_id
        LEFT JOIN upvotes u ON u.comment_id = c.id
        WHERE c.tool_id = $1
        GROUP BY c.id, p.nickname, p.auth_method, p.is_admin
        ORDER BY {order}
        "#
    );
    let rows = sqlx::query_as::<_, CommentRow>(&sql)
        .bind(tool_id)
        .bind(viewer.as_ref().map(|v| v.id))
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load comments: {e}")))?;

    Ok(rows.into_iter().map(CommentRow::into_view).collect())
}

#[cfg(feature = "ssr")]
#[derive(Debug, sqlx::FromRow)]
struct CommentRow {
    id: Uuid,
    tool_id: Uuid,
    parent_id: Option<Uuid>,
    user_id: Uuid,
    content: String,
    created_at: chrono::DateTime<chrono::Utc>,
    author_nickname: Option<String>,
    author_auth_method: Option<String>,
    author_is_admin: bool,
    upvote_count: Option<i64>,
    viewer_upvoted: Option<bool>,
}

#[cfg(feature = "ssr")]
impl CommentRow {
    fn into_view(self) -> CommentView {
        CommentView {
            id: self.id,
            tool_id: self.tool_id,
            parent_id: self.parent_id,
            user_id: self.user_id,
            content: self.content,
            created_at: self.created_at,
            author_nickname: self.author_nickname,
            author_auth_method: self.author_auth_method,
            author_is_admin: self.author_is_admin,
            upvote_count: self.upvote_count.unwrap_or(0),
            viewer_upvoted: self.viewer_upvoted.unwrap_or(false),
        }
    }
}

/// Count approved-tool comments (for list sort / badges).
#[server(GetToolCommentCount, "/api")]
pub async fn get_tool_comment_count(slug: String) -> Result<i64, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let count = sqlx::query_scalar::<_, i64>(&format!(
        r#"
        SELECT COUNT(*)::bigint
        FROM comments c
        JOIN tools t ON t.id = c.tool_id
        WHERE t.slug = $1 AND {TOOLS_APPROVED_WHERE}
        "#
    ))
    .bind(slug)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("comment count failed: {e}")))?;

    Ok(count)
}

/// Post a comment or reply (authenticated).
#[server(CreateComment, "/api")]
pub async fn create_comment(
    slug: String,
    content: String,
    parent_id: Option<Uuid>,
) -> Result<Comment, ServerFnError> {
    if let Err(msg) = validate_comment_content(&content) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let user = require_user(&parts, &pool, &jwt_secret, &issuer).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::CreateComment) {
        return Err(ServerFnError::new(limit.to_string()));
    }

    let tool_id = sqlx::query_scalar::<_, Uuid>(&format!(
        "SELECT id FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}"
    ))
    .bind(&slug)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to resolve tool: {e}")))?
    .ok_or_else(|| ServerFnError::new(format!("tool not found: {slug}")))?;

    if let Some(parent) = parent_id {
        let parent_row = sqlx::query_as::<_, (Option<Uuid>,)>(
            "SELECT parent_id FROM comments WHERE id = $1 AND tool_id = $2",
        )
        .bind(parent)
        .bind(tool_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("parent lookup failed: {e}")))?;

        match parent_row {
            Some((None,)) => {}
            Some((Some(_),)) => {
                return Err(ServerFnError::new("only one level of replies is allowed"));
            }
            None => return Err(ServerFnError::new("parent comment not found")),
        }
    }

    let comment = sqlx::query_as::<_, Comment>(
        r#"
        INSERT INTO comments (tool_id, parent_id, user_id, content)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(tool_id)
    .bind(parent_id)
    .bind(user.id)
    .bind(content.trim())
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to create comment: {e}")))?;

    Ok(comment)
}

/// Toggle upvote on a comment (authenticated).
#[server(ToggleUpvote, "/api")]
pub async fn toggle_upvote(comment_id: Uuid) -> Result<bool, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let user = require_user(&parts, &pool, &jwt_secret, &issuer).await?;

    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM upvotes WHERE comment_id = $1 AND user_id = $2",
    )
    .bind(comment_id)
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("upvote lookup failed: {e}")))?;

    if exists > 0 {
        sqlx::query("DELETE FROM upvotes WHERE comment_id = $1 AND user_id = $2")
            .bind(comment_id)
            .bind(user.id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("failed to remove upvote: {e}")))?;
        Ok(false)
    } else {
        sqlx::query("INSERT INTO upvotes (comment_id, user_id) VALUES ($1, $2)")
            .bind(comment_id)
            .bind(user.id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("failed to add upvote: {e}")))?;
        Ok(true)
    }
}

/// Whether the current user bookmarked a tool (false when signed out).
#[server(IsBookmarked, "/api")]
pub async fn is_bookmarked(slug: String) -> Result<bool, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let Some(user) =
        optional_session_result(session_from_parts(&parts, &pool, &jwt_secret, &issuer).await)?
    else {
        return Ok(false);
    };

    let bookmarked = sqlx::query_scalar::<_, i64>(&format!(
        r#"
        SELECT COUNT(*)::bigint
        FROM bookmarks b
        JOIN tools t ON t.id = b.tool_id
        WHERE t.slug = $1 AND b.user_id = $2 AND {TOOLS_APPROVED_WHERE}
        "#
    ))
    .bind(slug)
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("bookmark lookup failed: {e}")))?;

    Ok(bookmarked > 0)
}

/// Toggle bookmark on a tool (authenticated).
#[server(ToggleBookmark, "/api")]
pub async fn toggle_bookmark(slug: String) -> Result<bool, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let user = require_user(&parts, &pool, &jwt_secret, &issuer).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::ToggleBookmark) {
        return Err(ServerFnError::new(limit.to_string()));
    }

    let tool_id = sqlx::query_scalar::<_, Uuid>(&format!(
        "SELECT id FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}"
    ))
    .bind(&slug)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to resolve tool: {e}")))?
    .ok_or_else(|| ServerFnError::new(format!("tool not found: {slug}")))?;

    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM bookmarks WHERE tool_id = $1 AND user_id = $2",
    )
    .bind(tool_id)
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("bookmark lookup failed: {e}")))?;

    if exists > 0 {
        sqlx::query("DELETE FROM bookmarks WHERE tool_id = $1 AND user_id = $2")
            .bind(tool_id)
            .bind(user.id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("failed to remove bookmark: {e}")))?;
        Ok(false)
    } else {
        sqlx::query("INSERT INTO bookmarks (tool_id, user_id) VALUES ($1, $2)")
            .bind(tool_id)
            .bind(user.id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("failed to add bookmark: {e}")))?;
        Ok(true)
    }
}

/// Admin category row with approved-tool count.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCategoryView {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub description: String,
    pub sort_order: i32,
    pub tool_count: i64,
}

/// Validate category id/label/icon/description for admin CRUD.
pub(crate) fn validate_category_input(
    id: &str,
    label: &str,
    icon: &str,
    description: &str,
    sort_order: i32,
) -> Result<(), &'static str> {
    let id = id.trim();
    if id.len() < 2 || id.len() > 32 {
        return Err("category id must be 2–32 characters");
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("category id must be lowercase letters, digits, or hyphens");
    }
    let label = label.trim();
    if label.is_empty() || label.len() > 100 {
        return Err("label must be 1–100 characters");
    }
    let icon = icon.trim();
    if icon.is_empty() || icon.len() > 32 {
        return Err("icon must be 1–32 characters");
    }
    if !icon.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("icon may only contain letters, numbers, and hyphens");
    }
    let description = description.trim();
    if description.is_empty() || description.len() > 500 {
        return Err("description must be 1–500 characters");
    }
    if !(0..=9999).contains(&sort_order) {
        return Err("sort order must be 0–9999");
    }
    Ok(())
}

/// List all categories with tool counts (admin).
#[server(ListAdminCategories, "/api")]
pub async fn list_admin_categories() -> Result<Vec<AdminCategoryView>, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let rows = sqlx::query_as::<_, (String, String, String, String, i32, i64)>(
        r#"
        SELECT c.id, c.label, c.icon, c.description, c.sort_order,
               COUNT(t.id) AS tool_count
        FROM categories c
        LEFT JOIN tools t ON t.function = c.id
        GROUP BY c.id, c.label, c.icon, c.description, c.sort_order
        ORDER BY c.sort_order ASC
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to list categories: {e}")))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, label, icon, description, sort_order, tool_count)| AdminCategoryView {
                id,
                label,
                icon,
                description,
                sort_order,
                tool_count,
            },
        )
        .collect())
}

/// Create a function category (admin).
#[server(CreateCategory, "/api")]
pub async fn create_category(
    id: String,
    label: String,
    icon: String,
    description: String,
    sort_order: i32,
) -> Result<Category, ServerFnError> {
    if let Err(msg) = validate_category_input(&id, &label, &icon, &description, sort_order) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let category = sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO categories (id, label, icon, description, sort_order)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(id.trim())
    .bind(label.trim())
    .bind(icon.trim())
    .bind(description.trim())
    .bind(sort_order)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to create category: {e}")))?;

    Ok(category)
}

/// Update a function category (admin).
#[server(UpdateCategory, "/api")]
pub async fn update_category(
    id: String,
    label: String,
    icon: String,
    description: String,
    sort_order: i32,
) -> Result<Category, ServerFnError> {
    if let Err(msg) = validate_category_input(&id, &label, &icon, &description, sort_order) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let category = sqlx::query_as::<_, Category>(
        r#"
        UPDATE categories
        SET label = $2, icon = $3, description = $4, sort_order = $5
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id.trim())
    .bind(label.trim())
    .bind(icon.trim())
    .bind(description.trim())
    .bind(sort_order)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to update category: {e}")))?
    .ok_or_else(|| ServerFnError::new(format!("category not found: {id}")))?;

    Ok(category)
}

/// Delete a category when no tools reference it (admin).
#[server(DeleteCategory, "/api")]
pub async fn delete_category(id: String) -> Result<(), ServerFnError> {
    let id = id.trim().to_string();
    if id.is_empty() {
        return Err(ServerFnError::new("category id required"));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let tool_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tools WHERE function = $1")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("tool count failed: {e}")))?;

    if tool_count > 0 {
        return Err(ServerFnError::new(
            "cannot delete category with linked tools — reassign tools first",
        ));
    }

    let result = sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to delete category: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new(format!("category not found: {id}")));
    }

    Ok(())
}

/// Public featured carousel card joined to tool slug/name.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct FeaturedCardView {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub tool_slug: String,
    pub tool_name: String,
    pub image_url: String,
    pub headline: Option<String>,
    pub subtitle: Option<String>,
    pub sort_order: i32,
}

/// Admin featured card row with linked tool metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct AdminFeaturedCardView {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub tool_slug: String,
    pub tool_name: String,
    pub image_url: String,
    pub headline: Option<String>,
    pub subtitle: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Tool picker row for featured-card admin forms.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ToolPickerItem {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
}

/// Active featured cards for the public carousel (ordered).
#[server(GetFeaturedCards, "/api")]
pub async fn get_featured_cards() -> Result<Vec<FeaturedCardView>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let rows = sqlx::query_as::<_, FeaturedCardView>(
        r#"
        SELECT
            fc.id,
            fc.tool_id,
            t.slug AS tool_slug,
            t.name AS tool_name,
            fc.image_url,
            fc.headline,
            fc.subtitle,
            fc.sort_order
        FROM featured_cards fc
        INNER JOIN tools t ON t.id = fc.tool_id
        WHERE fc.is_active = true
          AND t.approval_status = 'approved'
        ORDER BY fc.sort_order ASC, fc.created_at ASC
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to load featured cards: {e}")))?;

    Ok(rows)
}

/// List all featured cards for admin management.
#[server(ListFeaturedCards, "/api")]
pub async fn list_featured_cards() -> Result<Vec<AdminFeaturedCardView>, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let rows = sqlx::query_as::<_, AdminFeaturedCardView>(
        r#"
        SELECT
            fc.id,
            fc.tool_id,
            t.slug AS tool_slug,
            t.name AS tool_name,
            fc.image_url,
            fc.headline,
            fc.subtitle,
            fc.sort_order,
            fc.is_active,
            fc.created_at,
            fc.updated_at
        FROM featured_cards fc
        INNER JOIN tools t ON t.id = fc.tool_id
        ORDER BY fc.sort_order ASC, fc.created_at ASC
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to list featured cards: {e}")))?;

    Ok(rows)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeaturedCardInput {
    pub tool_id: Uuid,
    pub image_url: String,
    pub headline: Option<String>,
    pub subtitle: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateFeaturedCardInput {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub image_url: String,
    pub headline: Option<String>,
    pub subtitle: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UploadFeaturedImageInput {
    pub filename: String,
    pub content_type: String,
    pub data_base64: String,
}

/// Create a featured carousel card (admin).
#[server(CreateFeaturedCard, "/api")]
pub async fn create_featured_card(input: FeaturedCardInput) -> Result<FeaturedCard, ServerFnError> {
    if let Err(msg) = validate_featured_card_input(
        &input.image_url,
        input.headline.as_deref(),
        input.subtitle.as_deref(),
    ) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    ensure_featured_tool_exists(&pool, input.tool_id).await?;

    let card = sqlx::query_as::<_, FeaturedCard>(
        r#"
        INSERT INTO featured_cards (tool_id, image_url, headline, subtitle, sort_order, is_active)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(input.tool_id)
    .bind(input.image_url.trim())
    .bind(normalize_optional_text(input.headline))
    .bind(normalize_optional_text(input.subtitle))
    .bind(input.sort_order)
    .bind(input.is_active)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to create featured card: {e}")))?;

    Ok(card)
}

/// Update a featured carousel card (admin).
#[server(UpdateFeaturedCard, "/api")]
pub async fn update_featured_card(
    input: UpdateFeaturedCardInput,
) -> Result<FeaturedCard, ServerFnError> {
    if let Err(msg) = validate_featured_card_input(
        &input.image_url,
        input.headline.as_deref(),
        input.subtitle.as_deref(),
    ) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    ensure_featured_tool_exists(&pool, input.tool_id).await?;

    let card = sqlx::query_as::<_, FeaturedCard>(
        r#"
        UPDATE featured_cards
        SET tool_id = $2,
            image_url = $3,
            headline = $4,
            subtitle = $5,
            sort_order = $6,
            is_active = $7
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(input.id)
    .bind(input.tool_id)
    .bind(input.image_url.trim())
    .bind(normalize_optional_text(input.headline))
    .bind(normalize_optional_text(input.subtitle))
    .bind(input.sort_order)
    .bind(input.is_active)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to update featured card: {e}")))?;

    Ok(card)
}

/// Delete a featured carousel card (admin).
#[server(DeleteFeaturedCard, "/api")]
pub async fn delete_featured_card(id: Uuid) -> Result<(), ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let result = sqlx::query("DELETE FROM featured_cards WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to delete featured card: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("featured card not found"));
    }

    Ok(())
}

/// Upload a featured-card image to Supabase Storage (admin).
#[server(UploadFeaturedImage, "/api")]
pub async fn upload_featured_image(
    input: UploadFeaturedImageInput,
) -> Result<String, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let config =
        use_context::<Config>().ok_or_else(|| ServerFnError::new("configuration not available"))?;

    let bytes = {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(input.data_base64.trim())
            .map_err(|e| ServerFnError::new(format!("invalid image encoding: {e}")))?
    };

    if let Err(msg) = validate_featured_image_upload(&input.content_type, bytes.len()) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let ext = featured_image_extension(&input.content_type, &input.filename)
        .ok_or_else(|| ServerFnError::new("unsupported image type"))?;
    let object_path = format!("{}.{}", Uuid::new_v4(), ext);
    let upload_url = format!(
        "{}/storage/v1/object/featured/{}",
        config.supabase_url.trim_end_matches('/'),
        object_path
    );
    let public_url = format!(
        "{}/storage/v1/object/public/featured/{}",
        config.supabase_url.trim_end_matches('/'),
        object_path
    );

    let response = reqwest::Client::new()
        .post(&upload_url)
        .header("apikey", &config.supabase_service_key)
        .header(
            "Authorization",
            format!("Bearer {}", config.supabase_service_key),
        )
        .header("Content-Type", &input.content_type)
        .header("x-upsert", "true")
        .body(bytes)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("image upload failed: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!(
            "storage upload failed ({status}): {body}"
        )));
    }

    Ok(public_url)
}

/// Search approved tools by name or slug for the featured-card picker (admin).
#[server(SearchToolsForPicker, "/api")]
pub async fn search_tools_for_picker(
    query: String,
    limit: i64,
) -> Result<Vec<ToolPickerItem>, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }

    let limit = limit.clamp(1, 50);
    let pattern = format!("%{q}%");
    let rows = sqlx::query_as::<_, ToolPickerItem>(
        r#"
        SELECT id, name, slug
        FROM tools
        WHERE approval_status = 'approved'
          AND (name ILIKE $1 OR slug ILIKE $1)
        ORDER BY stars DESC, name ASC
        LIMIT $2
        "#,
    )
    .bind(pattern)
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("tool search failed: {e}")))?;

    Ok(rows)
}

#[cfg(feature = "ssr")]
async fn ensure_featured_tool_exists(
    pool: &sqlx::PgPool,
    tool_id: Uuid,
) -> Result<(), ServerFnError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tools WHERE id = $1 AND approval_status = 'approved')",
    )
    .bind(tool_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ServerFnError::new(format!("tool lookup failed: {e}")))?;

    if exists {
        Ok(())
    } else {
        Err(ServerFnError::new("approved tool not found"))
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) const MAX_FEATURED_IMAGE_BYTES: usize = 5 * 1024 * 1024;

pub(crate) fn validate_featured_image_upload(
    content_type: &str,
    bytes_len: usize,
) -> Result<(), &'static str> {
    let allowed = ["image/jpeg", "image/png", "image/webp", "image/svg+xml"];
    if !allowed.contains(&content_type) {
        return Err("unsupported image type");
    }
    if bytes_len == 0 {
        return Err("image is empty");
    }
    if bytes_len > MAX_FEATURED_IMAGE_BYTES {
        return Err("image too large (max 5 MB)");
    }
    Ok(())
}

pub(crate) fn validate_featured_card_input(
    image_url: &str,
    headline: Option<&str>,
    subtitle: Option<&str>,
) -> Result<(), &'static str> {
    if image_url.trim().is_empty() {
        return Err("image URL is required");
    }
    if let Some(h) = headline {
        if h.chars().count() > 120 {
            return Err("headline is too long");
        }
    }
    if let Some(s) = subtitle {
        if s.chars().count() > 200 {
            return Err("subtitle is too long");
        }
    }
    Ok(())
}

pub(crate) fn featured_image_extension(content_type: &str, filename: &str) -> Option<&'static str> {
    match content_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        _ => filename
            .rsplit('.')
            .next()
            .and_then(|ext| match ext.to_ascii_lowercase().as_str() {
                "jpg" | "jpeg" => Some("jpg"),
                "png" => Some("png"),
                "webp" => Some("webp"),
                "svg" => Some("svg"),
                _ => None,
            }),
    }
}

/// Pure selection helper for tests — mirrors public featured-card ordering/filtering.
#[allow(dead_code)]
pub(crate) fn select_active_featured_cards(
    cards: &mut [FeaturedCardView],
) -> Vec<FeaturedCardView> {
    cards.sort_by(|a, b| {
        a.sort_order
            .cmp(&b.sort_order)
            .then_with(|| a.id.cmp(&b.id))
    });
    cards.to_vec()
}

/// Admin user row with activity counts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUserView {
    pub id: Uuid,
    pub nickname: Option<String>,
    pub auth_method: String,
    pub is_admin: bool,
    pub is_banned: bool,
    pub comment_count: i64,
    pub bookmark_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// List profiles for moderation (admin).
#[server(ListAdminUsers, "/api")]
pub async fn list_admin_users(
    query: Option<String>,
    limit: i64,
) -> Result<Vec<AdminUserView>, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let limit = limit.clamp(1, 100);
    let pattern = query
        .as_ref()
        .map(|q| q.trim())
        .filter(|q| !q.is_empty())
        .map(|q| format!("%{q}%"));

    let rows = if let Some(pat) = pattern {
        sqlx::query_as::<_, AdminUserRow>(
            r#"
            SELECT
                p.id, p.nickname, p.auth_method, p.is_admin, p.is_banned, p.created_at,
                (SELECT COUNT(*) FROM comments c WHERE c.user_id = p.id) AS comment_count,
                (SELECT COUNT(*) FROM bookmarks b WHERE b.user_id = p.id) AS bookmark_count
            FROM profiles p
            WHERE p.nickname ILIKE $1 OR p.auth_method ILIKE $1
            ORDER BY p.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(pat)
        .bind(limit)
        .fetch_all(&pool)
        .await
    } else {
        sqlx::query_as::<_, AdminUserRow>(
            r#"
            SELECT
                p.id, p.nickname, p.auth_method, p.is_admin, p.is_banned, p.created_at,
                (SELECT COUNT(*) FROM comments c WHERE c.user_id = p.id) AS comment_count,
                (SELECT COUNT(*) FROM bookmarks b WHERE b.user_id = p.id) AS bookmark_count
            FROM profiles p
            ORDER BY p.created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&pool)
        .await
    }
    .map_err(|e| ServerFnError::new(format!("failed to list users: {e}")))?;

    Ok(rows.into_iter().map(AdminUserRow::into_view).collect())
}

#[cfg(feature = "ssr")]
#[derive(Debug, sqlx::FromRow)]
struct AdminUserRow {
    id: Uuid,
    nickname: Option<String>,
    auth_method: String,
    is_admin: bool,
    is_banned: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    comment_count: Option<i64>,
    bookmark_count: Option<i64>,
}

#[cfg(feature = "ssr")]
impl AdminUserRow {
    fn into_view(self) -> AdminUserView {
        AdminUserView {
            id: self.id,
            nickname: self.nickname,
            auth_method: self.auth_method,
            is_admin: self.is_admin,
            is_banned: self.is_banned,
            comment_count: self.comment_count.unwrap_or(0),
            bookmark_count: self.bookmark_count.unwrap_or(0),
            created_at: self.created_at,
        }
    }
}

/// Ban or unban a user (admin).
#[server(SetUserBanned, "/api")]
pub async fn set_user_banned(user_id: Uuid, banned: bool) -> Result<(), ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let admin = require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    if admin.id == user_id {
        return Err(ServerFnError::new("cannot change your own ban status"));
    }

    let result =
        sqlx::query("UPDATE profiles SET is_banned = $1, updated_at = now() WHERE id = $2")
            .bind(banned)
            .bind(user_id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("failed to update ban status: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("user not found"));
    }

    Ok(())
}

/// Grant or revoke admin role (admin).
#[server(SetUserAdmin, "/api")]
pub async fn set_user_admin(user_id: Uuid, is_admin: bool) -> Result<(), ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let admin = require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    if admin.id == user_id && !is_admin {
        return Err(ServerFnError::new("cannot remove your own admin role"));
    }

    let result = sqlx::query("UPDATE profiles SET is_admin = $1, updated_at = now() WHERE id = $2")
        .bind(is_admin)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to update admin status: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("user not found"));
    }

    Ok(())
}

/// Delete a user profile and cascaded social data (admin).
#[server(DeleteUser, "/api")]
pub async fn delete_user(user_id: Uuid) -> Result<(), ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let admin = require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    if admin.id == user_id {
        return Err(ServerFnError::new("cannot delete your own account"));
    }

    let result = sqlx::query("DELETE FROM profiles WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to delete user: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("user not found"));
    }

    Ok(())
}

/// Admin comment row for moderation queue.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCommentView {
    pub id: Uuid,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub author_id: Uuid,
    pub author_nickname: Option<String>,
    pub author_is_banned: bool,
    pub tool_name: String,
    pub tool_slug: String,
}

/// List recent comments for moderation (admin).
#[server(ListAdminComments, "/api")]
pub async fn list_admin_comments(
    query: Option<String>,
    limit: i64,
) -> Result<Vec<AdminCommentView>, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let limit = limit.clamp(1, 100);
    let pattern = query
        .as_ref()
        .map(|q| q.trim())
        .filter(|q| !q.is_empty())
        .map(|q| format!("%{q}%"));

    let rows = if let Some(pat) = pattern {
        sqlx::query_as::<_, AdminCommentRow>(
            r#"
            SELECT
                c.id, c.content, c.created_at,
                p.id AS author_id, p.nickname AS author_nickname, p.is_banned AS author_is_banned,
                t.name AS tool_name, t.slug AS tool_slug
            FROM comments c
            JOIN profiles p ON p.id = c.user_id
            JOIN tools t ON t.id = c.tool_id
            WHERE c.content ILIKE $1 OR p.nickname ILIKE $1 OR t.name ILIKE $1
            ORDER BY c.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(pat)
        .bind(limit)
        .fetch_all(&pool)
        .await
    } else {
        sqlx::query_as::<_, AdminCommentRow>(
            r#"
            SELECT
                c.id, c.content, c.created_at,
                p.id AS author_id, p.nickname AS author_nickname, p.is_banned AS author_is_banned,
                t.name AS tool_name, t.slug AS tool_slug
            FROM comments c
            JOIN profiles p ON p.id = c.user_id
            JOIN tools t ON t.id = c.tool_id
            ORDER BY c.created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&pool)
        .await
    }
    .map_err(|e| ServerFnError::new(format!("failed to list comments: {e}")))?;

    Ok(rows.into_iter().map(AdminCommentRow::into_view).collect())
}

#[cfg(feature = "ssr")]
#[derive(Debug, sqlx::FromRow)]
struct AdminCommentRow {
    id: Uuid,
    content: String,
    created_at: chrono::DateTime<chrono::Utc>,
    author_id: Uuid,
    author_nickname: Option<String>,
    author_is_banned: bool,
    tool_name: String,
    tool_slug: String,
}

#[cfg(feature = "ssr")]
impl AdminCommentRow {
    fn into_view(self) -> AdminCommentView {
        AdminCommentView {
            id: self.id,
            content: redact_secrets(&self.content),
            created_at: self.created_at,
            author_id: self.author_id,
            author_nickname: self.author_nickname,
            author_is_banned: self.author_is_banned,
            tool_name: self.tool_name,
            tool_slug: self.tool_slug,
        }
    }
}

/// Delete a comment (admin).
#[server(DeleteAdminComment, "/api")]
pub async fn delete_admin_comment(comment_id: Uuid) -> Result<(), ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let result = sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(comment_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to delete comment: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("comment not found"));
    }

    Ok(())
}

/// Delete a comment and ban its author (admin).
#[server(DeleteCommentAndBanUser, "/api")]
pub async fn delete_comment_and_ban_user(comment_id: Uuid) -> Result<(), ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let admin = require_admin(&parts, &pool, &jwt_secret, &issuer).await?;

    let author_id = sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM comments WHERE id = $1")
        .bind(comment_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("comment lookup failed: {e}")))?
        .ok_or_else(|| ServerFnError::new("comment not found"))?;

    if author_id == admin.id {
        return Err(ServerFnError::new("cannot ban yourself"));
    }

    sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(comment_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to delete comment: {e}")))?;

    sqlx::query("UPDATE profiles SET is_banned = true, updated_at = now() WHERE id = $1")
        .bind(author_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to ban user: {e}")))?;

    Ok(())
}

/// Payload for public tool submission intake.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubmitToolInput {
    pub name: String,
    pub description: String,
    pub tool_type: String,
    pub function: String,
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
    pub npm_package: Option<String>,
    pub mcp_endpoint: Option<String>,
    pub install_command: Option<String>,
    pub chains_raw: String,
    pub category_suggestion: Option<String>,
    pub official_team_claim: bool,
    pub verification_note: Option<String>,
}

/// Payload for reporting a published listing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportToolInput {
    pub slug: String,
    pub reason: String,
    pub details: Option<String>,
}

/// Payload for requesting project claim (skeleton).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClaimToolInput {
    pub slug: String,
    pub verification_note: String,
    pub contact_email: Option<String>,
}

/// Scanned intake metadata attached to a submission row.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmissionScanResult {
    pub crypto_relevance_score: i32,
    pub relevance_status: String,
    pub install_risk_level: String,
}

const SUBMIT_TOOL_TYPES: &[&str] = &["mcp", "cli", "sdk", "api", "skill", "x402"];
const SUBMIT_FUNCTIONS: &[&str] = &[
    "bridge",
    "swap",
    "wallet",
    "payments",
    "lending",
    "staking",
    "trading",
    "nft",
    "data",
    "dev-tool",
    "identity",
    "governance",
    "social",
    "ai-agent",
];

/// Validate optional https URL (localhost http allowed for dev).
pub(crate) fn validate_optional_https_url(value: Option<&str>) -> Result<(), &'static str> {
    let Some(raw) = value.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    if raw.len() > 500 {
        return Err("URL must be at most 500 characters");
    }
    if raw.starts_with("https://") {
        return Ok(());
    }
    if raw.starts_with("http://localhost") || raw.starts_with("http://127.0.0.1") {
        return Ok(());
    }
    Err("URLs must use https:// (http://localhost allowed in dev)")
}

/// Parse comma-separated chain list from submission form.
pub(crate) fn parse_submission_chains(raw: &str) -> Vec<String> {
    raw.split([',', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .take(20)
        .collect()
}

/// Intake gate: minimally plausible submission (relevance gates public approval, not intake).
pub(crate) fn submission_is_minimally_plausible(input: &SubmitToolInput) -> bool {
    let name = input.name.trim();
    let description = input.description.trim();
    if name.len() < 2 || name.len() > 100 {
        return false;
    }
    if description.len() < 20 || description.len() > 500 {
        return false;
    }
    if !SUBMIT_TOOL_TYPES.contains(&input.tool_type.trim()) {
        return false;
    }
    if !SUBMIT_FUNCTIONS.contains(&input.function.trim()) {
        return false;
    }
    let has_link = [
        input.repo_url.as_deref(),
        input.homepage.as_deref(),
        input.npm_package.as_deref(),
        input.mcp_endpoint.as_deref(),
    ]
    .into_iter()
    .any(|v| v.is_some_and(|s| !s.trim().is_empty()));
    has_link
}

/// Validate submission form input.
pub(crate) fn validate_submit_tool_input(input: &SubmitToolInput) -> Result<(), &'static str> {
    if !submission_is_minimally_plausible(input) {
        return Err(
            "submission must include name (2–100), description (20–500), valid type/function, and at least one link",
        );
    }
    validate_optional_https_url(input.repo_url.as_deref())?;
    validate_optional_https_url(input.homepage.as_deref())?;
    validate_optional_https_url(input.mcp_endpoint.as_deref())?;
    if let Some(npm) = input
        .npm_package
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if npm.len() > 200 {
            return Err("npm package must be at most 200 characters");
        }
    }
    if let Some(cmd) = input
        .install_command
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if cmd.len() > 500 {
            return Err("install command must be at most 500 characters");
        }
        if cmd.contains('\n') || cmd.contains('\r') {
            return Err("install command must be a single line");
        }
    }
    if let Some(note) = input
        .verification_note
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if note.len() > 1000 {
            return Err("verification note must be at most 1000 characters");
        }
    }
    if let Some(cat) = input
        .category_suggestion
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if cat.len() > 100 {
            return Err("category suggestion must be at most 100 characters");
        }
    }
    Ok(())
}

/// Run relevance and install safety scanners on submission intake.
#[cfg(feature = "ssr")]
pub(crate) fn scan_submission(input: &SubmitToolInput) -> SubmissionScanResult {
    let chains = parse_submission_chains(&input.chains_raw);
    let relevance = assess_relevance(&RelevanceInput {
        name: input.name.trim(),
        description: Some(input.description.trim()),
        tool_type: input.tool_type.trim(),
        repo_url: input.repo_url.as_deref().map(str::trim),
        homepage: input.homepage.as_deref().map(str::trim),
        npm_package: input.npm_package.as_deref().map(str::trim),
        mcp_endpoint: input.mcp_endpoint.as_deref().map(str::trim),
        chains: &chains,
        source: "user_submission",
    });
    let install = assess_install(
        input.install_command.as_deref().map(str::trim),
        input.npm_package.as_deref().map(str::trim),
    );
    SubmissionScanResult {
        crypto_relevance_score: relevance.score,
        relevance_status: relevance.status,
        install_risk_level: install.risk_level,
    }
}

/// Validate report reason against allowlist.
pub(crate) fn validate_report_reason(reason: &str) -> Result<(), &'static str> {
    if TOOL_REPORT_REASONS.iter().any(|(k, _)| *k == reason) {
        Ok(())
    } else {
        Err("invalid report reason")
    }
}

/// Validate report details length.
pub(crate) fn validate_report_details(details: Option<&str>) -> Result<(), &'static str> {
    if let Some(text) = details.map(str::trim).filter(|s| !s.is_empty()) {
        if text.len() > 1000 {
            return Err("report details must be at most 1000 characters");
        }
    }
    Ok(())
}

/// Validate claim request input.
pub(crate) fn validate_claim_tool_input(input: &ClaimToolInput) -> Result<(), &'static str> {
    let slug = input.slug.trim();
    if slug.is_empty() || slug.len() > 120 {
        return Err("tool slug is required");
    }
    let note = input.verification_note.trim();
    if note.len() < 20 || note.len() > 2000 {
        return Err("verification note must be 20–2000 characters");
    }
    if let Some(email) = input
        .contact_email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if email.len() > 200 || !email.contains('@') {
            return Err("contact email is invalid");
        }
    }
    Ok(())
}

/// Submit a tool suggestion for operator review (authenticated, never directly public).
#[server(SubmitTool, "/api")]
pub async fn submit_tool(input: SubmitToolInput) -> Result<ToolSubmission, ServerFnError> {
    if let Err(msg) = validate_submit_tool_input(&input) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let user = require_user(&parts, &pool, &jwt_secret, &issuer).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::SubmitTool) {
        return Err(ServerFnError::new(limit.to_string()));
    }

    let scan = scan_submission(&input);
    let chains = parse_submission_chains(&input.chains_raw);
    let slug = base_slug(input.name.trim());

    let duplicate = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint FROM (
          SELECT slug FROM tools WHERE lower(slug) = lower($1)
          UNION ALL
          SELECT payload->>'slug' FROM tool_submissions
            WHERE status IN ('pending', 'needs_info')
              AND lower(payload->>'slug') = lower($1)
        ) d
        "#,
    )
    .bind(&slug)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("duplicate check failed: {e}")))?;

    if duplicate > 0 {
        return Err(ServerFnError::new(
            "a similar tool is already listed or pending review",
        ));
    }

    let payload = ToolSubmissionPayload {
        name: input.name.trim().to_string(),
        description: input.description.trim().to_string(),
        tool_type: input.tool_type.trim().to_string(),
        function: input.function.trim().to_string(),
        repo_url: input
            .repo_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        homepage: input
            .homepage
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        npm_package: input
            .npm_package
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        mcp_endpoint: input
            .mcp_endpoint
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        install_command: input
            .install_command
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        chains,
        category_suggestion: input
            .category_suggestion
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        official_team_claim: input.official_team_claim,
        verification_note: input
            .verification_note
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        slug,
    };

    let payload_json = serde_json::to_value(&payload)
        .map_err(|e| ServerFnError::new(format!("failed to encode submission: {e}")))?;

    let row = sqlx::query_as::<_, ToolSubmission>(
        r#"
        INSERT INTO tool_submissions (
          submitted_by, status, payload,
          crypto_relevance_score, relevance_status, install_risk_level
        )
        VALUES ($1, 'pending', $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(user.id)
    .bind(payload_json)
    .bind(scan.crypto_relevance_score)
    .bind(scan.relevance_status)
    .bind(scan.install_risk_level)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to save submission: {e}")))?;

    Ok(row)
}

/// List the current user's tool submissions.
#[server(ListMySubmissions, "/api")]
pub async fn list_my_submissions() -> Result<Vec<ToolSubmission>, ServerFnError> {
    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let user = require_user(&parts, &pool, &jwt_secret, &issuer).await?;

    let rows = sqlx::query_as::<_, ToolSubmission>(
        r#"
        SELECT * FROM tool_submissions
        WHERE submitted_by = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to list submissions: {e}")))?;

    Ok(rows)
}

/// Report a published listing (authenticated).
#[server(ReportTool, "/api")]
pub async fn report_tool(input: ReportToolInput) -> Result<ToolReport, ServerFnError> {
    if let Err(msg) = validate_report_reason(input.reason.trim()) {
        return Err(ServerFnError::new(msg.to_string()));
    }
    if let Err(msg) = validate_report_details(input.details.as_deref()) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let slug = input.slug.trim();
    if slug.is_empty() {
        return Err(ServerFnError::new("tool slug is required"));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let user = require_user(&parts, &pool, &jwt_secret, &issuer).await?;

    let tool_id = sqlx::query_scalar::<_, Uuid>(&format!(
        "SELECT id FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}"
    ))
    .bind(slug)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to resolve tool: {e}")))?
    .ok_or_else(|| ServerFnError::new("tool not found"))?;

    let details = input
        .details
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let row = sqlx::query_as::<_, ToolReport>(
        r#"
        INSERT INTO tool_reports (tool_id, reported_by, reason, details, status)
        VALUES ($1, $2, $3, $4, 'open')
        RETURNING *
        "#,
    )
    .bind(tool_id)
    .bind(user.id)
    .bind(input.reason.trim())
    .bind(details)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to save report: {e}")))?;

    Ok(row)
}

/// Request project claim for a listing (skeleton — sets claim_pending).
#[server(RequestToolClaim, "/api")]
pub async fn request_tool_claim(input: ClaimToolInput) -> Result<ToolClaimRequest, ServerFnError> {
    if let Err(msg) = validate_claim_tool_input(&input) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret, issuer) = request_context()?;
    let user = require_user(&parts, &pool, &jwt_secret, &issuer).await?;

    let tool = sqlx::query_as::<_, Tool>(&format!(
        "SELECT * FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}"
    ))
    .bind(input.slug.trim())
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to resolve tool: {e}")))?
    .ok_or_else(|| ServerFnError::new("tool not found"))?;

    if tool.claim_state == "claimed" {
        return Err(ServerFnError::new("this listing is already claimed"));
    }
    if tool.claim_state == "claim_pending" {
        return Err(ServerFnError::new(
            "a claim request is already pending review",
        ));
    }

    let contact_email = input
        .contact_email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("transaction failed: {e}")))?;

    let claim = sqlx::query_as::<_, ToolClaimRequest>(
        r#"
        INSERT INTO tool_claim_requests (tool_id, requested_by, verification_note, contact_email, status)
        VALUES ($1, $2, $3, $4, 'pending')
        RETURNING *
        "#,
    )
    .bind(tool.id)
    .bind(user.id)
    .bind(input.verification_note.trim())
    .bind(contact_email)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to save claim request: {e}")))?;

    sqlx::query("UPDATE tools SET claim_state = 'claim_pending', updated_at = now() WHERE id = $1")
        .bind(tool.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to update claim state: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("commit failed: {e}")))?;

    Ok(claim)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_list_request_serializes_filters_field() {
        let req = ToolListRequest {
            sort: "hot".into(),
            offset: 0,
            limit: 50,
            filters: ToolFilters {
                function: vec!["bridge".into()],
                ..Default::default()
            },
            query: Some("mcp".into()),
        };
        let json = serde_json::to_value(&req).expect("serialize request");
        assert!(json.get("filters").is_some());
        assert_eq!(json["sort"], "hot");

        let round_trip: ToolListRequest =
            serde_json::from_value(json).expect("deserialize request");
        assert_eq!(round_trip.sort, "hot");
        assert_eq!(round_trip.limit, 50);
        assert_eq!(round_trip.filters.function, vec!["bridge"]);
        assert_eq!(round_trip.query.as_deref(), Some("mcp"));
    }

    #[test]
    fn list_tools_limit_uses_max_cap_not_legacy_100() {
        assert_eq!(clamp_list_tools_limit(100), 100);
        assert_eq!(clamp_list_tools_limit(150), 150);
        assert_eq!(clamp_list_tools_limit(500), MAX_LIST_TOOLS_LIMIT);
        assert_eq!(clamp_list_tools_limit(501), MAX_LIST_TOOLS_LIMIT);
        assert_eq!(clamp_list_tools_limit(0), 1);
    }

    #[test]
    fn browser_visible_limit_page_two_is_cumulative_100() {
        assert_eq!(browser_visible_limit_for_page(2), 100);
        assert_eq!(browser_visible_limit_for_page(1), 50);
        assert_eq!(browser_visible_limit_for_page(0), 50);
    }

    #[test]
    fn clamp_browser_page_param_bounds_window() {
        assert_eq!(clamp_browser_page_param(0), 1);
        assert_eq!(clamp_browser_page_param(2), 2);
        assert_eq!(clamp_browser_page_param(99), 10);
    }

    #[test]
    fn tool_list_request_limit_500_accepted() {
        let req = ToolListRequest {
            sort: "hot".into(),
            offset: 0,
            limit: MAX_LIST_TOOLS_LIMIT,
            filters: ToolFilters::default(),
            query: None,
        };
        assert!(validate_tool_list_request(&req).is_ok());
    }

    #[test]
    fn tool_list_request_limit_501_rejected() {
        let req = ToolListRequest {
            sort: "hot".into(),
            offset: 0,
            limit: MAX_LIST_TOOLS_LIMIT + 1,
            filters: ToolFilters::default(),
            query: None,
        };
        let err = validate_tool_list_request(&req).expect_err("limit 501 should fail");
        assert!(err.to_string().contains("limit must be between 1 and 500"));
    }

    #[test]
    fn append_tool_filters_supports_multi_select_any() {
        let mut sql = String::from("SELECT * FROM tools WHERE true");
        let mut idx = 1;
        let filters = ToolFilters {
            function: vec!["bridge".into(), "swap".into()],
            ..Default::default()
        };
        append_tool_filters(&mut sql, &filters, &mut idx);
        assert!(sql.contains("function = ANY($1)"));
    }

    #[test]
    fn list_tools_comments_sort_uses_comment_count() {
        let order = "(SELECT COUNT(*)::bigint FROM comments cm WHERE cm.tool_id = tools.id) DESC, created_at DESC";
        assert!(order.contains("comments cm"));
        assert!(order.contains("COUNT(*)"));
    }

    #[test]
    fn public_tool_where_used_in_public_queries() {
        let recent = format!(
            "SELECT * FROM tools WHERE {TOOLS_APPROVED_WHERE} ORDER BY stars DESC, created_at DESC LIMIT $1"
        );
        assert!(recent.contains("approval_status = 'approved'"));
        assert!(recent.contains("relevance_status = 'accepted'"));
        assert!(recent.contains("install_risk_level <> 'critical'"));
        assert!(recent.contains("quarantined_at IS NULL"));

        let categories =
            format!("LEFT JOIN tools t ON t.function = c.id AND t.{TOOLS_APPROVED_WHERE}");
        assert!(categories.contains("relevance_status = 'accepted'"));
    }

    fn sample_review_tool() -> Tool {
        let review = crate::models::tool::default_review_fields();
        Tool {
            id: Uuid::nil(),
            name: "Bridge MCP".into(),
            slug: "bridge-mcp".into(),
            description: Some("Ethereum bridge tool".into()),
            function: "bridge".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "mcp".into(),
            repo_url: Some("https://github.com/example/bridge".into()),
            homepage: None,
            npm_package: None,
            install_command: None,
            mcp_endpoint: None,
            chains: vec![],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "pending".into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: review.crypto_relevance_score,
            crypto_relevance_reasons: review.crypto_relevance_reasons,
            relevance_status: review.relevance_status,
            install_risk_level: review.install_risk_level,
            install_risk_reasons: review.install_risk_reasons,
            requires_secret: review.requires_secret,
            safe_copy_command: review.safe_copy_command,
            quarantined_at: review.quarantined_at,
            last_reviewed_at: review.last_reviewed_at,
            review_policy_version: review.review_policy_version,
            claim_state: "unclaimed".into(),
            license: None,
            pricing: "free".into(),
            x402_price: None,
            referral_enabled: false,
            referral_bps: None,
            referral_payout_address: None,
            referral_model: None,
            x402_pay_to_address: None,
            x402_builder_code: None,
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
            stars: 0,
            last_commit_at: None,
            source: "github".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn review_approval_gate_requires_trustworthy_url() {
        let mut tool = sample_review_tool();
        tool.repo_url = None;
        assert_eq!(
            validate_review_approval_gate(&tool, None),
            Err("approval requires a repo, homepage, npm package, or MCP endpoint")
        );
    }

    #[test]
    fn review_approval_gate_allows_needs_review_with_url() {
        let tool = sample_review_tool();
        assert!(validate_review_approval_gate(&tool, None).is_ok());
    }

    #[test]
    fn review_approval_gate_requires_override_for_rejected_relevance() {
        let mut tool = sample_review_tool();
        tool.relevance_status = "rejected".into();
        assert_eq!(
            validate_review_approval_gate(&tool, None),
            Err("override reason required when overriding rejected relevance or critical install risk")
        );
        assert!(validate_review_approval_gate(&tool, Some("operator override")).is_ok());
    }

    #[test]
    fn review_approval_gate_requires_override_for_critical_install() {
        let mut tool = sample_review_tool();
        tool.install_risk_level = "critical".into();
        assert_eq!(
            validate_review_approval_gate(&tool, None),
            Err("override reason required when overriding rejected relevance or critical install risk")
        );
    }

    #[test]
    fn review_override_required_detects_rejected_and_critical() {
        let mut tool = sample_review_tool();
        assert!(!review_override_required(&tool));
        tool.relevance_status = "rejected".into();
        assert!(review_override_required(&tool));
        tool.relevance_status = "accepted".into();
        tool.install_risk_level = "critical".into();
        assert!(review_override_required(&tool));
    }

    #[test]
    fn tool_has_trustworthy_url_accepts_repo_or_npm() {
        let mut tool = sample_review_tool();
        assert!(tool_has_trustworthy_url(&tool));
        tool.repo_url = None;
        tool.npm_package = Some("@example/pkg".into());
        assert!(tool_has_trustworthy_url(&tool));
    }

    #[test]
    fn set_tool_approval_validation_accepts_approved_and_pending() {
        assert!(validate_set_tool_approval_input("approved", None).is_ok());
        assert!(validate_set_tool_approval_input("pending", None).is_ok());
    }

    #[test]
    fn set_tool_approval_validation_rejects_without_reason() {
        assert_eq!(
            validate_set_tool_approval_input("rejected", None),
            Err("rejection requires a non-empty reason")
        );
        assert_eq!(
            validate_set_tool_approval_input("rejected", Some("   ")),
            Err("rejection requires a non-empty reason")
        );
    }

    #[test]
    fn set_tool_approval_validation_rejects_invalid_status() {
        assert!(validate_set_tool_approval_input("published", None).is_err());
    }

    #[test]
    fn list_pending_tools_sql_filters_pending_only() {
        assert!(LIST_PENDING_TOOLS_SQL.contains("approval_status = 'pending'"));
        assert!(!LIST_PENDING_TOOLS_SQL.contains("approved"));
    }

    #[test]
    fn review_queue_where_covers_all_queues() {
        for queue in REVIEW_QUEUES {
            assert!(
                review_queue_where(queue).is_ok(),
                "missing where for {queue}"
            );
        }
        assert_eq!(review_queue_where("unknown"), Err("unknown review queue"));
    }

    #[test]
    fn derive_lifecycle_state_maps_pending_and_quarantine() {
        let mut tool = sample_review_tool();
        assert_eq!(derive_lifecycle_state(&tool), "candidate");
        tool.last_reviewed_at = Some(chrono::Utc::now());
        assert_eq!(derive_lifecycle_state(&tool), "pending");
        tool.quarantined_at = Some(chrono::Utc::now());
        assert_eq!(derive_lifecycle_state(&tool), "flagged");
    }

    #[test]
    fn derive_claim_state_reads_tool_column() {
        let mut tool = sample_review_tool();
        assert_eq!(derive_claim_state(&tool), "unclaimed");
        tool.claim_state = "claim_pending".into();
        assert_eq!(derive_claim_state(&tool), "claim_pending");
    }

    fn sample_submit_input() -> SubmitToolInput {
        SubmitToolInput {
            name: "Bridge MCP".into(),
            description: "Ethereum bridge MCP server for crypto agents.".into(),
            tool_type: "mcp".into(),
            function: "bridge".into(),
            repo_url: Some("https://github.com/example/bridge".into()),
            homepage: None,
            npm_package: None,
            mcp_endpoint: None,
            install_command: Some("npm i @example/bridge-mcp".into()),
            chains_raw: "ethereum, arbitrum".into(),
            category_suggestion: None,
            official_team_claim: false,
            verification_note: None,
        }
    }

    #[test]
    fn validate_submit_tool_accepts_minimally_plausible_crypto_tool() {
        assert!(validate_submit_tool_input(&sample_submit_input()).is_ok());
    }

    #[test]
    fn validate_submit_tool_rejects_without_link() {
        let mut input = sample_submit_input();
        input.repo_url = None;
        assert!(validate_submit_tool_input(&input).is_err());
    }

    #[test]
    fn validate_submit_tool_rejects_short_description() {
        let mut input = sample_submit_input();
        input.description = "too short".into();
        assert!(validate_submit_tool_input(&input).is_err());
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn scan_submission_runs_relevance_and_install_scanners() {
        let scan = scan_submission(&sample_submit_input());
        assert!(scan.crypto_relevance_score > 0);
        assert!(!scan.relevance_status.is_empty());
        assert_eq!(scan.install_risk_level, "low");
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn scan_submission_accepts_low_relevance_intake() {
        let mut input = sample_submit_input();
        input.name = "Generic Helper".into();
        input.description = "A generic helper tool without crypto terms.".into();
        input.repo_url = Some("https://example.com".into());
        let scan = scan_submission(&input);
        assert!(scan.relevance_status == "needs_review" || scan.relevance_status == "rejected");
        assert!(validate_submit_tool_input(&input).is_ok());
    }

    #[test]
    fn validate_report_reason_accepts_allowlist() {
        assert!(validate_report_reason("scam_phishing").is_ok());
        assert!(validate_report_reason("broken_link").is_ok());
        assert!(validate_report_reason("invalid").is_err());
    }

    #[test]
    fn validate_claim_tool_input_bounds() {
        assert!(validate_claim_tool_input(&ClaimToolInput {
            slug: "bridge-mcp".into(),
            verification_note: "I maintain this repo and can verify via DNS TXT.".into(),
            contact_email: Some("team@example.com".into()),
        })
        .is_ok());
        assert!(validate_claim_tool_input(&ClaimToolInput {
            slug: "bridge-mcp".into(),
            verification_note: "short".into(),
            contact_email: None,
        })
        .is_err());
    }

    #[test]
    fn review_queue_where_reported_uses_open_reports() {
        let where_clause = review_queue_where("reported").expect("reported queue");
        assert!(where_clause.contains("tool_reports"));
        assert!(where_clause.contains("status = 'open'"));
    }

    #[test]
    fn validate_review_action_accepts_operator_actions() {
        assert!(validate_review_action("needs_info", "more context").is_ok());
        assert!(validate_review_action("quarantine", "unsafe install").is_ok());
        assert!(validate_review_action("mark_verified", "checked repo").is_ok());
        assert!(validate_review_action("mark_official", "official domain").is_ok());
        assert!(validate_review_action("needs_info", "   ").is_err());
    }

    #[test]
    fn review_audit_statuses_tracks_trust_and_quarantine() {
        let tool = sample_review_tool();
        assert_eq!(
            review_audit_statuses(&tool, "mark_verified"),
            ("community".into(), "verified".into())
        );
        assert_eq!(
            review_audit_statuses(&tool, "needs_info"),
            ("pending".into(), "needs_info".into())
        );
    }

    #[test]
    fn parse_search_keywords_splits_commas_and_newlines() {
        assert_eq!(
            parse_search_keywords("mcp-server, crypto-mcp\nweb3-mcp"),
            vec![
                "mcp-server".to_string(),
                "crypto-mcp".to_string(),
                "web3-mcp".to_string()
            ]
        );
    }

    #[test]
    fn validate_site_settings_accepts_defaults() {
        let keywords = vec!["mcp-server".into()];
        assert!(
            validate_update_site_settings_input(SiteSettingsValidationInput {
                site_name: "OnchainAI",
                slogan: "Crypto tools, unified.",
                description: "Discover tools.",
                mcp_endpoint: "npx mcp-remote www.onchain-ai.xyz/mcp",
                search_keywords: &keywords,
                default_referral_bps: Some(250),
                default_referral_payout_address: Some("0x0000000000000000000000000000000000000000"),
                x402_builder_code: Some("onchainai"),
            })
            .is_ok()
        );
    }

    #[test]
    fn validate_site_settings_rejects_empty_keywords() {
        assert!(
            validate_update_site_settings_input(SiteSettingsValidationInput {
                site_name: "OnchainAI",
                slogan: "Slogan",
                description: "Description here.",
                mcp_endpoint: "npx mcp-remote",
                search_keywords: &[],
                default_referral_bps: None,
                default_referral_payout_address: None,
                x402_builder_code: None,
            })
            .is_err()
        );
    }

    #[test]
    fn validate_site_settings_rejects_invalid_keyword_chars() {
        let keywords = vec!["bad keyword".into()];
        assert!(
            validate_update_site_settings_input(SiteSettingsValidationInput {
                site_name: "OnchainAI",
                slogan: "Slogan",
                description: "Description here.",
                mcp_endpoint: "npx mcp-remote",
                search_keywords: &keywords,
                default_referral_bps: None,
                default_referral_payout_address: None,
                x402_builder_code: None,
            })
            .is_err()
        );
    }

    #[test]
    fn validate_tool_referral_payload_allows_unverified_x402_referral() {
        assert!(validate_tool_referral_payload(&UpdateToolReferralPayload {
            slug: "paid-tool".into(),
            referral_enabled: true,
            referral_bps: Some(250),
            referral_payout_address: Some("0x0000000000000000000000000000000000000000".into()),
            referral_model: Some("attribution".into()),
            x402_pay_to_address: Some("0x1111111111111111111111111111111111111111".into()),
            x402_builder_code: Some("onchainai".into()),
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
        })
        .is_ok());
    }

    #[test]
    fn validate_tool_referral_payload_rejects_bad_bps_and_model() {
        let mut payload = UpdateToolReferralPayload {
            slug: "paid-tool".into(),
            referral_enabled: true,
            referral_bps: Some(10_001),
            referral_payout_address: None,
            referral_model: Some("mystery".into()),
            x402_pay_to_address: None,
            x402_builder_code: None,
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
        };
        assert!(validate_tool_referral_payload(&payload).is_err());
        payload.referral_bps = Some(100);
        assert!(validate_tool_referral_payload(&payload).is_err());
        payload.referral_model = Some("split".into());
        assert!(validate_tool_referral_payload(&payload).is_ok());
    }

    #[test]
    fn validate_trigger_crawler_source_accepts_known_sources() {
        assert!(validate_trigger_crawler_source("npm").is_ok());
        assert!(validate_trigger_crawler_source("sync_stars").is_ok());
    }

    #[test]
    fn admin_review_queue_redacts_secrets_in_tool_json() {
        use crate::server::secret_redaction::assert_json_has_no_secrets;

        let review_fields = crate::models::tool::default_review_fields();
        let tool = Tool {
            id: Uuid::new_v4(),
            name: "Leak test".into(),
            slug: "leak-test".into(),
            description: Some("SUPABASE_SERVICE_KEY=leaked-service-key".into()),
            function: "bridge".into(),
            asset_class: "multi".into(),
            actor: "agent".into(),
            tool_type: "mcp".into(),
            repo_url: None,
            homepage: None,
            npm_package: None,
            install_command: Some("GITHUB_CLIENT_SECRET=leaked-client-secret".into()),
            mcp_endpoint: None,
            chains: vec![],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "pending".into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: 0,
            crypto_relevance_reasons: vec![],
            relevance_status: "needs_review".into(),
            install_risk_level: "low".into(),
            install_risk_reasons: vec![],
            requires_secret: false,
            safe_copy_command: None,
            quarantined_at: None,
            last_reviewed_at: None,
            review_policy_version: review_fields.review_policy_version,
            claim_state: "unclaimed".into(),
            license: None,
            pricing: "free".into(),
            x402_price: None,
            referral_enabled: false,
            referral_bps: None,
            referral_payout_address: None,
            referral_model: None,
            x402_pay_to_address: None,
            x402_builder_code: None,
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
            stars: 0,
            last_commit_at: None,
            source: "manual".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let item = ReviewQueueItem {
            tool: redact_tool_for_admin(tool),
            duplicate_candidates: vec![],
            lifecycle_state: "candidate".into(),
            claim_state: "unclaimed".into(),
        };
        let json = serde_json::to_string(&item).expect("serialize");
        assert_json_has_no_secrets(&json);
        assert!(!json.contains("leaked-service-key"));
        assert!(!json.contains("leaked-client-secret"));
    }

    #[test]
    fn validate_trigger_crawler_source_rejects_unknown() {
        assert!(validate_trigger_crawler_source("unknown").is_err());
    }

    #[test]
    fn validate_comment_content_bounds() {
        assert!(validate_comment_content("hello").is_ok());
        assert!(validate_comment_content("").is_err());
        assert!(validate_comment_content(&"x".repeat(2001)).is_err());
    }

    #[test]
    fn validate_category_input_accepts_slug_id() {
        assert!(validate_category_input(
            "my-cat",
            "My Category",
            "git-branch",
            "A test category.",
            10
        )
        .is_ok());
    }

    #[test]
    fn validate_category_input_rejects_uppercase_id() {
        assert!(validate_category_input("Bad-ID", "Label", "icon", "Description.", 1).is_err());
    }

    #[test]
    fn validate_featured_image_upload_accepts_png_within_limit() {
        assert!(validate_featured_image_upload("image/png", 1024).is_ok());
    }

    #[test]
    fn validate_featured_image_upload_rejects_oversized_and_bad_type() {
        assert!(validate_featured_image_upload("image/png", MAX_FEATURED_IMAGE_BYTES + 1).is_err());
        assert!(validate_featured_image_upload("application/pdf", 100).is_err());
        assert!(validate_featured_image_upload("image/png", 0).is_err());
    }

    #[test]
    fn validate_featured_card_input_bounds() {
        assert!(validate_featured_card_input(
            "https://cdn.example/card.png",
            Some("Headline"),
            None
        )
        .is_ok());
        assert!(validate_featured_card_input("   ", None, None).is_err());
        assert!(validate_featured_card_input(
            "https://cdn.example/card.png",
            Some(&"x".repeat(121)),
            None
        )
        .is_err());
    }

    #[test]
    fn select_active_featured_cards_orders_by_sort_order() {
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let mut cards = vec![
            FeaturedCardView {
                id: id_b,
                tool_id: Uuid::new_v4(),
                tool_slug: "b".into(),
                tool_name: "B".into(),
                image_url: "https://cdn.example/b.png".into(),
                headline: None,
                subtitle: None,
                sort_order: 2,
            },
            FeaturedCardView {
                id: id_a,
                tool_id: Uuid::new_v4(),
                tool_slug: "a".into(),
                tool_name: "A".into(),
                image_url: "https://cdn.example/a.png".into(),
                headline: None,
                subtitle: None,
                sort_order: 1,
            },
        ];
        let ordered = select_active_featured_cards(&mut cards);
        assert_eq!(ordered[0].tool_slug, "a");
        assert_eq!(ordered[1].tool_slug, "b");
    }
}
