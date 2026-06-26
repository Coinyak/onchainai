//! Axum auth routes — direct GitHub OAuth, email magic links, and logout.

use crate::auth::session::{
    cookie_value, ensure_github_profile, issue_access_token, post_auth_redirect_path,
    ACCESS_TOKEN_COOKIE, GITHUB_STATE_COOKIE,
};
use crate::config::Config;
use crate::AppState;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use getrandom::getrandom;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub error: Option<String>,
    pub state: Option<String>,
    pub token_hash: Option<String>,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub otp_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GithubUser {
    id: i64,
    login: String,
    avatar_url: Option<String>,
}

fn callback_url(config: &Config) -> String {
    if config.siwx_domain.contains("localhost") {
        format!("http://localhost:{}/auth/callback", config.port)
    } else {
        format!("https://{}/auth/callback", config.siwx_domain)
    }
}

fn set_cookie(name: &str, value: &str, max_age_secs: i64, secure: bool, same_site: &str) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    format!(
        "{name}={value}; Path=/; HttpOnly; SameSite={same_site}; Max-Age={max_age_secs}{secure_flag}"
    )
}

fn clear_cookie(name: &str, secure: bool, same_site: &str) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    format!("{name}=; Path=/; HttpOnly; SameSite={same_site}; Max-Age=0{secure_flag}")
}

fn generate_oauth_state() -> String {
    let mut bytes = [0u8; 32];
    getrandom(&mut bytes).expect("OS random unavailable");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// `GET /auth/github` — redirect to GitHub OAuth (callback stays on this app).
pub async fn github_login(State(state): State<AppState>) -> Result<Response, StatusCode> {
    let config = &state.config;
    let oauth_state = generate_oauth_state();
    let redirect_uri = callback_url(config);
    let authorize_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}",
        urlencoding::encode(&config.github_client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("read:user"),
        urlencoding::encode(&oauth_state),
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        // OAuth state cookie must survive the cross-site redirect back from
        // GitHub, so it stays SameSite=Lax (Strict would drop it on the
        // top-level navigation and break the callback).
        set_cookie(GITHUB_STATE_COOKIE, &oauth_state, 600, false, "Lax")
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok((headers, Redirect::temporary(&authorize_url)).into_response())
}

async fn exchange_github_code(
    config: &Config,
    code: &str,
) -> Result<GithubTokenResponse, StatusCode> {
    let client = reqwest::Client::builder()
        .user_agent("OnchainAI/0.1")
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let redirect_uri = callback_url(config);
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header(header::ACCEPT, "application/json")
        .json(&serde_json::json!({
            "client_id": config.github_client_id,
            "client_secret": config.github_client_secret,
            "code": code,
            "redirect_uri": redirect_uri,
        }))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !response.status().is_success() {
        return Err(StatusCode::BAD_REQUEST);
    }

    response
        .json::<GithubTokenResponse>()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)
}

async fn fetch_github_user(access_token: &str) -> Result<GithubUser, StatusCode> {
    let client = reqwest::Client::builder()
        .user_agent("OnchainAI/0.1")
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = client
        .get("https://api.github.com/user")
        .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
        .header(header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !response.status().is_success() {
        return Err(StatusCode::BAD_REQUEST);
    }

    response
        .json::<GithubUser>()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)
}

/// `GET /auth/callback` — GitHub code exchange or email magic-link completion.
pub async fn oauth_callback(
    State(state): State<AppState>,
    Query(query): Query<OAuthCallbackQuery>,
    headers_in: HeaderMap,
) -> Result<Response, StatusCode> {
    if query.error.is_some() {
        return Ok(Redirect::to("/?auth=error").into_response());
    }

    if let Some(token_hash) = query.token_hash.filter(|t| !t.is_empty()) {
        return crate::auth::email::complete_magic_link(&state.pool, &state.config, &token_hash)
            .await;
    }

    let code = query.code.ok_or(StatusCode::BAD_REQUEST)?;
    let state_param = query.state.ok_or(StatusCode::BAD_REQUEST)?;
    let config = &state.config;

    let cookie_header = headers_in
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let cookie_state =
        cookie_value(cookie_header, GITHUB_STATE_COOKIE).ok_or(StatusCode::BAD_REQUEST)?;
    if cookie_state != state_param {
        return Err(StatusCode::BAD_REQUEST);
    }

    let token = exchange_github_code(config, &code).await?;
    let github_user = fetch_github_user(&token.access_token).await?;
    let user_id = ensure_github_profile(
        &state.pool,
        &state.config,
        github_user.id,
        &github_user.login,
        github_user.avatar_url.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let access_token = issue_access_token(
        user_id,
        &config.jwt_secret,
        config.siwx_session_ttl,
        &config.jwt_issuer(),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let secure_cookie = !config.siwx_domain.contains("localhost");

    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        // Session cookie is SameSite=Strict for CSRF hardening (SECURITY.md).
        set_cookie(
            ACCESS_TOKEN_COOKIE,
            &access_token,
            config.siwx_session_ttl,
            secure_cookie,
            "Strict",
        )
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    headers.append(
        header::SET_COOKIE,
        clear_cookie(GITHUB_STATE_COOKIE, false, "Lax")
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    let redirect_to = post_auth_redirect_path(&state.pool, user_id).await;

    Ok((headers, Redirect::to(&redirect_to)).into_response())
}

/// `POST /auth/logout` — clear session cookie.
pub async fn logout(State(state): State<AppState>) -> Response {
    let secure_cookie = !state.config.siwx_domain.contains("localhost");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        clear_cookie(ACCESS_TOKEN_COOKIE, secure_cookie, "Strict")
            .parse()
            .unwrap_or_else(|_| "onchainai_access_token=; Path=/".parse().unwrap()),
    );
    (headers, Redirect::to("/")).into_response()
}
