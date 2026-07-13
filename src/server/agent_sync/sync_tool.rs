//! Toolkit bookmark sync from agents.
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::tokens::normalize_source_client;
use super::types::*;
use crate::server::fn_error::FnError;
use crate::server::functions::{resolve_bookmark_tool_id, validate_toolkit_tags};

pub async fn sync_tool(
    pool: &PgPool,
    auth: &AgentAuth,
    req: SyncToolRequest,
) -> Result<SyncToolResponse, FnError> {
    if !auth.scopes.iter().any(|s| s == "toolkit:write") {
        return Err(FnError::new("token missing toolkit:write scope"));
    }

    let slug = req.slug.trim().to_string();
    if slug.is_empty() {
        return Err(FnError::new("slug required"));
    }

    if let Some(key) = req.idempotency_key.as_deref().filter(|k| !k.is_empty()) {
        if let Some(cached) = load_idempotent_sync(pool, auth.user_id, key).await? {
            return Ok(cached);
        }
    }

    #[derive(sqlx::FromRow)]
    struct RiskRow {
        install_risk_level: String,
    }
    let risk = sqlx::query_as::<_, RiskRow>(
        r#"
        SELECT install_risk_level
        FROM tools
        WHERE slug = $1
          AND approval_status = 'approved'
          AND relevance_status = 'accepted'
          AND quarantined_at IS NULL
        "#,
    )
    .bind(&slug)
    .fetch_optional(pool)
    .await
    .map_err(|e| FnError::new(format!("tool lookup failed: {e}")))?
    .ok_or_else(|| FnError::new(format!("tool not found: {slug}")))?;

    if risk.install_risk_level == "critical" {
        return Err(FnError::new(
            "critical install risk tools cannot be saved via agent",
        ));
    }

    let tool_id = resolve_bookmark_tool_id(pool, &slug).await?;
    let source_client = normalize_source_client(req.source_client.as_deref())?
        .or_else(|| Some(auth.client.clone()));

    let tags = if let Some(tags) = req.tags.as_ref() {
        validate_toolkit_tags(tags)?
    } else {
        Vec::new()
    };

    let agent_note = req.note.clone().filter(|n| !n.trim().is_empty());

    let existed: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bookmarks WHERE tool_id = $1 AND user_id = $2)",
    )
    .bind(tool_id)
    .bind(auth.user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("bookmark exists check failed: {e}")))?;

    let updated_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        r#"
        INSERT INTO bookmarks (tool_id, user_id, source, source_client, note, tags)
        VALUES ($1, $2, 'agent', $3, $4, $5)
        ON CONFLICT (tool_id, user_id) DO UPDATE SET
          source = CASE WHEN bookmarks.source = 'web' THEN 'agent' ELSE bookmarks.source END,
          source_client = COALESCE(EXCLUDED.source_client, bookmarks.source_client),
          note = CASE
            WHEN bookmarks.note IS NULL AND EXCLUDED.note IS NOT NULL THEN EXCLUDED.note
            ELSE bookmarks.note
          END,
          tags = CASE
            WHEN bookmarks.tags = '{}' AND EXCLUDED.tags <> '{}' THEN EXCLUDED.tags
            ELSE bookmarks.tags
          END,
          updated_at = now()
        RETURNING updated_at
        "#,
    )
    .bind(tool_id)
    .bind(auth.user_id)
    .bind(&source_client)
    .bind(agent_note)
    .bind(&tags)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("bookmark upsert failed: {e}")))?;

    let response = SyncToolResponse {
        ok: true,
        slug: slug.clone(),
        bookmarked: true,
        created: !existed,
        source: "agent".into(),
        updated_at,
    };

    if let Some(key) = req.idempotency_key.as_deref().filter(|k| !k.is_empty()) {
        let _ = record_sync_log(pool, auth, &slug, key, &response).await;
    }

    Ok(response)
}

async fn load_idempotent_sync(
    pool: &PgPool,
    user_id: Uuid,
    key: &str,
) -> Result<Option<SyncToolResponse>, FnError> {
    let detail: Option<serde_json::Value> = sqlx::query_scalar(
        r#"
        SELECT detail
        FROM agent_sync_log
        WHERE user_id = $1 AND idempotency_key = $2 AND status = 'ok'
        "#,
    )
    .bind(user_id)
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(|e| FnError::new(format!("idempotency lookup failed: {e}")))?;

    let Some(detail) = detail else {
        return Ok(None);
    };
    serde_json::from_value(detail)
        .map(Some)
        .map_err(|e| FnError::new(format!("idempotency decode failed: {e}")))
}

async fn record_sync_log(
    pool: &PgPool,
    auth: &AgentAuth,
    slug: &str,
    key: &str,
    response: &SyncToolResponse,
) -> Result<(), FnError> {
    let detail = serde_json::to_value(response)
        .map_err(|e| FnError::new(format!("sync log serialize failed: {e}")))?;
    let _ = sqlx::query(
        r#"
        INSERT INTO agent_sync_log (user_id, agent_token_id, action, tool_slug, idempotency_key, detail)
        VALUES ($1, $2, 'sync_tool', $3, $4, $5)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(auth.user_id)
    .bind(auth.token_id)
    .bind(slug)
    .bind(key)
    .bind(detail)
    .execute(pool)
    .await;
    Ok(())
}

pub fn link_required_payload() -> serde_json::Value {
    serde_json::json!({
        "code": "link_required",
        "message": "Link your OnchainAI account to save tools to your toolkit.",
        "link_url": LINK_URL
    })
}
