//! Agent Sync REST — token management, device flow, toolkit sync.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::server::agent_sync::{
    device_approve, device_poll, device_start, has_active_link, list_tokens, mint_token,
    revoke_token, sync_blueprint_node, sync_tool, AgentAuth, SyncBlueprintNodeRequest,
    SyncToolRequest,
};
use crate::server::rate_limit::{
    check_agent_token_mint_limit, check_user_rate_limit, UserRateLimitAction,
};
use crate::AppState;

use super::auth::require_user_from;
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v2/agent/tokens",
            post(create_token).get(list_agent_tokens),
        )
        .route("/api/v2/agent/tokens/{id}", delete(revoke_agent_token))
        .route("/api/v2/agent/link-status", get(link_status))
        .route("/api/v2/agent/device/start", post(start_device))
        .route("/api/v2/agent/device/approve", post(approve_device))
        .route("/api/v2/agent/device/poll", post(poll_device))
        .route("/api/v2/agent/sync/tool", post(sync_tool_endpoint))
        .route(
            "/api/v2/agent/sync/blueprint-node",
            post(sync_blueprint_node_endpoint),
        )
        .with_state(state)
}

pub async fn require_agent_from(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AgentAuth, ApiError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    crate::server::agent_sync::resolve_bearer(&state.pool, auth_header)
        .await
        .ok_or_else(|| ApiError::Unauthorized("invalid or expired agent token".into()))
}

#[derive(Debug, Deserialize)]
struct CreateTokenBody {
    label: Option<String>,
    client: Option<String>,
    expires_in_days: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct DeviceStartBody {
    client: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeviceApproveBody {
    user_code: String,
    label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DevicePollBody {
    device_code: String,
}

async fn create_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateTokenBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    if let Err(limit) = check_agent_token_mint_limit(user.id) {
        return Err(ApiError::TooManyRequests(limit.to_string()));
    }

    let minted = mint_token(
        &state.pool,
        user.id,
        body.label,
        body.client.as_deref(),
        body.expires_in_days,
    )
    .await
    .map_err(ApiError::from_server_fn)?;

    Ok(Json(serde_json::json!({
        "id": minted.id,
        "token": minted.token,
        "token_prefix": minted.token_prefix,
        "expires_at": minted.expires_at,
    })))
}

async fn list_agent_tokens(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    let items = list_tokens(&state.pool, user.id)
        .await
        .map_err(ApiError::from_server_fn)?;
    Ok(Json(serde_json::json!({ "items": items })))
}

async fn revoke_agent_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    revoke_token(&state.pool, user.id, id)
        .await
        .map_err(ApiError::from_server_fn)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn link_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    let linked = has_active_link(&state.pool, user.id)
        .await
        .map_err(ApiError::from_server_fn)?;
    Ok(Json(serde_json::json!({ "linked": linked })))
}

async fn start_device(
    State(state): State<AppState>,
    Json(body): Json<DeviceStartBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let started = device_start(&state.pool, body.client.as_deref())
        .await
        .map_err(ApiError::from_server_fn)?;
    Ok(Json(serde_json::to_value(started).unwrap_or_default()))
}

async fn approve_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DeviceApproveBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    if let Err(limit) = check_agent_token_mint_limit(user.id) {
        return Err(ApiError::TooManyRequests(limit.to_string()));
    }

    let approved = device_approve(&state.pool, user.id, &body.user_code, body.label)
        .await
        .map_err(ApiError::from_server_fn)?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "id": approved.id,
        "token_prefix": approved.token_prefix,
        "expires_at": approved.expires_at,
        "message": "Agent linked. Your coding tool will receive the token automatically."
    })))
}

async fn poll_device(
    State(state): State<AppState>,
    Json(body): Json<DevicePollBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let polled = device_poll(&state.pool, &body.device_code)
        .await
        .map_err(ApiError::from_server_fn)?;
    Ok(Json(serde_json::to_value(polled).unwrap_or_default()))
}

async fn sync_tool_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SyncToolRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let auth = require_agent_from(&state, &headers).await?;
    if let Err(limit) = check_user_rate_limit(auth.user_id, UserRateLimitAction::ToggleBookmark) {
        return Err(ApiError::TooManyRequests(limit.to_string()));
    }

    let result = sync_tool(&state.pool, &auth, body)
        .await
        .map_err(ApiError::from_server_fn)?;
    Ok(Json(serde_json::to_value(result).unwrap_or_default()))
}

async fn sync_blueprint_node_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SyncBlueprintNodeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let auth = require_agent_from(&state, &headers).await?;
    if let Err(limit) = check_user_rate_limit(auth.user_id, UserRateLimitAction::AgentBlueprintSync)
    {
        return Err(ApiError::TooManyRequests(limit.to_string()));
    }

    let result = sync_blueprint_node(&state.pool, &auth, body)
        .await
        .map_err(ApiError::from_server_fn)?;
    Ok(Json(serde_json::to_value(result).unwrap_or_default()))
}
