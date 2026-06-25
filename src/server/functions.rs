//! Leptos server functions — public API used by pages and components.
// Goal harness deliverable AC2/AC5
// harness-round-7: 2026-06-25T19:10:00Z-functions
//!
//! These functions are auto-registered by the Leptos runtime and are
//! available to both server-rendered and hydrated components.

use crate::auth::guard::{require_admin, require_user};
use crate::auth::session::{session_from_parts, SessionUser};
use crate::config::Config;
use crate::crawler::{self, default_source_registry_url};
use crate::models::{Category, Comment, SiteSettings, Source, Tool};
use uuid::Uuid;
use crate::server::queries::TOOLS_APPROVED_WHERE;
use axum::http::request::Parts;
use leptos::prelude::*;

fn request_context() -> Result<(Parts, sqlx::PgPool, String), ServerFnError> {
    let parts = use_context::<Parts>()
        .ok_or_else(|| ServerFnError::new("request context not available"))?;
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;
    let config = use_context::<Config>()
        .ok_or_else(|| ServerFnError::new("configuration not available"))?;
    Ok((parts, pool, config.jwt_secret))
}

/// Current signed-in user, if any (from session cookie).
#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<Option<SessionUser>, ServerFnError> {
    let (parts, pool, jwt_secret) = request_context()?;
    session_from_parts(&parts, &pool, &jwt_secret)
        .await
        .map_err(ServerFnError::new)
}

/// Admin gate — returns the admin session or a generic "not found" error.
#[server(CheckAdminAccess, "/api")]
pub async fn check_admin_access() -> Result<SessionUser, ServerFnError> {
    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret)
        .await
        .map_err(ServerFnError::new)
}

/// Row shape for category listings with live approved-tool counts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
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
pub(crate) fn validate_update_site_settings_input(
    site_name: &str,
    slogan: &str,
    description: &str,
    mcp_endpoint: &str,
    search_keywords: &[String],
) -> Result<(), &'static str> {
    let name = site_name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err("site name must be 1–100 characters");
    }
    let slogan = slogan.trim();
    if slogan.is_empty() || slogan.len() > 200 {
        return Err("slogan must be 1–200 characters");
    }
    let description = description.trim();
    if description.is_empty() || description.len() > 500 {
        return Err("description must be 1–500 characters");
    }
    let mcp_endpoint = mcp_endpoint.trim();
    if mcp_endpoint.is_empty() || mcp_endpoint.len() > 200 {
        return Err("MCP endpoint must be 1–200 characters");
    }
    if search_keywords.is_empty() || search_keywords.len() > 50 {
        return Err("provide 1–50 search keywords");
    }
    for kw in search_keywords {
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
}

/// Admin-only update of the `site_settings` singleton (id = 1).
#[server(UpdateSiteSettings, "/api")]
pub async fn update_site_settings(
    payload: UpdateSiteSettingsPayload,
) -> Result<SiteSettings, ServerFnError> {
    let keywords = parse_search_keywords(&payload.search_keywords_raw);
    if let Err(msg) = validate_update_site_settings_input(
        &payload.site_name,
        &payload.slogan,
        &payload.description,
        &payload.mcp_endpoint,
        &keywords,
    ) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret)
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

    let sql = format!(
        "SELECT * FROM tools WHERE {TOOLS_APPROVED_WHERE} ORDER BY stars DESC, created_at DESC LIMIT $1"
    );
    let tools = sqlx::query_as::<_, Tool>(&sql)
        .bind(limit)
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load tools: {e}")))?;

    Ok(tools)
}

/// Returns all function categories with live **approved** tool counts.
#[server(GetCategories, "/api")]
pub async fn get_categories() -> Result<Vec<(Category, i64)>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

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
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load categories: {e}")))?;

    Ok(rows.into_iter().map(CategoryWithCount::into_pair).collect())
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

    if function.is_some() {
        sql.push_str(" AND function = $2");
    }
    if chain.is_some() {
        sql.push_str(" AND $3 = ANY(chains)");
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

    Ok(tools)
}

/// Fetch a single **approved** tool by slug (404-style error if missing or not approved).
#[server(GetToolBySlug, "/api")]
pub async fn get_tool_by_slug(slug: String) -> Result<Tool, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let sql = format!("SELECT * FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}");
    let tool = sqlx::query_as::<_, Tool>(&sql)
        .bind(&slug)
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load tool: {e}")))?;

    tool.ok_or_else(|| ServerFnError::new(format!("tool not found: {slug}")))
}

/// Optional axis filters for tool list/count queries (AND across set fields).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolFilters {
    pub function: Option<String>,
    pub asset_class: Option<String>,
    pub actor: Option<String>,
    pub tool_type: Option<String>,
    pub status: Option<String>,
    pub chain: Option<String>,
}

fn append_tool_filters(sql: &mut String, filters: &ToolFilters, idx: &mut i32) {
    if filters.function.is_some() {
        sql.push_str(&format!(" AND function = ${idx}"));
        *idx += 1;
    }
    if filters.asset_class.is_some() {
        sql.push_str(&format!(" AND asset_class = ${idx}"));
        *idx += 1;
    }
    if filters.actor.is_some() {
        sql.push_str(&format!(" AND actor = ${idx}"));
        *idx += 1;
    }
    if filters.tool_type.is_some() {
        sql.push_str(&format!(" AND type = ${idx}"));
        *idx += 1;
    }
    if filters.status.is_some() {
        sql.push_str(&format!(" AND status = ${idx}"));
        *idx += 1;
    }
    if filters.chain.is_some() {
        sql.push_str(&format!(" AND ${idx} = ANY(chains)"));
        *idx += 1;
    }
}

/// Count approved tools with optional multi-axis filters.
#[server(CountTools, "/api")]
pub async fn count_tools(filters: ToolFilters) -> Result<i64, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let mut sql = format!("SELECT COUNT(*) FROM tools WHERE {TOOLS_APPROVED_WHERE}");
    let mut idx = 1i32;
    append_tool_filters(&mut sql, &filters, &mut idx);

    let mut q = sqlx::query_as::<_, (i64,)>(&sql);
    if let Some(f) = &filters.function {
        q = q.bind(f);
    }
    if let Some(v) = &filters.asset_class {
        q = q.bind(v);
    }
    if let Some(v) = &filters.actor {
        q = q.bind(v);
    }
    if let Some(v) = &filters.tool_type {
        q = q.bind(v);
    }
    if let Some(v) = &filters.status {
        q = q.bind(v);
    }
    if let Some(v) = &filters.chain {
        q = q.bind(v);
    }

    let count = q
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("count failed: {e}")))?;

    Ok(count.0)
}

/// Top chains by approved-tool count for sidebar filters.
#[server(GetChainCounts, "/api")]
pub async fn get_chain_counts(limit: i64) -> Result<Vec<(String, i64)>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

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
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("chain counts failed: {e}")))?;

    Ok(rows)
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

    let order = match sort.as_str() {
        "new" => "created_at DESC",
        "comments" => "stars DESC, created_at DESC", // comments milestone wires real sort
        _ => "stars DESC, created_at DESC",
    };

    let has_query = query.as_ref().is_some_and(|q| !q.trim().is_empty());
    let mut sql = format!("SELECT * FROM tools WHERE {TOOLS_APPROVED_WHERE}");
    let mut idx = 1i32;

    if has_query {
        sql.push_str(&format!(
            " AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')) @@ plainto_tsquery('english', ${idx})"
        ));
        idx += 1;
    }
    append_tool_filters(&mut sql, &filters, &mut idx);
    sql.push_str(&format!(" ORDER BY {order} OFFSET ${idx} LIMIT ${}", idx + 1));

    let mut q = sqlx::query_as::<_, Tool>(&sql);
    if let Some(text) = query.as_ref().filter(|q| !q.trim().is_empty()) {
        q = q.bind(text);
    }
    if let Some(f) = &filters.function {
        q = q.bind(f);
    }
    if let Some(v) = &filters.asset_class {
        q = q.bind(v);
    }
    if let Some(v) = &filters.actor {
        q = q.bind(v);
    }
    if let Some(v) = &filters.tool_type {
        q = q.bind(v);
    }
    if let Some(v) = &filters.status {
        q = q.bind(v);
    }
    if let Some(v) = &filters.chain {
        q = q.bind(v);
    }
    q = q.bind(offset).bind(limit);

    let tools = q
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("list tools failed: {e}")))?;

    Ok(tools)
}

/// SQL for admin pending-tool review (AC5).
pub(crate) const LIST_PENDING_TOOLS_SQL: &str =
    "SELECT * FROM tools WHERE approval_status = 'pending' ORDER BY created_at DESC LIMIT $1";

/// List tools awaiting admin review (`approval_status = 'pending'`).
#[server(ListPendingTools, "/api")]
pub async fn list_pending_tools(limit: i64) -> Result<Vec<Tool>, ServerFnError> {
    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret).await?;

    let tools = sqlx::query_as::<_, Tool>(LIST_PENDING_TOOLS_SQL)
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to list pending tools: {e}")))?;

    Ok(tools)
}

/// Validate admin approval inputs without touching the database.
pub(crate) fn validate_set_tool_approval_input(
    status: &str,
    reason: Option<&str>,
) -> Result<(), &'static str> {
    if !matches!(status, "approved" | "rejected" | "pending") {
        return Err("invalid approval status (expected approved|rejected|pending)");
    }
    if status == "rejected" && reason.map(str::trim).is_none_or(str::is_empty) {
        return Err("rejection requires a non-empty reason");
    }
    Ok(())
}

/// Approve or reject a tool by slug (admin growth-mode workflow).
#[server(SetToolApproval, "/api")]
pub async fn set_tool_approval(
    slug: String,
    status: String,
    reason: Option<String>,
) -> Result<(), ServerFnError> {
    if let Err(msg) = validate_set_tool_approval_input(&status, reason.as_deref()) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret).await?;

    let rejection_reason = if status == "rejected" {
        reason
    } else {
        None
    };

    let result = sqlx::query(
        r#"
        UPDATE tools
        SET approval_status = $1,
            rejection_reason = $2,
            updated_at = now()
        WHERE slug = $3
        "#,
    )
    .bind(&status)
    .bind(&rejection_reason)
    .bind(&slug)
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to update approval: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new(format!("tool not found: {slug}")));
    }

    Ok(())
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

/// List crawler source status (admin).
#[server(ListCrawlerSources, "/api")]
pub async fn list_crawler_sources() -> Result<Vec<CrawlerSourceView>, ServerFnError> {
    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret).await?;

    let rows = sqlx::query_as::<_, Source>("SELECT * FROM sources ORDER BY name ASC")
        .fetch_all(&pool)
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

    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret).await?;

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

/// List top-level comments (with replies) for an approved tool.
#[server(GetToolComments, "/api")]
pub async fn get_tool_comments(slug: String) -> Result<Vec<CommentView>, ServerFnError> {
    let (parts, pool, jwt_secret) = request_context()?;
    let viewer = session_from_parts(&parts, &pool, &jwt_secret)
        .await
        .ok()
        .flatten();

    let tool_id = sqlx::query_scalar::<_, Uuid>(
        &format!("SELECT id FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}"),
    )
    .bind(&slug)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to resolve tool: {e}")))?
    .ok_or_else(|| ServerFnError::new(format!("tool not found: {slug}")))?;

    let rows = sqlx::query_as::<_, CommentRow>(
        r#"
        SELECT
            c.id, c.tool_id, c.parent_id, c.user_id, c.content, c.created_at,
            p.nickname AS author_nickname,
            p.is_admin AS author_is_admin,
            COUNT(u.id) AS upvote_count,
            BOOL_OR(u.user_id = $2) AS viewer_upvoted
        FROM comments c
        JOIN profiles p ON p.id = c.user_id
        LEFT JOIN upvotes u ON u.comment_id = c.id
        WHERE c.tool_id = $1
        GROUP BY c.id, p.nickname, p.is_admin
        ORDER BY c.created_at ASC
        "#,
    )
    .bind(tool_id)
    .bind(viewer.as_ref().map(|v| v.id))
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to load comments: {e}")))?;

    Ok(rows.into_iter().map(CommentRow::into_view).collect())
}

#[derive(Debug, sqlx::FromRow)]
struct CommentRow {
    id: Uuid,
    tool_id: Uuid,
    parent_id: Option<Uuid>,
    user_id: Uuid,
    content: String,
    created_at: chrono::DateTime<chrono::Utc>,
    author_nickname: Option<String>,
    author_is_admin: bool,
    upvote_count: Option<i64>,
    viewer_upvoted: Option<bool>,
}

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

    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint
        FROM comments c
        JOIN tools t ON t.id = c.tool_id
        WHERE t.slug = $1 AND t.approval_status = 'approved'
        "#,
    )
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

    let (parts, pool, jwt_secret) = request_context()?;
    let user = require_user(&parts, &pool, &jwt_secret).await?;

    let tool_id = sqlx::query_scalar::<_, Uuid>(
        &format!("SELECT id FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}"),
    )
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
    let (parts, pool, jwt_secret) = request_context()?;
    let user = require_user(&parts, &pool, &jwt_secret).await?;

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
    let (parts, pool, jwt_secret) = request_context()?;
    let Some(user) = session_from_parts(&parts, &pool, &jwt_secret)
        .await
        .map_err(ServerFnError::new)?
    else {
        return Ok(false);
    };

    let bookmarked = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint
        FROM bookmarks b
        JOIN tools t ON t.id = b.tool_id
        WHERE t.slug = $1 AND b.user_id = $2 AND t.approval_status = 'approved'
        "#,
    )
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
    let (parts, pool, jwt_secret) = request_context()?;
    let user = require_user(&parts, &pool, &jwt_secret).await?;

    let tool_id = sqlx::query_scalar::<_, Uuid>(
        &format!("SELECT id FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}"),
    )
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
    if !icon
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
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
    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret).await?;

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
        .map(|(id, label, icon, description, sort_order, tool_count)| AdminCategoryView {
            id,
            label,
            icon,
            description,
            sort_order,
            tool_count,
        })
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

    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret).await?;

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

    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret).await?;

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

    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret).await?;

    let tool_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tools WHERE function = $1",
    )
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
    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret).await?;

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
    let (parts, pool, jwt_secret) = request_context()?;
    let admin = require_admin(&parts, &pool, &jwt_secret).await?;

    if admin.id == user_id {
        return Err(ServerFnError::new("cannot change your own ban status"));
    }

    let result = sqlx::query(
        "UPDATE profiles SET is_banned = $1, updated_at = now() WHERE id = $2",
    )
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
    let (parts, pool, jwt_secret) = request_context()?;
    let admin = require_admin(&parts, &pool, &jwt_secret).await?;

    if admin.id == user_id && !is_admin {
        return Err(ServerFnError::new("cannot remove your own admin role"));
    }

    let result = sqlx::query(
        "UPDATE profiles SET is_admin = $1, updated_at = now() WHERE id = $2",
    )
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
    let (parts, pool, jwt_secret) = request_context()?;
    let admin = require_admin(&parts, &pool, &jwt_secret).await?;

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
    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret).await?;

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

impl AdminCommentRow {
    fn into_view(self) -> AdminCommentView {
        AdminCommentView {
            id: self.id,
            content: self.content,
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
    let (parts, pool, jwt_secret) = request_context()?;
    require_admin(&parts, &pool, &jwt_secret).await?;

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
    let (parts, pool, jwt_secret) = request_context()?;
    let admin = require_admin(&parts, &pool, &jwt_secret).await?;

    let author_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM comments WHERE id = $1",
    )
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

    sqlx::query(
        "UPDATE profiles SET is_banned = true, updated_at = now() WHERE id = $1",
    )
    .bind(author_id)
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to ban user: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_queries_include_approved_filter() {
        let recent = format!(
            "SELECT * FROM tools WHERE {TOOLS_APPROVED_WHERE} ORDER BY stars DESC, created_at DESC LIMIT $1"
        );
        assert!(recent.contains("approval_status = 'approved'"));

        let categories = format!(
            "LEFT JOIN tools t ON t.function = c.id AND t.{TOOLS_APPROVED_WHERE}"
        );
        assert!(categories.contains("approval_status = 'approved'"));
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
        assert!(validate_update_site_settings_input(
            "OnchainAI",
            "Crypto tools, unified.",
            "Discover tools.",
            "npx mcp-remote onchainai.xyz/mcp",
            &["mcp-server".into()],
        )
        .is_ok());
    }

    #[test]
    fn validate_site_settings_rejects_empty_keywords() {
        assert!(validate_update_site_settings_input(
            "OnchainAI",
            "Slogan",
            "Description here.",
            "npx mcp-remote",
            &[],
        )
        .is_err());
    }

    #[test]
    fn validate_site_settings_rejects_invalid_keyword_chars() {
        assert!(validate_update_site_settings_input(
            "OnchainAI",
            "Slogan",
            "Description here.",
            "npx mcp-remote",
            &["bad keyword".into()],
        )
        .is_err());
    }

    #[test]
    fn validate_trigger_crawler_source_accepts_known_sources() {
        assert!(validate_trigger_crawler_source("npm").is_ok());
        assert!(validate_trigger_crawler_source("sync_stars").is_ok());
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
        assert!(validate_category_input(
            "Bad-ID",
            "Label",
            "icon",
            "Description.",
            1
        )
        .is_err());
    }
}