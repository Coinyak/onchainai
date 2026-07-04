//! Agent Sync — token mint, device flow, toolkit upsert for coding-tool clients.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, Utc};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::SITE_ORIGIN;
use crate::server::fn_error::FnError;
use crate::server::functions::{resolve_bookmark_tool_id, validate_toolkit_tags};

pub const TOKEN_PREFIX: &str = "oai_ag_";
pub const MAX_ACTIVE_TOKENS: i64 = 5;
pub const DEFAULT_TOKEN_TTL_DAYS: i64 = 90;
pub const DEVICE_SESSION_TTL_MINUTES: i64 = 15;
pub const DEVICE_POLL_INTERVAL_SECS: u64 = 5;
pub const LINK_URL: &str = "https://www.onchain-ai.xyz/connect#agent-sync";
const MAX_BLUEPRINTS_PER_USER: i64 = 20;
const BLUEPRINT_MAX_NODES: usize = 120;
const BLUEPRINT_GRID: i32 = 8;
const BLUEPRINT_NODE_TOOL_HEIGHT: i32 = 64;
const AGENT_SESSION_START_X: i32 = 40;
const AGENT_SESSION_START_Y: i32 = 40;
const AGENT_NODE_STACK_GAP: i32 = 8;
const MAX_TOOL_NODE_CHAINS: usize = 8;
const MAX_CHAIN_ID_LEN: usize = 64;

#[derive(Debug, Clone)]
pub struct AgentAuth {
    pub user_id: Uuid,
    pub token_id: Uuid,
    pub client: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MintTokenResponse {
    pub id: Uuid,
    pub token: String,
    pub token_prefix: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AgentTokenListItem {
    pub id: Uuid,
    pub label: String,
    pub token_prefix: String,
    pub client: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DeviceStartResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: i64,
    pub interval: u64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DevicePollResponse {
    Pending,
    Approved {
        token: String,
        token_prefix: String,
        expires_at: DateTime<Utc>,
    },
    Expired,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncToolResponse {
    pub ok: bool,
    pub slug: String,
    pub bookmarked: bool,
    pub created: bool,
    pub source: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SyncToolRequest {
    pub slug: String,
    pub note: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source_client: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SyncBlueprintNodeRequest {
    pub blueprint_id: Option<Uuid>,
    pub slug: String,
    pub chains: Option<Vec<String>>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncBlueprintNodeResponse {
    pub ok: bool,
    pub slug: String,
    pub blueprint_id: Uuid,
    pub node_id: Option<String>,
    pub appended: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    pub updated_at: DateTime<Utc>,
}

pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn generate_opaque_token() -> Result<String, FnError> {
    let mut bytes = [0u8; 32];
    getrandom(&mut bytes).map_err(|_| FnError::new("random unavailable"))?;
    Ok(format!("{}{}", TOKEN_PREFIX, URL_SAFE_NO_PAD.encode(bytes)))
}

fn token_display_prefix(token: &str) -> String {
    token.chars().take(16).collect()
}

fn normalize_client(client: Option<&str>) -> Result<String, FnError> {
    let c = client.unwrap_or("generic").trim().to_ascii_lowercase();
    match c.as_str() {
        "cursor" | "claude-code" | "windsurf" | "generic" => Ok(c),
        _ => Err(FnError::new(
            "client must be cursor, claude-code, windsurf, or generic",
        )),
    }
}

fn normalize_source_client(client: Option<&str>) -> Result<Option<String>, FnError> {
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

fn normalize_user_code_input(code: &str) -> String {
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

fn generate_user_code() -> Result<String, FnError> {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut bytes = [0u8; 8];
    getrandom(&mut bytes).map_err(|_| FnError::new("random unavailable"))?;
    let raw: String = bytes
        .iter()
        .map(|b| CHARSET[(*b as usize) % CHARSET.len()] as char)
        .collect();
    Ok(format!("{}-{}", &raw[..4], &raw[4..]))
}

fn generate_device_code() -> Result<String, FnError> {
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

    sqlx::query(
        r#"
        UPDATE agent_device_sessions
        SET status = 'approved',
            user_id = $2,
            agent_token_id = $3,
            pending_token = $4,
            approved_at = now()
        WHERE id = $1
        "#,
    )
    .bind(session.id)
    .bind(user_id)
    .bind(minted.id)
    .bind(&minted.token)
    .execute(pool)
    .await
    .map_err(|e| FnError::new(format!("device approve failed: {e}")))?;

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
        let _ = sqlx::query(
            "UPDATE agent_device_sessions SET status = 'expired' WHERE device_code_hash = $1 AND status = 'pending'",
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

pub fn agent_session_title(now: DateTime<Utc>) -> String {
    format!("Agent session · {}", now.format("%Y-%m-%d"))
}

fn snap_to_grid(value: i32) -> i32 {
    ((value + BLUEPRINT_GRID / 2) / BLUEPRINT_GRID) * BLUEPRINT_GRID
}

fn next_agent_tool_node_coords(nodes: &Value) -> (i32, i32) {
    let Some(arr) = nodes.as_array() else {
        return (
            snap_to_grid(AGENT_SESSION_START_X),
            snap_to_grid(AGENT_SESSION_START_Y),
        );
    };

    let mut max_bottom = -1;
    let mut anchor_x = AGENT_SESSION_START_X;

    for item in arr {
        if item.get("kind").and_then(|v| v.as_str()) != Some("tool") {
            continue;
        }
        let x = item.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y = item.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let bottom = y + BLUEPRINT_NODE_TOOL_HEIGHT;
        if bottom > max_bottom {
            max_bottom = bottom;
            anchor_x = x;
        }
    }

    if max_bottom < 0 {
        return (
            snap_to_grid(AGENT_SESSION_START_X),
            snap_to_grid(AGENT_SESSION_START_Y),
        );
    }

    (
        snap_to_grid(anchor_x),
        snap_to_grid(max_bottom + AGENT_NODE_STACK_GAP),
    )
}

fn slug_on_canvas(nodes: &Value, slug: &str) -> bool {
    nodes.as_array().is_some_and(|arr| {
        arr.iter().any(|node| {
            node.get("kind").and_then(|v| v.as_str()) == Some("tool")
                && node
                    .get("slug")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s == slug)
        })
    })
}

fn normalize_agent_tool_chains(chains: Option<&[String]>) -> Result<Vec<String>, FnError> {
    let Some(values) = chains else {
        return Ok(Vec::new());
    };
    if values.len() > MAX_TOOL_NODE_CHAINS {
        return Err(FnError::new(format!(
            "at most {MAX_TOOL_NODE_CHAINS} chains per tool node"
        )));
    }
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw in values {
        let chain_id = raw.trim().to_lowercase();
        if chain_id.is_empty() {
            continue;
        }
        if chain_id.chars().count() > MAX_CHAIN_ID_LEN {
            return Err(FnError::new(format!(
                "chain id must be at most {MAX_CHAIN_ID_LEN} characters"
            )));
        }
        if !chain_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(FnError::new("chain id contains invalid characters"));
        }
        if seen.insert(chain_id.clone()) {
            normalized.push(chain_id);
        }
    }
    Ok(normalized)
}

fn initial_tool_node_chains(tool_chains: &[String]) -> Vec<String> {
    if tool_chains.len() == 1 {
        let id = tool_chains[0].trim().to_lowercase();
        if !id.is_empty() {
            return vec![id];
        }
    }
    Vec::new()
}

async fn load_tool_chains(pool: &PgPool, slug: &str) -> Result<Vec<String>, FnError> {
    let chains: Option<Vec<String>> = sqlx::query_scalar(
        r#"
        SELECT chains
        FROM tools
        WHERE slug = $1
          AND approval_status = 'approved'
          AND relevance_status = 'accepted'
          AND quarantined_at IS NULL
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(|e| FnError::new(format!("tool chains lookup failed: {e}")))?;

    Ok(chains.unwrap_or_default())
}

async fn find_or_create_agent_blueprint(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
) -> Result<(Uuid, Value, Value), FnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: Uuid,
        nodes: Value,
        edges: Value,
    }

    if let Some(row) = sqlx::query_as::<_, Row>(
        "SELECT id, nodes, edges FROM blueprints WHERE user_id = $1 AND title = $2 LIMIT 1",
    )
    .bind(user_id)
    .bind(title)
    .fetch_optional(pool)
    .await
    .map_err(|e| FnError::new(format!("blueprint lookup failed: {e}")))?
    {
        return Ok((row.id, row.nodes, row.edges));
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blueprints WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| FnError::new(format!("blueprint count failed: {e}")))?;

    if count >= MAX_BLUEPRINTS_PER_USER {
        return Err(FnError::new(format!(
            "you can save at most {MAX_BLUEPRINTS_PER_USER} blueprints"
        )));
    }

    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO blueprints (user_id, title, nodes, edges)
        VALUES ($1, $2, '[]'::jsonb, '[]'::jsonb)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(title)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("blueprint create failed: {e}")))?;

    Ok((id, json!([]), json!([])))
}

pub async fn sync_blueprint_node(
    pool: &PgPool,
    auth: &AgentAuth,
    req: SyncBlueprintNodeRequest,
) -> Result<SyncBlueprintNodeResponse, FnError> {
    if !auth.scopes.iter().any(|s| s == "blueprint:write") {
        return Err(FnError::new("token missing blueprint:write scope"));
    }

    let slug = req.slug.trim().to_string();
    if slug.is_empty() {
        return Err(FnError::new("slug required"));
    }

    if let Some(key) = req.idempotency_key.as_deref().filter(|k| !k.is_empty()) {
        if let Some(cached) = load_idempotent_blueprint_sync(pool, auth.user_id, key).await? {
            return Ok(cached);
        }
    }

    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM tools
            WHERE slug = $1
              AND approval_status = 'approved'
              AND relevance_status = 'accepted'
              AND quarantined_at IS NULL
        )
        "#,
    )
    .bind(&slug)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("tool lookup failed: {e}")))?;

    if !exists {
        return Err(FnError::new(format!("tool not found: {slug}")));
    }

    let session_title = agent_session_title(Utc::now());
    let (blueprint_id, mut nodes, _edges) = if let Some(id) = req.blueprint_id {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            nodes: Value,
            edges: Value,
        }
        let row = sqlx::query_as::<_, Row>(
            "SELECT id, nodes, edges FROM blueprints WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(auth.user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| FnError::new(format!("blueprint load failed: {e}")))?
        .ok_or_else(|| FnError::new("blueprint not found"))?;
        (row.id, row.nodes, row.edges)
    } else {
        find_or_create_agent_blueprint(pool, auth.user_id, &session_title).await?
    };

    if slug_on_canvas(&nodes, &slug) {
        let updated_at = sqlx::query_scalar::<_, DateTime<Utc>>(
            "SELECT updated_at FROM blueprints WHERE id = $1 AND user_id = $2",
        )
        .bind(blueprint_id)
        .bind(auth.user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| FnError::new(format!("blueprint timestamp failed: {e}")))?;

        let response = SyncBlueprintNodeResponse {
            ok: true,
            slug,
            blueprint_id,
            node_id: None,
            appended: false,
            skip_reason: Some("duplicate_slug".into()),
            updated_at,
        };
        if let Some(key) = req.idempotency_key.as_deref().filter(|k| !k.is_empty()) {
            let _ = record_blueprint_sync_log(pool, auth, &response.slug, key, &response).await;
        }
        return Ok(response);
    }

    let node_count = nodes.as_array().map(|a| a.len()).unwrap_or(0);
    if node_count >= BLUEPRINT_MAX_NODES {
        let updated_at = sqlx::query_scalar::<_, DateTime<Utc>>(
            "SELECT updated_at FROM blueprints WHERE id = $1 AND user_id = $2",
        )
        .bind(blueprint_id)
        .bind(auth.user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| FnError::new(format!("blueprint timestamp failed: {e}")))?;

        let response = SyncBlueprintNodeResponse {
            ok: true,
            slug,
            blueprint_id,
            node_id: None,
            appended: false,
            skip_reason: Some("node_limit".into()),
            updated_at,
        };
        if let Some(key) = req.idempotency_key.as_deref().filter(|k| !k.is_empty()) {
            let _ = record_blueprint_sync_log(pool, auth, &response.slug, key, &response).await;
        }
        return Ok(response);
    }

    let tool_chains = load_tool_chains(pool, &slug).await?;
    let chains = if let Some(requested) = req.chains.as_deref() {
        let normalized = normalize_agent_tool_chains(Some(requested))?;
        if normalized.is_empty() {
            initial_tool_node_chains(&tool_chains)
        } else {
            normalized
        }
    } else {
        initial_tool_node_chains(&tool_chains)
    };

    let (x, y) = next_agent_tool_node_coords(&nodes);
    let node_id = Uuid::new_v4().to_string();
    let mut node = json!({
        "id": node_id,
        "kind": "tool",
        "slug": slug,
        "x": x,
        "y": y,
    });
    if !chains.is_empty() {
        node["chains"] = json!(chains);
    }

    let mut arr = nodes.as_array().cloned().unwrap_or_default();
    arr.push(node);
    nodes = Value::Array(arr);

    let updated_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        r#"
        UPDATE blueprints
        SET nodes = $3
        WHERE id = $1 AND user_id = $2
        RETURNING updated_at
        "#,
    )
    .bind(blueprint_id)
    .bind(auth.user_id)
    .bind(&nodes)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("blueprint append failed: {e}")))?;

    let response = SyncBlueprintNodeResponse {
        ok: true,
        slug: slug.clone(),
        blueprint_id,
        node_id: Some(node_id),
        appended: true,
        skip_reason: None,
        updated_at,
    };

    if let Some(key) = req.idempotency_key.as_deref().filter(|k| !k.is_empty()) {
        let _ = record_blueprint_sync_log(pool, auth, &slug, key, &response).await;
    }

    Ok(response)
}

pub async fn save_stack_to_blueprint(
    pool: &PgPool,
    auth: &AgentAuth,
    slugs: Vec<String>,
    title: Option<String>,
) -> Result<Value, FnError> {
    if slugs.is_empty() {
        return Err(FnError::new("slugs required"));
    }
    if slugs.len() > 25 {
        return Err(FnError::new("at most 25 slugs per stack save"));
    }

    let blueprint_title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| agent_session_title(Utc::now()));

    let (blueprint_id, _, _) =
        find_or_create_agent_blueprint(pool, auth.user_id, &blueprint_title).await?;

    let mut toolkit_results = Vec::new();
    let mut blueprint_results = Vec::new();

    for slug in slugs {
        let slug = slug.trim().to_string();
        if slug.is_empty() {
            continue;
        }
        let toolkit = sync_tool(
            pool,
            auth,
            SyncToolRequest {
                slug: slug.clone(),
                note: None,
                tags: None,
                source_client: Some("mcp".into()),
                idempotency_key: None,
            },
        )
        .await?;
        toolkit_results.push(toolkit);

        let blueprint = sync_blueprint_node(
            pool,
            auth,
            SyncBlueprintNodeRequest {
                blueprint_id: Some(blueprint_id),
                slug,
                chains: None,
                idempotency_key: None,
            },
        )
        .await?;
        blueprint_results.push(blueprint);
    }

    Ok(json!({
        "ok": true,
        "blueprint_id": blueprint_id,
        "title": blueprint_title,
        "toolkit": toolkit_results,
        "blueprint": blueprint_results,
    }))
}

async fn load_idempotent_blueprint_sync(
    pool: &PgPool,
    user_id: Uuid,
    key: &str,
) -> Result<Option<SyncBlueprintNodeResponse>, FnError> {
    let detail: Option<Value> = sqlx::query_scalar(
        r#"
        SELECT detail
        FROM agent_sync_log
        WHERE user_id = $1 AND idempotency_key = $2 AND action = 'sync_blueprint_node' AND status = 'ok'
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

async fn record_blueprint_sync_log(
    pool: &PgPool,
    auth: &AgentAuth,
    slug: &str,
    key: &str,
    response: &SyncBlueprintNodeResponse,
) -> Result<(), FnError> {
    let detail = serde_json::to_value(response)
        .map_err(|e| FnError::new(format!("sync log serialize failed: {e}")))?;
    let _ = sqlx::query(
        r#"
        INSERT INTO agent_sync_log (user_id, agent_token_id, action, tool_slug, blueprint_id, idempotency_key, detail)
        VALUES ($1, $2, 'sync_blueprint_node', $3, $4, $5, $6)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(auth.user_id)
    .bind(auth.token_id)
    .bind(slug)
    .bind(response.blueprint_id)
    .bind(key)
    .bind(detail)
    .execute(pool)
    .await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_token_is_stable_hex() {
        let h = hash_token("oai_ag_test");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn token_prefix_constant() {
        assert!(TOKEN_PREFIX.starts_with("oai_ag_"));
    }

    #[test]
    fn next_agent_tool_node_coords_stacks_below_last_tool() {
        let nodes = json!([
            {"id": "a", "kind": "tool", "slug": "foo", "x": 40, "y": 40},
            {"id": "b", "kind": "note", "text": "hi", "x": 200, "y": 200}
        ]);
        let (x, y) = next_agent_tool_node_coords(&nodes);
        assert_eq!(x, 40);
        assert_eq!(y, 112);
    }

    #[test]
    fn next_agent_tool_node_coords_defaults_when_empty() {
        let (x, y) = next_agent_tool_node_coords(&json!([]));
        assert_eq!(x, 40);
        assert_eq!(y, 40);
    }

    #[test]
    fn slug_on_canvas_detects_tool_slug() {
        let nodes = json!([{"id": "a", "kind": "tool", "slug": "foo", "x": 0, "y": 0}]);
        assert!(slug_on_canvas(&nodes, "foo"));
        assert!(!slug_on_canvas(&nodes, "bar"));
    }
}
