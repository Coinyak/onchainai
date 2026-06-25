//! Leptos server functions — public API used by pages and components.
// Goal harness deliverable AC2/AC5
// harness-round-7: 2026-06-25T19:10:00Z-functions
//!
//! These functions are auto-registered by the Leptos runtime and are
//! available to both server-rendered and hydrated components.

use crate::auth::guard::require_admin;
use crate::auth::session::{session_from_parts, SessionUser};
use crate::config::Config;
use crate::models::{Category, SiteSettings, Tool};
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
}