//! Google OAuth 2.0 sign-in (authorization code flow, confidential client).
//!
//! Mirrors the GitHub flow in [`crate::auth::routes`]: a random `state` in an
//! HttpOnly cookie provides CSRF protection across the redirect, the code is
//! exchanged server-side with the client secret, and the resolved profile gets
//! a server-minted session JWT (same issuer/audience as GitHub/SIWX). Google
//! sign-in is inert unless `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` are
//! configured — `/auth/google` then short-circuits to a "not configured"
//! redirect so the server still boots without Google credentials.

use crate::auth::oauth_state::{mint_oauth_state, verify_oauth_state};
use crate::auth::session::{
    auth_http_client, cookie_secure_for_domain, cookie_value, ensure_google_profile,
    issue_access_token, local_dev_host, post_auth_redirect_path, set_session_hint_cookie,
    ACCESS_TOKEN_COOKIE, GOOGLE_STATE_COOKIE,
};
use crate::config::Config;
use crate::AppState;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

const AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";
const SCOPE: &str = "openid email profile";

#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: Option<String>,
    pub error: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUserinfo {
    sub: String,
    email: Option<String>,
    name: Option<String>,
    picture: Option<String>,
}

fn callback_url(config: &Config) -> String {
    if let Some(uri) = config
        .google_redirect_uri
        .as_deref()
        .map(str::trim)
        .filter(|uri| !uri.is_empty())
    {
        return uri.to_string();
    }
    if let Some(host) = local_dev_host(&config.siwx_domain) {
        format!("http://{host}:{}/auth/google/callback", config.port)
    } else {
        format!("https://{}/auth/google/callback", config.siwx_domain)
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

fn google_oauth_state_valid(config: &Config, headers: &HeaderMap, state_param: &str) -> bool {
    let cookie_header = headers.get(header::COOKIE).and_then(|v| v.to_str().ok());
    let cookie_state = cookie_header.and_then(|h| cookie_value(h, GOOGLE_STATE_COOKIE));
    if cookie_state == Some(state_param) {
        return true;
    }
    verify_oauth_state(&config.jwt_secret, state_param)
}

/// `GET /auth/google` — redirect to Google OAuth (callback returns to this app).
pub async fn google_login(State(state): State<AppState>) -> Result<Response, StatusCode> {
    let config = &state.config;
    let Some(client_id) = config.google_client_id.as_deref().filter(|s| !s.is_empty()) else {
        return Ok(Redirect::to("/login?auth=google_not_configured").into_response());
    };

    let oauth_state = mint_oauth_state(&config.jwt_secret);
    let redirect_uri = callback_url(config);
    let authorize_url = format!(
        "{AUTHORIZE_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&access_type=online&prompt=select_account",
        urlencoding::encode(client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(SCOPE),
        urlencoding::encode(&oauth_state),
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        // State cookie must survive the cross-site redirect back from Google,
        // so it stays SameSite=Lax (Strict would drop it on the top-level
        // navigation and break the callback).
        set_cookie(
            GOOGLE_STATE_COOKIE,
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

async fn exchange_google_code(
    config: &Config,
    code: &str,
) -> Result<GoogleTokenResponse, StatusCode> {
    let client_id = config.google_client_id.as_deref().unwrap_or_default();
    let client_secret = config.google_client_secret.as_deref().unwrap_or_default();
    let redirect_uri = callback_url(config);

    let client = auth_http_client().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = client
        .post(TOKEN_URL)
        .header(header::ACCEPT, "application/json")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !response.status().is_success() {
        return Err(StatusCode::BAD_REQUEST);
    }

    response
        .json::<GoogleTokenResponse>()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)
}

async fn fetch_google_userinfo(access_token: &str) -> Result<GoogleUserinfo, StatusCode> {
    let client = auth_http_client().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = client
        .get(USERINFO_URL)
        .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !response.status().is_success() {
        return Err(StatusCode::BAD_REQUEST);
    }

    response
        .json::<GoogleUserinfo>()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)
}

/// `GET /auth/google/callback` — validate state, exchange code, set session.
pub async fn google_callback(
    State(state): State<AppState>,
    Query(query): Query<GoogleCallbackQuery>,
    headers_in: HeaderMap,
) -> Result<Response, StatusCode> {
    if query.error.is_some() {
        return Ok(Redirect::to("/login?auth=google_denied").into_response());
    }

    let config = &state.config;
    if !config.google_oauth_enabled() {
        return Ok(Redirect::to("/login?auth=google_not_configured").into_response());
    }

    let Some(code) = query.code.filter(|c| !c.is_empty()) else {
        return Ok(Redirect::to("/login?auth=google_missing_code").into_response());
    };
    let Some(state_param) = query.state.filter(|s| !s.is_empty()) else {
        return Ok(Redirect::to("/login?auth=google_missing_state").into_response());
    };

    if !google_oauth_state_valid(config, &headers_in, &state_param) {
        tracing::warn!("Google OAuth state validation failed (cookie and HMAC mismatch)");
        return Ok(Redirect::to("/login?auth=google_state_mismatch").into_response());
    }

    let token = match exchange_google_code(config, &code).await {
        Ok(token) => token,
        Err(_) => {
            return Ok(Redirect::to("/login?auth=google_token_exchange").into_response());
        }
    };
    let userinfo = match fetch_google_userinfo(&token.access_token).await {
        Ok(user) => user,
        Err(_) => {
            return Ok(Redirect::to("/login?auth=google_user_fetch").into_response());
        }
    };

    let user_id = match ensure_google_profile(
        &state.pool,
        &state.config,
        &userinfo.sub,
        userinfo.email.as_deref(),
        userinfo.name.as_deref(),
        userinfo.picture.as_deref(),
    )
    .await
    {
        Ok(id) => id,
        Err(err) => {
            tracing::error!(error = %err, google_sub = %userinfo.sub, "Google profile setup failed");
            let code = err.auth_query_code_google();
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
        // Session cookie is SameSite=Lax so it is sent on the top-level
        // navigation back from Google (see routes.rs for the full rationale).
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
        clear_cookie(GOOGLE_STATE_COOKIE, secure_cookie, "Lax")
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    let redirect_to = post_auth_redirect_path(&state.pool, user_id).await;

    Ok((headers, Redirect::to(&redirect_to)).into_response())
}

#[cfg(test)]
mod tests {
    use super::{callback_url, clear_cookie, set_cookie};
    use crate::auth::session::{
        cookie_secure_for_domain, ACCESS_TOKEN_COOKIE, GOOGLE_STATE_COOKIE,
    };
    use crate::config::Config;

    fn sample_config(siwx_domain: &str, port: u16, google_redirect_uri: Option<&str>) -> Config {
        Config {
            database_url: String::new(),
            supabase_url: "https://example.supabase.co".into(),
            supabase_anon_key: String::new(),
            supabase_service_key: String::new(),
            github_client_id: String::new(),
            github_client_secret: String::new(),
            github_redirect_uri: None,
            google_client_id: Some("google-id".into()),
            google_client_secret: Some("google-secret".into()),
            google_redirect_uri: google_redirect_uri.map(str::to_string),
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
        assert_eq!(
            callback_url(&config),
            "http://localhost:3000/auth/google/callback"
        );
    }

    #[test]
    fn callback_url_uses_https_siwx_domain_for_production() {
        let config = sample_config("www.onchain-ai.xyz", 3000, None);
        assert_eq!(
            callback_url(&config),
            "https://www.onchain-ai.xyz/auth/google/callback"
        );
    }

    #[test]
    fn callback_url_honors_redirect_uri_override() {
        let config = sample_config(
            "localhost:3000",
            3000,
            Some("http://127.0.0.1:3000/auth/google/callback"),
        );
        assert_eq!(
            callback_url(&config),
            "http://127.0.0.1:3000/auth/google/callback"
        );
    }

    #[test]
    fn state_cookie_is_secure_and_lax_in_production() {
        assert!(cookie_secure_for_domain("www.onchain-ai.xyz"));
        let cookie = set_cookie(GOOGLE_STATE_COOKIE, "abc", 600, true, "Lax");
        assert!(cookie.contains("; Secure"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("HttpOnly"));
    }

    #[test]
    fn state_cookie_omits_secure_on_localhost() {
        assert!(!cookie_secure_for_domain("localhost:3000"));
        let cookie = set_cookie(GOOGLE_STATE_COOKIE, "abc", 600, false, "Lax");
        assert!(!cookie.contains("; Secure"));
    }

    #[test]
    fn session_cookie_cleared_with_matching_flags() {
        let cookie = clear_cookie(ACCESS_TOKEN_COOKIE, true, "Lax");
        assert!(cookie.contains("; Secure"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Max-Age=0"));
    }

    #[test]
    fn google_oauth_enabled_reflects_credentials() {
        let mut config = sample_config("www.onchain-ai.xyz", 3000, None);
        assert!(config.google_oauth_enabled());
        config.google_client_secret = None;
        assert!(!config.google_oauth_enabled());
    }
}
