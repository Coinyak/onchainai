//! SSR-only session helpers (DB, JWT minting, profile upserts).

use super::{cookie_value, AuthSessionError, SessionUser, ACCESS_TOKEN_COOKIE};
use crate::config::Config;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use getrandom::getrandom;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Required JWT `aud` claim. Supabase stamps `authenticated` on user tokens;
/// server-minted SIWX/GitHub tokens use the same audience.
pub const JWT_AUDIENCE: &str = "authenticated";

/// Minimal Supabase JWT claims (`sub` is the user id).
#[derive(Debug, Deserialize)]
struct SupabaseClaims {
    sub: String,
}

/// Verify an HS256 access token and return the user id.
///
/// Enforces the full claim set required by `docs/SECURITY.md`: signature,
/// `exp`, `nbf` (when present), `iss`, `aud`, and a parseable `sub`. The
/// `nbf` claim is validated only when present so Supabase-issued tokens
/// (which omit it) still pass, while server-minted tokens that include it
/// are checked.
pub fn user_id_from_jwt(
    token: &str,
    jwt_secret: &str,
    issuer: &str,
) -> Result<Uuid, AuthSessionError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.validate_nbf = true;
    validation.leeway = 0;
    validation.set_issuer(&[issuer]);
    validation.set_audience(&[JWT_AUDIENCE]);
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);

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

/// Resolve the current user from request `Parts` (Leptos server fn / SSR context).
pub async fn session_from_parts(
    parts: &axum::http::request::Parts,
    pool: &PgPool,
    jwt_secret: &str,
    issuer: &str,
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

    let user_id = user_id_from_jwt(token, jwt_secret, issuer)?;
    let user = load_session_user(pool, user_id).await?;
    Ok(Some(user))
}

/// HS256 access token for SIWX and other server-issued sessions.
#[derive(Debug, Serialize)]
struct AccessClaims {
    sub: String,
    exp: i64,
    iat: i64,
    nbf: i64,
    iss: String,
    aud: String,
}

/// Mint a Supabase-compatible HS256 JWT (`sub` = profile id).
///
/// Stamps `iss`/`aud`/`nbf` so the minted token satisfies the same
/// validation [`user_id_from_jwt`] applies to Supabase tokens.
pub fn issue_access_token(
    user_id: Uuid,
    jwt_secret: &str,
    ttl_secs: i64,
    issuer: &str,
) -> Result<String, AuthSessionError> {
    let now = Utc::now().timestamp();
    let claims = AccessClaims {
        sub: user_id.to_string(),
        exp: now + ttl_secs,
        iat: now,
        nbf: now,
        iss: issuer.to_string(),
        aud: JWT_AUDIENCE.to_string(),
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
    config: &Config,
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

    let nickname = siwx_nickname(wallet_address);
    let id = create_supabase_user_for_siwx(config, wallet_address, chain_id, &nickname)
        .await
        .map_err(sqlx::Error::Protocol)?;

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

#[derive(Debug, Deserialize)]
struct AdminUserResponse {
    id: Uuid,
}

/// Upsert a GitHub profile keyed by sanitized login (reuses existing row when present).
pub async fn ensure_github_profile(
    pool: &PgPool,
    config: &Config,
    github_id: i64,
    login: &str,
    avatar_url: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    let nickname = sanitize_nickname(login).unwrap_or_else(|| {
        let fallback: String = login
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(12)
            .collect();
        if fallback.len() >= 2 {
            fallback
        } else {
            format!(
                "gh{}",
                Uuid::new_v4()
                    .simple()
                    .to_string()
                    .chars()
                    .take(8)
                    .collect::<String>()
            )
        }
    });

    let existing = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id FROM profiles
        WHERE auth_method = 'github' AND nickname = $1
        LIMIT 1
        "#,
    )
    .bind(&nickname)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = existing {
        if avatar_url.is_some() {
            sqlx::query(
                r#"
                UPDATE profiles
                SET avatar_url = COALESCE($2, avatar_url), updated_at = now()
                WHERE id = $1
                "#,
            )
            .bind(id)
            .bind(avatar_url)
            .execute(pool)
            .await?;
        }
        return Ok(id);
    }

    let id = create_supabase_user_for_github(config, github_id, login, avatar_url)
        .await
        .map_err(sqlx::Error::Protocol)?;

    sqlx::query(
        r#"
        INSERT INTO profiles (id, nickname, avatar_url, auth_method)
        VALUES ($1, $2, $3, 'github')
        "#,
    )
    .bind(id)
    .bind(&nickname)
    .bind(avatar_url)
    .execute(pool)
    .await?;

    Ok(id)
}

async fn create_supabase_user_for_siwx(
    config: &Config,
    wallet_address: &str,
    chain_id: &str,
    nickname: &str,
) -> Result<Uuid, String> {
    let wallet_key = wallet_address.trim().to_lowercase();
    let email = format!("siwx-{wallet_key}@oauth.onchainai.local");
    let password = random_password();

    let client = reqwest::Client::builder()
        .user_agent("OnchainAI/0.1")
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/auth/v1/admin/users", config.supabase_url);
    let response = client
        .post(url)
        .header("apikey", &config.supabase_service_key)
        .header(
            "Authorization",
            format!("Bearer {}", config.supabase_service_key),
        )
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "email_confirm": true,
            "user_metadata": {
                "user_name": nickname,
                "preferred_username": nickname,
                "wallet_address": wallet_address,
                "chain_id": chain_id,
            },
            "app_metadata": {
                "provider": "siwx",
                "providers": ["siwx"]
            }
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "supabase admin create user failed ({status}): {body}"
        ));
    }

    let user = response
        .json::<AdminUserResponse>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(user.id)
}

async fn create_supabase_user_for_github(
    config: &Config,
    github_id: i64,
    login: &str,
    avatar_url: Option<&str>,
) -> Result<Uuid, String> {
    let email = format!("github-{github_id}@oauth.onchainai.local");
    let password = random_password();

    let client = reqwest::Client::builder()
        .user_agent("OnchainAI/0.1")
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/auth/v1/admin/users", config.supabase_url);
    let response = client
        .post(url)
        .header("apikey", &config.supabase_service_key)
        .header(
            "Authorization",
            format!("Bearer {}", config.supabase_service_key),
        )
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "email_confirm": true,
            "user_metadata": {
                "user_name": login,
                "preferred_username": login,
                "avatar_url": avatar_url,
            },
            "app_metadata": {
                "provider": "github",
                "providers": ["github"]
            }
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "supabase admin create user failed ({status}): {body}"
        ));
    }

    let user = response
        .json::<AdminUserResponse>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(user.id)
}

fn random_password() -> String {
    let mut bytes = [0u8; 32];
    getrandom(&mut bytes).expect("OS random unavailable");
    format!("Gh!{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn sanitize_nickname(raw: &str) -> Option<String> {
    let sanitized: String = raw
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

/// Post-auth redirect — onboarding gate for new profiles.
pub async fn post_auth_redirect_path(pool: &PgPool, user_id: Uuid) -> String {
    let done = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT onboarding_completed_at IS NOT NULL
        FROM profiles
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(false);

    if done {
        "/".into()
    } else {
        "/onboarding/profile".into()
    }
}

fn validate_nickname(raw: &str) -> Option<String> {
    let sanitized: String = raw
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(20)
        .collect();
    if (2..=20).contains(&sanitized.len()) {
        Some(sanitized)
    } else {
        None
    }
}

pub fn auto_nickname() -> String {
    let mut bytes = [0u8; 2];
    getrandom(&mut bytes).expect("OS random unavailable");
    format!("user-{:04x}", u16::from_be_bytes(bytes))
}

/// Complete first-login onboarding (nickname optional when `skip`).
pub async fn complete_onboarding(
    pool: &PgPool,
    user_id: Uuid,
    nickname: Option<&str>,
    bio: Option<&str>,
    skip: bool,
) -> Result<(), AuthSessionError> {
    let nick = if skip {
        nickname
            .and_then(validate_nickname)
            .unwrap_or_else(auto_nickname)
    } else {
        nickname
            .and_then(validate_nickname)
            .ok_or(AuthSessionError::InvalidToken)?
    };

    if let Some(bio) = bio {
        let trimmed = bio.trim();
        if trimmed.len() > 200 {
            return Err(AuthSessionError::InvalidToken);
        }
    }

    let taken = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM profiles
            WHERE nickname = $1 AND id <> $2
        )
        "#,
    )
    .bind(&nick)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AuthSessionError::Database(e.to_string()))?;

    if taken {
        return Err(AuthSessionError::InvalidToken);
    }

    sqlx::query(
        r#"
        UPDATE profiles
        SET nickname = $2,
            bio = COALESCE($3, bio),
            onboarding_completed_at = now(),
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(&nick)
    .bind(bio.map(str::trim).filter(|s| !s.is_empty()))
    .execute(pool)
    .await
    .map_err(|e| AuthSessionError::Database(e.to_string()))?;

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{issue_access_token, user_id_from_jwt, JWT_AUDIENCE};
    use chrono::Utc;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use serde::Serialize;
    use uuid::Uuid;

    const SECRET: &str = "test-secret-at-least-32-bytes-long-aaaa";
    const ISSUER: &str = "https://proj.supabase.co/auth/v1";

    #[derive(Serialize)]
    struct TestClaims {
        sub: String,
        exp: i64,
        iss: String,
        aud: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        nbf: Option<i64>,
    }

    fn encode_token<T: Serialize>(claims: &T) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap()
    }

    fn claims(aud: &str, exp_offset: i64, nbf: Option<i64>) -> TestClaims {
        TestClaims {
            sub: Uuid::new_v4().to_string(),
            exp: Utc::now().timestamp() + exp_offset,
            iss: ISSUER.into(),
            aud: aud.into(),
            nbf,
        }
    }

    #[test]
    fn self_minted_token_roundtrips() {
        let uid = Uuid::new_v4();
        let token = issue_access_token(uid, SECRET, 3600, ISSUER).unwrap();
        assert_eq!(user_id_from_jwt(&token, SECRET, ISSUER).unwrap(), uid);
    }

    #[test]
    fn supabase_style_token_without_nbf_validates() {
        // Supabase access tokens omit `nbf`; validation must still accept them.
        let c = claims(JWT_AUDIENCE, 3600, None);
        let token = encode_token(&c);
        assert_eq!(
            user_id_from_jwt(&token, SECRET, ISSUER)
                .unwrap()
                .to_string(),
            c.sub
        );
    }

    #[test]
    fn wrong_issuer_rejected() {
        let token = issue_access_token(Uuid::new_v4(), SECRET, 3600, ISSUER).unwrap();
        assert!(user_id_from_jwt(&token, SECRET, "https://evil.example/auth/v1").is_err());
    }

    #[test]
    fn wrong_secret_rejected() {
        let token = issue_access_token(Uuid::new_v4(), SECRET, 3600, ISSUER).unwrap();
        assert!(user_id_from_jwt(&token, "a-totally-different-secret-value-zz", ISSUER).is_err());
    }

    #[test]
    fn wrong_audience_rejected() {
        let token = encode_token(&claims("anon", 3600, None));
        assert!(user_id_from_jwt(&token, SECRET, ISSUER).is_err());
    }

    #[test]
    fn missing_issuer_claim_rejected() {
        #[derive(Serialize)]
        struct NoIss {
            sub: String,
            exp: i64,
            aud: String,
        }
        let token = encode_token(&NoIss {
            sub: Uuid::new_v4().to_string(),
            exp: Utc::now().timestamp() + 3600,
            aud: JWT_AUDIENCE.into(),
        });
        assert!(user_id_from_jwt(&token, SECRET, ISSUER).is_err());
    }

    #[test]
    fn expired_token_rejected() {
        let token = encode_token(&claims(JWT_AUDIENCE, -10, None));
        assert!(user_id_from_jwt(&token, SECRET, ISSUER).is_err());
    }

    #[test]
    fn future_nbf_rejected() {
        let token = encode_token(&claims(
            JWT_AUDIENCE,
            3600,
            Some(Utc::now().timestamp() + 600),
        ));
        assert!(user_id_from_jwt(&token, SECRET, ISSUER).is_err());
    }
}
