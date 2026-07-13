//! Agent token mint / list / revoke / bearer resolve.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, Utc};
use getrandom::getrandom;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::types::*;
use crate::server::fn_error::FnError;

pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn generate_opaque_token() -> Result<String, FnError> {
    let mut bytes = [0u8; 32];
    getrandom(&mut bytes).map_err(|_| FnError::new("random unavailable"))?;
    Ok(format!("{}{}", TOKEN_PREFIX, URL_SAFE_NO_PAD.encode(bytes)))
}

pub(crate) fn token_display_prefix(token: &str) -> String {
    token.chars().take(16).collect()
}

pub(crate) fn normalize_client(client: Option<&str>) -> Result<String, FnError> {
    let c = client.unwrap_or("generic").trim().to_ascii_lowercase();
    match c.as_str() {
        "cursor" | "claude-code" | "windsurf" | "generic" => Ok(c),
        _ => Err(FnError::new(
            "client must be cursor, claude-code, windsurf, or generic",
        )),
    }
}

pub(crate) fn normalize_source_client(client: Option<&str>) -> Result<Option<String>, FnError> {
    let Some(c) = client else {
        return Ok(None);
    };
    let c = c.trim().to_ascii_lowercase();
    match c.as_str() {
        "cursor" | "claude-code" | "windsurf" | "mcp" | "generic" => Ok(Some(c)),
        _ => Err(FnError::new(
            "source_client must be cursor, claude-code, windsurf, mcp, or generic",
        )),
    }
}

pub(crate) fn normalize_user_code_input(code: &str) -> String {
    let raw: String = code
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_ascii_uppercase();
    if raw.len() == 8 {
        format!("{}-{}", &raw[..4], &raw[4..])
    } else {
        code.trim().to_ascii_uppercase()
    }
}

pub(crate) fn generate_user_code() -> Result<String, FnError> {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut bytes = [0u8; 8];
    getrandom(&mut bytes).map_err(|_| FnError::new("random unavailable"))?;
    let raw: String = bytes
        .iter()
        .map(|b| CHARSET[(*b as usize) % CHARSET.len()] as char)
        .collect();
    Ok(format!("{}-{}", &raw[..4], &raw[4..]))
}

pub(crate) fn generate_device_code() -> Result<String, FnError> {
    let mut bytes = [0u8; 32];
    getrandom(&mut bytes).map_err(|_| FnError::new("random unavailable"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub async fn count_active_tokens(pool: &PgPool, user_id: Uuid) -> Result<i64, FnError> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM agent_tokens
        WHERE user_id = $1
          AND revoked_at IS NULL
          AND expires_at > now()
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("token count failed: {e}")))
}

pub async fn resolve_bearer(pool: &PgPool, authorization: Option<&str>) -> Option<AgentAuth> {
    let header = authorization?;
    let token = header.strip_prefix("Bearer ")?.trim();
    if !token.starts_with(TOKEN_PREFIX) || token.len() < TOKEN_PREFIX.len() + 16 {
        return None;
    }
    let hash = hash_token(token);
    #[derive(sqlx::FromRow)]
    struct Row {
        id: Uuid,
        user_id: Uuid,
        client: String,
        scopes: Vec<String>,
    }
    let row = sqlx::query_as::<_, Row>(
        r#"
        SELECT id, user_id, client, scopes
        FROM agent_tokens
        WHERE token_hash = $1
          AND revoked_at IS NULL
          AND expires_at > now()
        "#,
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;

    let _ =
        sqlx::query("UPDATE agent_tokens SET last_used_at = now() WHERE id = $1 AND user_id = $2")
            .bind(row.id)
            .bind(row.user_id)
            .execute(pool)
            .await;

    Some(AgentAuth {
        user_id: row.user_id,
        token_id: row.id,
        client: row.client,
        scopes: row.scopes,
    })
}

pub async fn mint_token(
    pool: &PgPool,
    user_id: Uuid,
    label: Option<String>,
    client: Option<&str>,
    expires_in_days: Option<i64>,
) -> Result<MintTokenResponse, FnError> {
    let active = count_active_tokens(pool, user_id).await?;
    if active >= MAX_ACTIVE_TOKENS {
        return Err(FnError::new(format!(
            "at most {MAX_ACTIVE_TOKENS} active agent tokens; revoke one first"
        )));
    }

    let client = normalize_client(client)?;
    let days = expires_in_days
        .unwrap_or(DEFAULT_TOKEN_TTL_DAYS)
        .clamp(1, 365);
    let label = label
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| "Agent link".into());
    if label.len() > 80 {
        return Err(FnError::new("label must be at most 80 characters"));
    }

    let plaintext = generate_opaque_token()?;
    let prefix = token_display_prefix(&plaintext);
    let hash = hash_token(&plaintext);
    let expires_at = Utc::now() + Duration::days(days);

    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO agent_tokens (user_id, label, client, token_prefix, token_hash, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(&label)
    .bind(&client)
    .bind(&prefix)
    .bind(&hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("token mint failed: {e}")))?;

    Ok(MintTokenResponse {
        id,
        token: plaintext,
        token_prefix: prefix,
        expires_at,
    })
}

pub async fn list_tokens(pool: &PgPool, user_id: Uuid) -> Result<Vec<AgentTokenListItem>, FnError> {
    sqlx::query_as(
        r#"
        SELECT id, label, token_prefix, client, last_used_at, expires_at, revoked_at, created_at
        FROM agent_tokens
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 20
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| FnError::new(format!("token list failed: {e}")))
}

pub async fn revoke_token(pool: &PgPool, user_id: Uuid, token_id: Uuid) -> Result<(), FnError> {
    let result = sqlx::query(
        r#"
        UPDATE agent_tokens
        SET revoked_at = now()
        WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(token_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| FnError::new(format!("token revoke failed: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(FnError::new("token not found"));
    }
    Ok(())
}

pub async fn has_active_link(pool: &PgPool, user_id: Uuid) -> Result<bool, FnError> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM agent_tokens
        WHERE user_id = $1
          AND revoked_at IS NULL
          AND expires_at > now()
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("link status failed: {e}")))?;
    Ok(count > 0)
}

