//! Axum auth routes — direct GitHub OAuth, email magic links, and logout.

use crate::auth::oauth_state::{mint_oauth_state, verify_oauth_state};
use crate::auth::session::{
    auth_http_client, clear_session_hint_cookie, cookie_secure_for_domain, cookie_value,
    ensure_github_profile, issue_access_token, local_dev_host, post_auth_redirect_path,
    set_session_hint_cookie, ACCESS_TOKEN_COOKIE, GITHUB_STATE_COOKIE,
};
use crate::config::Config;
use crate::AppState;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
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
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug)]
enum GithubExchangeError {
    RedirectMismatch,
    Other,
}

#[derive(Debug, Deserialize)]
struct GithubUser {
    id: i64,
    login: String,
    avatar_url: Option<String>,
}

fn callback_url(config: &Config) -> String {
    if let Some(uri) = config
        .github_redirect_uri
        .as_deref()
        .map(str::trim)
        .filter(|uri| !uri.is_empty())
    {
        return uri.to_string();
    }
    if let Some(host) = local_dev_host(&config.siwx_domain) {
        format!("http://{host}:{}/auth/callback", config.port)
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

fn github_oauth_state_valid(config: &Config, headers: &HeaderMap, state_param: &str) -> bool {
    let cookie_header = headers.get(header::COOKIE).and_then(|v| v.to_str().ok());
    let cookie_state = cookie_header.and_then(|h| cookie_value(h, GITHUB_STATE_COOKIE));
    if cookie_state == Some(state_param) {
        return true;
    }
    verify_oauth_state(&config.jwt_secret, state_param)
}

/// `GET /auth/github` — redirect to GitHub OAuth (callback stays on this app).
pub async fn github_login(State(state): State<AppState>) -> Result<Response, StatusCode> {
    let config = &state.config;
    let oauth_state = mint_oauth_state(&config.jwt_secret);
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
        set_cookie(
            GITHUB_STATE_COOKIE,
            &oauth_state,
            600,
            cookie_secure_for_domain(&config.siwx_domain),
            "Lax",
        )
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok((headers, Redirect::temporary(&authorize_url)).into_response())
}

async fn exchange_github_code(
    config: &Config,
    code: &str,
) -> Result<GithubTokenResponse, GithubExchangeError> {
    let client = auth_http_client().map_err(|_| GithubExchangeError::Other)?;

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
        .map_err(|_| GithubExchangeError::Other)?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let token: GithubTokenResponse =
        serde_json::from_str(&body).map_err(|_| GithubExchangeError::Other)?;

    if let Some(err) = token.error.as_deref() {
        tracing::warn!(
            status = %status,
            github_error = err,
            description = token.error_description.as_deref().unwrap_or(""),
            redirect_uri = %redirect_uri,
            "GitHub OAuth token exchange rejected"
        );
        return Err(if err == "redirect_uri_mismatch" {
            GithubExchangeError::RedirectMismatch
        } else {
            GithubExchangeError::Other
        });
    }

    if !status.is_success() || token.access_token.as_deref().unwrap_or("").is_empty() {
        tracing::warn!(
            status = %status,
            body_len = body.len(),
            "GitHub OAuth token exchange failed"
        );
        return Err(GithubExchangeError::Other);
    }

    Ok(token)
}

async fn fetch_github_user(access_token: &str) -> Result<GithubUser, StatusCode> {
    let client = auth_http_client().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
        return Ok(Redirect::to("/login?auth=github_denied").into_response());
    }

    if let Some(token_hash) = query.token_hash.filter(|t| !t.is_empty()) {
        return crate::auth::email::complete_magic_link(&state.pool, &state.config, &token_hash)
            .await;
    }

    let Some(code) = query.code.filter(|c| !c.is_empty()) else {
        return Ok(Redirect::to("/login?auth=github_missing_code").into_response());
    };
    let Some(state_param) = query.state.filter(|s| !s.is_empty()) else {
        return Ok(Redirect::to("/login?auth=github_missing_state").into_response());
    };
    let config = &state.config;

    if !github_oauth_state_valid(config, &headers_in, &state_param) {
        tracing::warn!("GitHub OAuth state validation failed (cookie and HMAC mismatch)");
        return Ok(Redirect::to("/login?auth=github_state_mismatch").into_response());
    }

    let token = match exchange_github_code(config, &code).await {
        Ok(token) => token,
        Err(GithubExchangeError::RedirectMismatch) => {
            return Ok(Redirect::to("/login?auth=github_redirect_mismatch").into_response());
        }
        Err(GithubExchangeError::Other) => {
            return Ok(Redirect::to("/login?auth=github_token_exchange").into_response());
        }
    };
    let access_token = token
        .access_token
        .as_deref()
        .ok_or(StatusCode::BAD_REQUEST)?;
    let github_user = match fetch_github_user(access_token).await {
        Ok(user) => user,
        Err(_) => {
            return Ok(Redirect::to("/login?auth=github_user_fetch").into_response());
        }
    };
    let user_id = match ensure_github_profile(
        &state.pool,
        &state.config,
        github_user.id,
        &github_user.login,
        github_user.avatar_url.as_deref(),
    )
    .await
    {
        Ok(id) => id,
        Err(err) => {
            tracing::error!(error = %err, github_id = github_user.id, "GitHub profile setup failed");
            let code = err.auth_query_code();
            return Ok(Redirect::to(&format!("/login?auth={code}")).into_response());
        }
    };

    let access_token = issue_access_token(
        user_id,
        &config.jwt_secret,
        config.siwx_session_ttl,
        &config.jwt_issuer(),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let secure_cookie = cookie_secure_for_domain(&config.siwx_domain);

    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        // Session cookie is SameSite=Lax: it must be sent on the top-level
        // navigation back from the GitHub/Supabase redirect (Strict withholds
        // it on that cross-site-initiated landing, so the user appears signed
        // out until the next same-site request). Lax blocks the cookie on
        // cross-site POST/subresource requests, which is the primary CSRF
        // defense for v1. No CSRF token or Origin-check middleware is
        // implemented yet; see docs/SECURITY.md §3.4 for the current posture.
        set_cookie(
            ACCESS_TOKEN_COOKIE,
            &access_token,
            config.siwx_session_ttl,
            secure_cookie,
            "Lax",
        )
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    headers.append(
        header::SET_COOKIE,
        set_session_hint_cookie(config.siwx_session_ttl, secure_cookie)
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    headers.append(
        header::SET_COOKIE,
        clear_cookie(GITHUB_STATE_COOKIE, secure_cookie, "Lax")
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    let redirect_to = post_auth_redirect_path(&state.pool, user_id).await;

    Ok((headers, Redirect::to(&redirect_to)).into_response())
}

fn append_set_cookie(headers: &mut HeaderMap, cookie: &str) -> Result<(), StatusCode> {
    headers.append(
        header::SET_COOKIE,
        cookie
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(())
}

fn clear_session_cookies(headers: &mut HeaderMap, secure_cookie: bool) -> Result<(), StatusCode> {
    append_set_cookie(
        headers,
        &clear_cookie(ACCESS_TOKEN_COOKIE, secure_cookie, "Lax"),
    )?;
    append_set_cookie(headers, &clear_session_hint_cookie(secure_cookie))?;
    Ok(())
}

/// `POST /auth/github/switch` — clear local session; user re-auths via Continue with GitHub.
pub async fn github_switch(State(state): State<AppState>) -> Response {
    logout_response(&state, "/login?signed_out=1")
}

fn logout_response(state: &AppState, redirect_to: &str) -> Response {
    let secure_cookie = cookie_secure_for_domain(&state.config.siwx_domain);
    let mut headers = HeaderMap::new();
    if clear_session_cookies(&mut headers, secure_cookie).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    (headers, Redirect::to(redirect_to)).into_response()
}

/// `GET /auth/logout` — clear session via navigation (reliable Set-Cookie on full page load).
pub async fn logout_get(State(state): State<AppState>) -> Response {
    logout_response(&state, "/login?signed_out=1")
}

/// `POST /auth/logout` — clear session cookie.
pub async fn logout(State(state): State<AppState>) -> Response {
    logout_response(&state, "/login?signed_out=1")
}

#[cfg(test)]
mod tests {
    use super::{callback_url, clear_cookie, set_cookie};
    use crate::auth::session::{
        cookie_secure_for_domain, ACCESS_TOKEN_COOKIE, GITHUB_STATE_COOKIE,
    };
    use crate::config::Config;

    fn sample_config(siwx_domain: &str, port: u16, github_redirect_uri: Option<&str>) -> Config {
        Config {
            database_url: String::new(),
            supabase_url: "https://example.supabase.co".into(),
            supabase_anon_key: String::new(),
            supabase_service_key: String::new(),
            github_client_id: String::new(),
            github_client_secret: String::new(),
            github_redirect_uri: github_redirect_uri.map(str::to_string),
            google_client_id: None,
            google_client_secret: None,
            google_redirect_uri: None,
            siwx_domain: siwx_domain.into(),
            siwx_session_ttl: 86_400,
            jwt_secret: String::new(),
            github_api_token: None,
            admin_github_logins: Vec::new(),
            port,
        }
    }

    #[test]
    fn callback_url_uses_localhost_port_for_dev() {
        let config = sample_config("localhost:3000", 3000, None);
        assert_eq!(callback_url(&config), "http://localhost:3000/auth/callback");
    }

    #[test]
    fn callback_url_uses_loopback_ip_for_127_dev() {
        let config = sample_config("127.0.0.1:3000", 3000, None);
        assert_eq!(callback_url(&config), "http://127.0.0.1:3000/auth/callback");
    }

    #[test]
    fn callback_url_uses_https_siwx_domain_for_production() {
        let config = sample_config("www.onchain-ai.xyz", 3000, None);
        assert_eq!(
            callback_url(&config),
            "https://www.onchain-ai.xyz/auth/callback"
        );
    }

    #[test]
    fn callback_url_honors_github_redirect_uri_override() {
        let config = sample_config(
            "localhost:3000",
            3000,
            Some("http://127.0.0.1:3000/auth/callback"),
        );
        assert_eq!(callback_url(&config), "http://127.0.0.1:3000/auth/callback");
    }

    #[test]
    fn oauth_state_cookie_is_secure_in_production() {
        assert!(cookie_secure_for_domain("www.onchain-ai.xyz"));
        let cookie = set_cookie(GITHUB_STATE_COOKIE, "abc", 600, true, "Lax");
        assert!(cookie.contains("; Secure"));
        assert!(cookie.contains("SameSite=Lax"));
    }

    #[test]
    fn oauth_state_cookie_omits_secure_on_localhost() {
        assert!(!cookie_secure_for_domain("localhost:3000"));
        assert!(!cookie_secure_for_domain("127.0.0.1:3000"));
        let cookie = set_cookie(GITHUB_STATE_COOKIE, "abc", 600, false, "Lax");
        assert!(!cookie.contains("; Secure"));
    }

    #[test]
    fn clear_github_state_cookie_preserves_secure_in_production() {
        let cookie = clear_cookie(GITHUB_STATE_COOKIE, true, "Lax");
        assert!(cookie.contains("; Secure"));
        assert!(cookie.contains("Max-Age=0"));
        assert!(cookie.contains("SameSite=Lax"));
    }

    #[test]
    fn session_cookie_is_lax_and_secure_in_production() {
        let cookie = set_cookie(ACCESS_TOKEN_COOKIE, "tok", 86_400, true, "Lax");
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("; Secure"));
        assert!(cookie.contains("HttpOnly"));
    }

    #[test]
    fn logout_clears_session_cookie_with_matching_flags() {
        let cookie = clear_cookie(ACCESS_TOKEN_COOKIE, true, "Lax");
        assert!(cookie.contains("; Secure"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Max-Age=0"));
    }

    #[test]
    fn logout_clears_session_hint_cookie_with_matching_flags() {
        use crate::auth::session::{clear_session_hint_cookie, SESSION_HINT_COOKIE};

        let cookie = clear_session_hint_cookie(true);
        assert!(cookie.contains(SESSION_HINT_COOKIE));
        assert!(cookie.contains("; Secure"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Max-Age=0"));
    }

    #[test]
    fn github_switch_clears_session_cookies_with_matching_flags() {
        let access = clear_cookie(ACCESS_TOKEN_COOKIE, true, "Lax");
        assert!(access.contains("; Secure"));
        assert!(access.contains("SameSite=Lax"));
        assert!(access.contains("Max-Age=0"));

        use crate::auth::session::{clear_session_hint_cookie, SESSION_HINT_COOKIE};

        let hint = clear_session_hint_cookie(true);
        assert!(hint.contains(SESSION_HINT_COOKIE));
        assert!(hint.contains("Max-Age=0"));
    }
}
