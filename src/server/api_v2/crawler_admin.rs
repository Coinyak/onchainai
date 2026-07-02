//! Admin crawler endpoints.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::crawler;
use crate::models::Source;
use crate::server::functions::{
    list_crawler_sources_inner, update_crawler_source_inner, validate_trigger_crawler_source,
    validate_update_crawler_source, CrawlerSourceView, UpdateCrawlerSourcePayload,
};
use crate::AppState;

use super::auth::require_admin_from;
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/admin/crawler/sources", get(list_crawler_sources))
        .route(
            "/api/v2/admin/crawler/sources/{id}",
            put(update_crawler_source),
        )
        .route(
            "/api/v2/admin/crawler/trigger",
            post(trigger_crawler_source),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct TriggerBody {
    source: String,
}

async fn list_crawler_sources(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<CrawlerSourceView>>, ApiError> {
    require_admin_from(&state, &headers).await?;
    list_crawler_sources_inner(&state.pool)
        .await
        .map_err(ApiError::from_server_fn)
        .map(Json)
}

async fn update_crawler_source(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCrawlerSourcePayload>,
) -> Result<Json<Source>, ApiError> {
    if let Err(msg) = validate_update_crawler_source(payload.schedule_minutes) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    require_admin_from(&state, &headers).await?;

    update_crawler_source_inner(&state.pool, id, payload.schedule_minutes, payload.enabled)
        .await
        .map_err(ApiError::from_server_fn)
        .map(Json)
}

async fn trigger_crawler_source(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TriggerBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Err(msg) = validate_trigger_crawler_source(&body.source) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    require_admin_from(&state, &headers).await?;

    let pool_bg = state.pool.clone();
    let source_bg = body.source.clone();
    tokio::spawn(async move {
        crawler::trigger_source(&pool_bg, &source_bg).await;
    });

    Ok(Json(serde_json::json!({ "ok": true })))
}
