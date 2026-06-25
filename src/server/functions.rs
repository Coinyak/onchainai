//! Leptos server functions — public API used by pages and components.
//!
//! These functions are auto-registered by the Leptos runtime and are
//! available to both server-rendered and hydrated components.

use crate::models::{Category, Tool};
use crate::server::queries::TOOLS_APPROVED_WHERE;
use leptos::prelude::*;

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

/// List tools awaiting admin review (`approval_status = 'pending'`).
#[server(ListPendingTools, "/api")]
pub async fn list_pending_tools(limit: i64) -> Result<Vec<Tool>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let tools = sqlx::query_as::<_, Tool>(
        "SELECT * FROM tools WHERE approval_status = 'pending' ORDER BY created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to list pending tools: {e}")))?;

    Ok(tools)
}

/// Approve or reject a tool by slug (admin growth-mode workflow).
#[server(SetToolApproval, "/api")]
pub async fn set_tool_approval(
    slug: String,
    status: String,
    reason: Option<String>,
) -> Result<(), ServerFnError> {
    if !matches!(status.as_str(), "approved" | "rejected" | "pending") {
        return Err(ServerFnError::new(format!(
            "invalid approval status: {status} (expected approved|rejected|pending)"
        )));
    }

    if status == "rejected" && reason.as_ref().is_none_or(|r| r.trim().is_empty()) {
        return Err(ServerFnError::new(
            "rejection requires a non-empty reason".to_string(),
        ));
    }

    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

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
}