//! Public catalog and toolkit endpoints.

use crate::filter_query::build_tool_filters;
use crate::models::tool::sanitize_tools_for_public_response;
use crate::models::{Category, Tool};
use crate::server::functions::{
    browser_visible_limit_for_page, build_toolkit_payload, clamp_browser_page_param,
    clamp_dashboard_list_limit, fetch_categories, fetch_chain_counts, fetch_count_tools,
    fetch_filtered_category_counts,
    fetch_list_tools, fetch_public_dashboard_snapshot, fetch_tool_by_slug,
    fetch_tool_comment_counts, resolve_bookmark_tool_id, validate_search_tools_input,
    validate_tool_filters, validate_tool_list_request, BrowserDataPayload, LoadBrowserDataRequest,
    MyToolkitPayload, PublicDashboardSnapshot, ToolComparisonView, ToolListRequest,
    ToolkitToolView, UpdateToolkitItemPayload,
};
use crate::server::queries::{
    APPROVED_TOOLS_BY_SLUGS_SQL, BOOKMARKED_SLUGS_SQL, RECENT_APPROVED_TOOLS_SQL,
    SEARCH_APPROVED_TOOLS_SQL, TOOL_COMMENT_COUNT_BY_SLUG_SQL, USER_TOOLKIT_SQL,
};
use crate::server::review_persistence::list_public_official_links;
use crate::trust_verification::verify_tool_trust;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;

use super::auth::{optional_user_from, require_user_from};
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/tools/recent", get(get_recent_tools))
        .route("/api/v2/categories", get(get_categories))
        .route("/api/v2/tools/search", get(search_tools))
        .route("/api/v2/tools/count", get(count_tools))
        .route("/api/v2/chains", get(get_chain_counts))
        .route("/api/v2/tools/compare", get(compare_tools))
        .route("/api/v2/tools/comment-counts", get(get_tool_comment_counts))
        .route("/api/v2/tools/list", post(list_tools))
        .route("/api/v2/browser-data", post(load_browser_data))
        .route("/api/v2/dashboard", get(get_dashboard))
        .route("/api/v2/toolkit", get(list_my_toolkit))
        .route("/api/v2/toolkit/{slug}", put(update_toolkit_item))
        .route(
            "/api/v2/tools/{slug}/comment-count",
            get(get_tool_comment_count),
        )
        .route("/api/v2/tools/{slug}", get(get_tool_by_slug))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct LimitQuery {
    #[serde(default = "default_recent_limit")]
    limit: i64,
}

fn default_recent_limit() -> i64 {
    20
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    query: String,
    function: Option<String>,
    chain: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CountToolsQuery {
    function: Option<String>,
    asset_class: Option<String>,
    actor: Option<String>,
    #[serde(rename = "type")]
    tool_type: Option<String>,
    status: Option<String>,
    pricing: Option<String>,
    install_risk: Option<String>,
    chain: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChainLimitQuery {
    #[serde(default = "default_chain_limit")]
    limit: i64,
}

fn default_chain_limit() -> i64 {
    12
}

#[derive(Debug, Deserialize)]
struct SlugsQuery {
    slugs: String,
}

#[derive(Debug, Deserialize)]
struct DashboardQuery {
    #[serde(default = "default_dashboard_limit")]
    limit: i64,
}

fn default_dashboard_limit() -> i64 {
    12
}

async fn get_recent_tools(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Vec<Tool>>, ApiError> {
    let limit = q.limit.clamp(1, 100);
    let tools = sqlx::query_as::<_, Tool>(RECENT_APPROVED_TOOLS_SQL)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to load tools: {e}")))?;
    Ok(Json(sanitize_tools_for_public_response(tools)))
}

async fn get_categories(
    State(state): State<AppState>,
) -> Result<Json<Vec<(Category, i64)>>, ApiError> {
    fetch_categories(&state.pool)
        .await
        .map_err(ApiError::from_server_fn)
        .map(Json)
}

async fn search_tools(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Vec<Tool>>, ApiError> {
    validate_search_tools_input(&q.query, &q.function, &q.chain)
        .map_err(ApiError::from_server_fn)?;

    let tools = sqlx::query_as::<_, Tool>(SEARCH_APPROVED_TOOLS_SQL)
        .bind(&q.query)
        .bind(q.function.as_deref())
        .bind(q.chain.as_deref())
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("search failed: {e}")))?;

    Ok(Json(sanitize_tools_for_public_response(tools)))
}

async fn get_tool_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Tool>, ApiError> {
    fetch_tool_by_slug(&state.pool, &slug)
        .await
        .map_err(ApiError::from_server_fn)?
        .ok_or_else(|| ApiError::NotFound(format!("tool not found: {slug}")))
        .map(Json)
}

async fn count_tools(
    State(state): State<AppState>,
    Query(q): Query<CountToolsQuery>,
) -> Result<Json<i64>, ApiError> {
    let filters = build_tool_filters(
        q.function,
        q.asset_class,
        q.actor,
        q.tool_type,
        q.status,
        q.pricing,
        q.install_risk,
        q.chain,
    );
    validate_tool_filters(&filters).map_err(ApiError::from_server_fn)?;
    fetch_count_tools(&state.pool, &filters)
        .await
        .map_err(ApiError::from_server_fn)
        .map(Json)
}

async fn get_chain_counts(
    State(state): State<AppState>,
    Query(q): Query<ChainLimitQuery>,
) -> Result<Json<Vec<(String, i64)>>, ApiError> {
    fetch_chain_counts(&state.pool, q.limit)
        .await
        .map_err(ApiError::from_server_fn)
        .map(Json)
}

async fn list_tools(
    State(state): State<AppState>,
    Json(req): Json<ToolListRequest>,
) -> Result<Json<Vec<Tool>>, ApiError> {
    validate_tool_list_request(&req).map_err(ApiError::from_server_fn)?;
    fetch_list_tools(
        &state.pool,
        &req.sort,
        req.offset,
        req.limit,
        &req.filters,
        req.query.as_deref(),
    )
    .await
    .map_err(ApiError::from_server_fn)
    .map(Json)
}

async fn load_browser_data(
    State(state): State<AppState>,
    Json(req): Json<LoadBrowserDataRequest>,
) -> Result<Json<BrowserDataPayload>, ApiError> {
    validate_tool_filters(&req.filters).map_err(ApiError::from_server_fn)?;

    let page = clamp_browser_page_param(req.page);
    let list_req = ToolListRequest {
        sort: req.sort.clone(),
        offset: 0,
        limit: browser_visible_limit_for_page(page),
        filters: req.filters.clone(),
        query: req.search_q.clone(),
    };
    validate_tool_list_request(&list_req).map_err(ApiError::from_server_fn)?;

    let preview_slug = req.selected.filter(|s| !s.is_empty());

    let (categories, chains, total, tools, preview_tool) = futures::join!(
        fetch_filtered_category_counts(&state.pool, &req.filters),
        fetch_chain_counts(&state.pool, 100),
        fetch_count_tools(&state.pool, &req.filters),
        fetch_list_tools(
            &state.pool,
            &list_req.sort,
            list_req.offset,
            list_req.limit,
            &list_req.filters,
            list_req.query.as_deref(),
        ),
        async {
            match preview_slug.as_deref() {
                Some(s) => fetch_tool_by_slug(&state.pool, s).await.ok().flatten(),
                None => None,
            }
        },
    );
    let categories = categories.map_err(ApiError::from_server_fn)?;
    let chains = chains.map_err(ApiError::from_server_fn)?;
    let total = total.map_err(ApiError::from_server_fn)?;
    let tools = tools.map_err(ApiError::from_server_fn)?;

    let slugs: Vec<String> = tools.iter().map(|t| t.slug.clone()).collect();
    let comment_counts = if slugs.is_empty() {
        std::collections::HashMap::new()
    } else {
        fetch_tool_comment_counts(&state.pool, &slugs)
            .await
            .map_err(ApiError::from_server_fn)?
            .into_iter()
            .collect()
    };

    Ok(Json(BrowserDataPayload {
        categories,
        chains,
        total,
        tools,
        comment_counts,
        preview_tool,
    }))
}

async fn get_dashboard(
    State(state): State<AppState>,
    Query(q): Query<DashboardQuery>,
) -> Result<Json<PublicDashboardSnapshot>, ApiError> {
    fetch_public_dashboard_snapshot(&state.pool, clamp_dashboard_list_limit(q.limit))
        .await
        .map_err(ApiError::from_server_fn)
        .map(Json)
}

async fn list_my_toolkit(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MyToolkitPayload>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    fetch_user_toolkit(&state.pool, user.id)
        .await
        .map_err(ApiError::from_server_fn)
        .map(Json)
}

async fn update_toolkit_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(mut input): Json<UpdateToolkitItemPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    input.slug = slug;
    let user = require_user_from(&state, &headers).await?;

    let note = validate_toolkit_note(input.note.clone()).map_err(ApiError::from_server_fn)?;
    let tags = validate_toolkit_tags(&input.tags).map_err(ApiError::from_server_fn)?;
    let tool_id = resolve_bookmark_tool_id(&state.pool, &input.slug)
        .await
        .map_err(ApiError::from_server_fn)?;

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
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to update toolkit item: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::BadRequest(
            "save the tool before editing toolkit metadata".into(),
        ));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn compare_tools(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<SlugsQuery>,
) -> Result<Json<Vec<ToolComparisonView>>, ApiError> {
    let viewer = optional_user_from(&state, &headers).await?;
    let normalized = crate::discovery::normalize_compare_slugs(&q.slugs);
    if normalized.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let tools = sqlx::query_as::<_, Tool>(APPROVED_TOOLS_BY_SLUGS_SQL)
        .bind(&normalized)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to load tools: {e}")))?;
    let tools = sanitize_tools_for_public_response(tools);

    let bookmarked_slugs: std::collections::HashSet<String> = if let Some(user) = viewer.as_ref() {
        sqlx::query_scalar::<_, String>(BOOKMARKED_SLUGS_SQL)
            .bind(&normalized)
            .bind(user.id)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(format!("bookmark lookup failed: {e}")))?
            .into_iter()
            .collect()
    } else {
        std::collections::HashSet::new()
    };

    let tool_map: std::collections::HashMap<String, Tool> =
        tools.into_iter().map(|t| (t.slug.clone(), t)).collect();

    let mut rows = Vec::new();
    for slug in &normalized {
        let Some(tool) = tool_map.get(slug) else {
            continue;
        };
        let official_links = list_public_official_links(&state.pool, tool.id)
            .await
            .map_err(ApiError::from_server_fn)?;
        let trust = verify_tool_trust(tool, &official_links);
        let viewer_bookmarked = bookmarked_slugs.contains(&tool.slug);
        rows.push(ToolComparisonView {
            tool: tool.clone(),
            official_links,
            trust_facts: trust.trust_facts,
            viewer_bookmarked,
        });
    }

    Ok(Json(rows))
}

async fn get_tool_comment_counts(
    State(state): State<AppState>,
    Query(q): Query<SlugsQuery>,
) -> Result<Json<Vec<(String, i64)>>, ApiError> {
    let slugs: Vec<String> = q
        .slugs
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    fetch_tool_comment_counts(&state.pool, &slugs)
        .await
        .map_err(ApiError::from_server_fn)
        .map(Json)
}

async fn get_tool_comment_count(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<i64>, ApiError> {
    let count = sqlx::query_scalar::<_, i64>(TOOL_COMMENT_COUNT_BY_SLUG_SQL)
        .bind(slug)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("comment count failed: {e}")))?;
    Ok(Json(count))
}

async fn fetch_user_toolkit(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<MyToolkitPayload, crate::server::fn_error::FnError> {
    use sqlx::{FromRow, Row};

    let rows = sqlx::query(USER_TOOLKIT_SQL)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            crate::server::fn_error::FnError::new(format!("failed to load toolkit: {e}"))
        })?;

    let mut items = Vec::new();
    for row in rows {
        let tool = Tool::from_row(&row).map_err(|e| {
            crate::server::fn_error::FnError::new(format!("failed to decode toolkit tool: {e}"))
        })?;
        let note = row
            .try_get::<Option<String>, _>("bookmark_note")
            .map_err(|e| {
                crate::server::fn_error::FnError::new(format!("failed to decode toolkit note: {e}"))
            })?;
        let tags = row
            .try_get::<Vec<String>, _>("bookmark_tags")
            .map_err(|e| {
                crate::server::fn_error::FnError::new(format!("failed to decode toolkit tags: {e}"))
            })?;
        let saved_at = row
            .try_get::<chrono::DateTime<chrono::Utc>, _>("bookmark_created_at")
            .map_err(|e| {
                crate::server::fn_error::FnError::new(format!(
                    "failed to decode toolkit saved_at: {e}"
                ))
            })?;
        let updated_at = row
            .try_get::<chrono::DateTime<chrono::Utc>, _>("bookmark_updated_at")
            .map_err(|e| {
                crate::server::fn_error::FnError::new(format!(
                    "failed to decode toolkit updated_at: {e}"
                ))
            })?;
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

fn validate_toolkit_tags(tags: &[String]) -> Result<Vec<String>, crate::server::fn_error::FnError> {
    if tags.len() > 8 {
        return Err(crate::server::fn_error::FnError::new(
            "toolkit tags accept at most 8 values",
        ));
    }
    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.trim().trim_start_matches('#').to_ascii_lowercase();
        if tag.is_empty() {
            continue;
        }
        if tag.len() > 32 {
            return Err(crate::server::fn_error::FnError::new(
                "toolkit tags must be at most 32 characters",
            ));
        }
        if tag
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))
        {
            return Err(crate::server::fn_error::FnError::new(
                "toolkit tags may contain letters, numbers, hyphens, and underscores",
            ));
        }
        if seen.insert(tag.clone()) {
            normalized.push(tag);
        }
    }
    Ok(normalized)
}

fn validate_toolkit_note(
    note: Option<String>,
) -> Result<Option<String>, crate::server::fn_error::FnError> {
    let note = note.map(|value| value.trim().to_string());
    let note = note.filter(|value| !value.is_empty());
    if note.as_ref().is_some_and(|value| value.len() > 500) {
        return Err(crate::server::fn_error::FnError::new(
            "toolkit note must be at most 500 characters",
        ));
    }
    Ok(note)
}
