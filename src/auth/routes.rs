//! Axum auth routes — GitHub OAuth via Supabase (PKCE) and logout.

use crate::auth::pkce::generate_pkce_pair;
use crate::auth::session::{
    cookie_value, ensure_profile, ACCESS_TOKEN_COOKIE, PKCE_VERIFIER_COOKIE,
};
use crate::config::Config;
use crate::AppState;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::collections::HashMap;
use supabase_auth::models::{LoginWithOAuthOptions, Provider};
use supabase_auth::models::AuthClient;

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub error: Option<String>,
}

fn auth_client(config: &Config) -> AuthClient {
    AuthClient::new(
        config.supabase_url.clone(),
        config.supabase_anon_key.clone(),
        config.jwt_secret.clone(),
    )
}

fn callback_url(config: &Config) -> String {
    if config.siwx_domain.contains("localhost") {
        format!("http://localhost:{}/auth/callback", config.port)
    } else {
        format!("https://{}/auth/callback", config.siwx_domain)
    }
}

fn set_cookie(name: &str, value: &str, max_age_secs: i64, secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    format!(
        "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age_secs}{secure_flag}"
    )
}

fn clear_cookie(name: &str) -> String {
    format!("{name}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

fn github_nickname(user: &supabase_auth::models::User) -> Option<String> {
    let meta = &user.user_metadata;
    let candidates = [
        meta.name.as_deref(),
        meta.full_name.as_deref(),
        meta.custom.get("user_name").and_then(|v| v.as_str()),
        meta.custom
            .get("preferred_username")
            .and_then(|v| v.as_str()),
        meta.custom.get("login").and_then(|v| v.as_str()),
    ];
    for raw in candidates.into_iter().flatten() {
        let sanitized: String = raw
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .take(20)
            .collect();
        if sanitized.len() >= 2 {
            return Some(sanitized);
        }
    }
    None
}

/// `GET /auth/github` — start GitHub OAuth (PKCE).
pub async fn github_login(State(state): State<AppState>) -> Result<Response, StatusCode> {
    let config = &state.config;
    let client = auth_client(config);

    let (verifier, challenge) = generate_pkce_pair();
    let redirect_to = callback_url(config);

    let options = LoginWithOAuthOptions {
        redirect_to: Some(redirect_to),
        query_params: Some(HashMap::from([
            ("response_type".into(), "code".into()),
            (
                "code_challenge".into(),
                challenge,
            ),
            ("code_challenge_method".into(), "S256".into()),
        ])),
        scopes: None,
        skip_browser_redirect: None,
    };

    let oauth = client
        .login_with_oauth(Provider::Github, Some(options))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        set_cookie(PKCE_VERIFIER_COOKIE, &verifier, 600, false)
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok((headers, Redirect::temporary(oauth.url.as_str())).into_response())
}

/// `GET /auth/callback` — exchange OAuth code, set session cookie, upsert profile.
pub async fn oauth_callback(
    State(state): State<AppState>,
    Query(query): Query<OAuthCallbackQuery>,
    headers_in: HeaderMap,
) -> Result<Response, StatusCode> {
    if query.error.is_some() {
        return Ok(Redirect::to("/?auth=error").into_response());
    }

    let code = query.code.ok_or(StatusCode::BAD_REQUEST)?;
    let config = &state.config;
    let client = auth_client(config);

    let cookie_header = headers_in
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let verifier = cookie_value(cookie_header, PKCE_VERIFIER_COOKIE).ok_or(StatusCode::BAD_REQUEST)?;

    let session = client
        .exchange_code_for_session(&code, verifier)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let nickname = github_nickname(&session.user);
    let avatar = session.user.user_metadata.avatar_url.as_deref();
    ensure_profile(
        &state.pool,
        session.user.id,
        "github",
        nickname.as_deref(),
        avatar,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let secure_cookie = !config.siwx_domain.contains("localhost");
    let max_age = session.expires_in.max(3600);

    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        set_cookie(ACCESS_TOKEN_COOKIE, &session.access_token, max_age, secure_cookie)
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    headers.append(
        header::SET_COOKIE,
        clear_cookie(PKCE_VERIFIER_COOKIE)
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok((headers, Redirect::to("/")).into_response())
}

/// `POST /auth/logout` — clear session cookie.
pub async fn logout() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        clear_cookie(ACCESS_TOKEN_COOKIE)
            .parse()
            .unwrap_or_else(|_| "onchainai_access_token=; Path=/".parse().unwrap()),
    );
    Redirect::to("/").into_response()
}