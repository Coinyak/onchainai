//! Email magic-link authentication via Supabase Auth.

use crate::auth::session::{
    ensure_profile, post_auth_redirect_path, ACCESS_TOKEN_COOKIE, PKCE_VERIFIER_COOKIE,
};
use crate::config::Config;
use crate::AppState;
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use supabase_auth::models::{AuthClient, OtpType, VerifyOtpParams, VerifyTokenHashParams};

#[derive(Debug, Deserialize)]
pub struct EmailLoginRequest {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct EmailLoginResponse {
    pub ok: bool,
}

fn auth_client(config: &Config) -> AuthClient {
    AuthClient::new(
        config.supabase_url.clone(),
        config.supabase_anon_key.clone(),
        config.jwt_secret.clone(),
    )
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

fn validate_email(email: &str) -> Result<&str, StatusCode> {
    let email = email.trim();
    if email.is_empty() || email.len() > 254 || !email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(email)
}

fn email_nickname(email: &str) -> Option<String> {
    let local = email.split('@').next()?.trim();
    let sanitized: String = local
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(20)
        .collect();
    if sanitized.len() >= 2 {
        Some(sanitized)
    } else {
        None
    }
}

/// `POST /auth/email` — send a Supabase magic-link email.
pub async fn send_magic_link(
    State(state): State<AppState>,
    Json(body): Json<EmailLoginRequest>,
) -> Result<Json<EmailLoginResponse>, StatusCode> {
    let email = validate_email(&body.email)?;
    let client = auth_client(&state.config);
    client
        .send_login_email_with_magic_link(email)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    Ok(Json(EmailLoginResponse { ok: true }))
}

/// Complete a magic-link callback (`token_hash` query param from Supabase).
pub async fn complete_magic_link(
    pool: &sqlx::PgPool,
    config: &Config,
    token_hash: &str,
) -> Result<Response, StatusCode> {
    let client = auth_client(config);
    let session = client
        .verify_otp(VerifyOtpParams::TokenHash(VerifyTokenHashParams {
            token_hash: token_hash.to_string(),
            otp_type: OtpType::Magiclink,
        }))
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let nickname = email_nickname(&session.user.email);
    ensure_profile(
        pool,
        session.user.id,
        "email",
        nickname.as_deref(),
        None,
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

    let redirect_to = post_auth_redirect_path(pool, session.user.id).await;

    Ok((headers, axum::response::Redirect::to(&redirect_to)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_nickname_from_local_part() {
        assert_eq!(email_nickname("alice@example.com").as_deref(), Some("alice"));
    }

    #[test]
    fn validate_email_rejects_empty() {
        assert!(validate_email("").is_err());
    }
}