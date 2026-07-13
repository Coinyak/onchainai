//! Device authorization flow for agent linking.
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::tokens::{
    generate_device_code, generate_user_code, hash_token, mint_token, normalize_client,
    normalize_user_code_input, revoke_token,
};
use super::types::*;
use crate::config::SITE_ORIGIN;
use crate::server::fn_error::FnError;

pub async fn device_start(
    pool: &PgPool,
    client: Option<&str>,
) -> Result<DeviceStartResponse, FnError> {
    let client = normalize_client(client)?;
    let device_code = generate_device_code()?;
    let user_code = generate_user_code()?;
    let device_hash = hash_token(&device_code);
    let expires_at = Utc::now() + Duration::minutes(DEVICE_SESSION_TTL_MINUTES);

    sqlx::query(
        r#"
        INSERT INTO agent_device_sessions (device_code_hash, user_code, client, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(&device_hash)
    .bind(&user_code)
    .bind(&client)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|e| FnError::new(format!("device session start failed: {e}")))?;

    Ok(DeviceStartResponse {
        device_code,
        user_code: user_code.clone(),
        verification_uri: format!("{SITE_ORIGIN}/connect#agent-sync"),
        expires_in: DEVICE_SESSION_TTL_MINUTES * 60,
        interval: DEVICE_POLL_INTERVAL_SECS,
    })
}

pub async fn device_approve(
    pool: &PgPool,
    user_id: Uuid,
    user_code: &str,
    label: Option<String>,
) -> Result<MintTokenResponse, FnError> {
    let normalized = normalize_user_code_input(user_code);
    #[derive(sqlx::FromRow)]
    struct SessionRow {
        id: Uuid,
        client: String,
        status: String,
        expires_at: DateTime<Utc>,
    }

    let session = sqlx::query_as::<_, SessionRow>(
        r#"
        SELECT id, client, status, expires_at
        FROM agent_device_sessions
        WHERE user_code = $1
        "#,
    )
    .bind(&normalized)
    .fetch_optional(pool)
    .await
    .map_err(|e| FnError::new(format!("device lookup failed: {e}")))?
    .ok_or_else(|| FnError::new("invalid or expired code"))?;

    if session.expires_at < Utc::now() || session.status == "expired" {
        return Err(FnError::new(
            "code expired; start a new link from your agent",
        ));
    }
    if session.status != "pending" {
        return Err(FnError::new("code already used"));
    }

    let minted = mint_token(
        pool,
        user_id,
        label.or(Some(format!("{} link", session.client))),
        Some(&session.client),
        None,
    )
    .await?;

    let updated = sqlx::query(
        r#"
        UPDATE agent_device_sessions
        SET status = 'approved',
            user_id = $2,
            agent_token_id = $3,
            pending_token = $4,
            approved_at = now()
        WHERE id = $1 AND status = 'pending'
        "#,
    )
    .bind(session.id)
    .bind(user_id)
    .bind(minted.id)
    .bind(&minted.token)
    .execute(pool)
    .await
    .map_err(|e| FnError::new(format!("device approve failed: {e}")))?;

    if updated.rows_affected() == 0 {
        // Lost a concurrent approve race — revoke the token we just minted so
        // no orphaned-but-valid credential remains.
        let _ = revoke_token(pool, user_id, minted.id).await;
        return Err(FnError::new("code already used"));
    }

    Ok(MintTokenResponse {
        id: minted.id,
        token: String::new(),
        token_prefix: minted.token_prefix,
        expires_at: minted.expires_at,
    })
}

pub async fn device_poll(pool: &PgPool, device_code: &str) -> Result<DevicePollResponse, FnError> {
    let hash = hash_token(device_code.trim());
    #[derive(sqlx::FromRow)]
    struct Row {
        status: String,
        expires_at: DateTime<Utc>,
    }

    let row = sqlx::query_as::<_, Row>(
        r#"
        SELECT status, expires_at, agent_token_id
        FROM agent_device_sessions
        WHERE device_code_hash = $1
        "#,
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| FnError::new(format!("device poll failed: {e}")))?
    .ok_or_else(|| FnError::new("unknown device session"))?;

    if row.expires_at < Utc::now() {
        // Expire the session and drop any unclaimed plaintext token so it
        // never lingers at rest past the TTL.
        let _ = sqlx::query(
            r#"
            UPDATE agent_device_sessions
            SET status = CASE WHEN status = 'pending' THEN 'expired' ELSE status END,
                pending_token = NULL
            WHERE device_code_hash = $1
            "#,
        )
        .bind(&hash)
        .execute(pool)
        .await;
        return Ok(DevicePollResponse::Expired);
    }

    if row.status == "pending" {
        return Ok(DevicePollResponse::Pending);
    }

    if row.status == "consumed" {
        return Ok(DevicePollResponse::Expired);
    }

    if row.status != "approved" {
        return Ok(DevicePollResponse::Expired);
    }

    #[derive(sqlx::FromRow)]
    struct ApprovedRow {
        pending_token: Option<String>,
        token_prefix: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    }

    let approved = sqlx::query_as::<_, ApprovedRow>(
        r#"
        SELECT d.pending_token, t.token_prefix, t.expires_at
        FROM agent_device_sessions d
        LEFT JOIN agent_tokens t ON t.id = d.agent_token_id
        WHERE d.device_code_hash = $1
        "#,
    )
    .bind(&hash)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("device approved load failed: {e}")))?;

    let Some(token) = approved.pending_token.filter(|t| !t.is_empty()) else {
        return Ok(DevicePollResponse::Expired);
    };

    sqlx::query(
        r#"
        UPDATE agent_device_sessions
        SET status = 'consumed', pending_token = NULL
        WHERE device_code_hash = $1
        "#,
    )
    .bind(&hash)
    .execute(pool)
    .await
    .map_err(|e| FnError::new(format!("device consume failed: {e}")))?;

    Ok(DevicePollResponse::Approved {
        token,
        token_prefix: approved.token_prefix.unwrap_or_default(),
        expires_at: approved.expires_at.unwrap_or_else(Utc::now),
    })
}
