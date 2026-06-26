//! Axum handlers for first-login profile onboarding.

use crate::auth::session::{
    complete_onboarding, cookie_value, user_id_from_jwt, ACCESS_TOKEN_COOKIE,
};
use crate::AppState;
use axum::{
    extract::{Form, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OnboardingForm {
    pub nickname: Option<String>,
    pub bio: Option<String>,
    pub next: Option<String>,
}

fn safe_next(next: Option<String>) -> String {
    next.filter(|s| s.starts_with('/') && !s.starts_with("//"))
        .unwrap_or_else(|| "/".into())
}

fn user_from_cookie(
    headers: &HeaderMap,
    jwt_secret: &str,
    issuer: &str,
) -> Result<uuid::Uuid, StatusCode> {
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let token = cookie_value(cookie_header, ACCESS_TOKEN_COOKIE).ok_or(StatusCode::UNAUTHORIZED)?;
    user_id_from_jwt(token, jwt_secret, issuer).map_err(|_| StatusCode::UNAUTHORIZED)
}

/// `POST /onboarding/complete` — save nickname/bio and mark onboarding done.
pub async fn complete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<OnboardingForm>,
) -> Result<Response, StatusCode> {
    let user_id = user_from_cookie(
        &headers,
        &state.config.jwt_secret,
        &state.config.jwt_issuer(),
    )?;
    complete_onboarding(
        &state.pool,
        user_id,
        form.nickname.as_deref(),
        form.bio.as_deref(),
        false,
    )
    .await
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Redirect::to(&safe_next(form.next)).into_response())
}

/// `POST /onboarding/skip` — auto nickname and mark onboarding done.
pub async fn skip(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<OnboardingForm>,
) -> Result<Response, StatusCode> {
    let user_id = user_from_cookie(
        &headers,
        &state.config.jwt_secret,
        &state.config.jwt_issuer(),
    )?;
    complete_onboarding(&state.pool, user_id, form.nickname.as_deref(), None, true)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Redirect::to(&safe_next(form.next)).into_response())
}
