//! Current user and admin gate endpoints.

use axum::{
    extract::State,
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;

use crate::auth::session::{
    cookie_secure_for_domain, session_hint_present, set_session_hint_cookie,
};
use crate::AppState;

use super::auth::{optional_user_from, require_admin_from};
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/me", get(get_me))
        .route("/api/v2/admin/check", get(check_admin))
        .with_state(state)
}

async fn get_me(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ApiError> {
    let user = optional_user_from(&state, &headers).await?;
    let has_user = user.is_some();
    let mut response = Json(user).into_response();

    if has_user {
        append_session_hint_if_missing(&headers, &state, response.headers_mut());
    }

    Ok(response)
}

async fn check_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = require_admin_from(&state, &headers).await?;
    Ok(Json(json!({ "ok": true, "user": user })))
}

fn append_session_hint_if_missing(
    headers: &HeaderMap,
    state: &AppState,
    response_headers: &mut HeaderMap,
) {
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if session_hint_present(cookie_header) {
        return;
    }

    let secure = cookie_secure_for_domain(&state.config.siwx_domain);
    let hint = set_session_hint_cookie(state.config.siwx_session_ttl, secure);
    let Ok(value) = hint.parse::<axum::http::HeaderValue>() else {
        tracing::warn!("session hint cookie rejected by header parser");
        return;
    };
    response_headers.insert(header::SET_COOKIE, value);
}
