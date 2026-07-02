//! Cookie session helpers for Axum `/api/v2/*` handlers.

use axum::http::{request::Parts, HeaderMap};

use crate::auth::guard::{require_admin, require_user};
use crate::auth::session::{optional_session_result, session_from_parts, SessionUser};
use crate::AppState;

use super::error::ApiError;

/// Build request `Parts` from incoming headers (cookie auth reads `parts.headers`).
pub fn parts_from_headers(headers: &HeaderMap) -> Parts {
    let mut request = axum::http::Request::builder()
        .body(())
        .expect("infallible empty body request");
    *request.headers_mut() = headers.clone();
    request.into_parts().0
}

pub async fn require_admin_from(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<SessionUser, ApiError> {
    let parts = parts_from_headers(headers);
    require_admin(&parts, &state.pool, &state.config)
        .await
        .map_err(|_| ApiError::Forbidden("not found".into()))
}

pub async fn require_user_from(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<SessionUser, ApiError> {
    let parts = parts_from_headers(headers);
    require_user(
        &parts,
        &state.pool,
        &state.config.jwt_secret,
        &state.config.jwt_issuer(),
    )
    .await
    .map_err(|_| ApiError::Unauthorized("sign in required".into()))
}

pub async fn optional_user_from(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<SessionUser>, ApiError> {
    let parts = parts_from_headers(headers);
    let result = session_from_parts(
        &parts,
        &state.pool,
        &state.config.jwt_secret,
        &state.config.jwt_issuer(),
    )
    .await;
    optional_session_result(result).map_err(|e| ApiError::Internal(e.to_string()))
}
