//! Session cookies and JWT verification for Supabase access tokens.

use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use leptos::server_fn::ServerFnError;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

pub const ACCESS_TOKEN_COOKIE: &str = "onchainai_access_token";
pub const PKCE_VERIFIER_COOKIE: &str = "onchainai_pkce_verifier";

/// Authenticated user resolved from JWT + profiles row.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionUser {
    pub id: Uuid,
    pub nickname: Option<String>,
    pub is_admin: bool,
    pub auth_method: String,
}

/// Minimal Supabase JWT claims (`sub` is the user id).
#[derive(Debug, Deserialize)]
struct SupabaseClaims {
    sub: String,
}

/// Verify a Supabase HS256 access token and return the user id.
pub fn user_id_from_jwt(token: &str, jwt_secret: &str) -> Result<Uuid, AuthSessionError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.leeway = 0;

    let data = decode::<SupabaseClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AuthSessionError::InvalidToken)?;

    Uuid::parse_str(&data.claims.sub).map_err(|_| AuthSessionError::InvalidToken)
}

/// Load profile flags for an authenticated user id.
pub async fn load_session_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<SessionUser, AuthSessionError> {
    let row = sqlx::query_as::<_, ProfileRow>(
        r#"
        SELECT id, nickname, is_admin, is_banned, auth_method
        FROM profiles
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AuthSessionError::Database(e.to_string()))?;

    let Some(row) = row else {
        return Err(AuthSessionError::ProfileMissing);
    };

    if row.is_banned {
        return Err(AuthSessionError::Banned);
    }

    Ok(SessionUser {
        id: row.id,
        nickname: row.nickname,
        is_admin: row.is_admin,
        auth_method: row.auth_method,
    })
}

/// Parse `Cookie` header value and extract the named cookie.
pub fn cookie_value<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    cookie_header.split(';').find_map(|part| {
        let part = part.trim();
        let (key, value) = part.split_once('=')?;
        if key == name {
            Some(value)
        } else {
            None
        }
    })
}

/// Resolve the current user from request `Parts` (Leptos server fn / SSR context).
pub async fn session_from_parts(
    parts: &axum::http::request::Parts,
    pool: &PgPool,
    jwt_secret: &str,
) -> Result<Option<SessionUser>, AuthSessionError> {
    let cookie_header = parts
        .headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok());

    let Some(cookie_header) = cookie_header else {
        return Ok(None);
    };

    let Some(token) = cookie_value(cookie_header, ACCESS_TOKEN_COOKIE) else {
        return Ok(None);
    };

    let user_id = user_id_from_jwt(token, jwt_secret)?;
    let user = load_session_user(pool, user_id).await?;
    Ok(Some(user))
}

/// HS256 access token for SIWX and other server-issued sessions.
#[derive(Debug, Serialize)]
struct AccessClaims {
    sub: String,
    exp: i64,
    iat: i64,
}

/// Mint a Supabase-compatible HS256 JWT (`sub` = profile id).
pub fn issue_access_token(
    user_id: Uuid,
    jwt_secret: &str,
    ttl_secs: i64,
) -> Result<String, AuthSessionError> {
    let now = Utc::now().timestamp();
    let claims = AccessClaims {
        sub: user_id.to_string(),
        exp: now + ttl_secs,
        iat: now,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|_| AuthSessionError::InvalidToken)
}

/// Upsert a SIWX profile keyed by wallet address (reuses existing row when present).
pub async fn ensure_siwx_profile(
    pool: &PgPool,
    wallet_address: &str,
    chain_id: &str,
) -> Result<Uuid, sqlx::Error> {
    let existing = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id FROM profiles
        WHERE wallet_address = $1 AND auth_method = 'siwx'
        LIMIT 1
        "#,
    )
    .bind(wallet_address)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = existing {
        return Ok(id);
    }

    let id = Uuid::new_v4();
    let nickname = siwx_nickname(wallet_address);
    sqlx::query(
        r#"
        INSERT INTO profiles (id, nickname, auth_method, wallet_address, chain_id)
        VALUES ($1, $2, 'siwx', $3, $4)
        "#,
    )
    .bind(id)
    .bind(&nickname)
    .bind(wallet_address)
    .bind(chain_id)
    .execute(pool)
    .await?;

    Ok(id)
}

fn siwx_nickname(wallet: &str) -> String {
    let w = wallet.trim();
    if w.len() <= 12 {
        return w.to_string();
    }
    format!("{}…{}", &w[..6], &w[w.len().saturating_sub(4)..])
}

/// Upsert a profile row after OAuth sign-in (first-user-admin trigger applies on insert).
pub async fn ensure_profile(
    pool: &PgPool,
    user_id: Uuid,
    auth_method: &str,
    nickname: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO profiles (id, nickname, avatar_url, auth_method)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (id) DO UPDATE SET
            avatar_url = COALESCE(EXCLUDED.avatar_url, profiles.avatar_url),
            updated_at = now()
        "#,
    )
    .bind(user_id)
    .bind(nickname)
    .bind(avatar_url)
    .bind(auth_method)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ProfileRow {
    id: Uuid,
    nickname: Option<String>,
    is_admin: bool,
    is_banned: bool,
    auth_method: String,
}

#[derive(Debug)]
pub enum AuthSessionError {
    InvalidToken,
    ProfileMissing,
    Banned,
    Database(String),
}

impl std::fmt::Display for AuthSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "invalid session"),
            Self::ProfileMissing => write!(f, "profile not found"),
            Self::Banned => write!(f, "account suspended"),
            Self::Database(msg) => write!(f, "database error: {msg}"),
        }
    }
}

impl From<AuthSessionError> for ServerFnError {
    fn from(e: AuthSessionError) -> Self {
        ServerFnError::new(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_cookie() {
        let header = "foo=bar; onchainai_access_token=abc123; baz=qux";
        assert_eq!(cookie_value(header, ACCESS_TOKEN_COOKIE), Some("abc123"));
        assert_eq!(cookie_value(header, "missing"), None);
    }
}