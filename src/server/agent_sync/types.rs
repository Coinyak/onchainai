//! Shared types and constants for Agent Sync.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const TOKEN_PREFIX: &str = "oai_ag_";
pub const MAX_ACTIVE_TOKENS: i64 = 5;
pub const DEFAULT_TOKEN_TTL_DAYS: i64 = 90;
pub const DEVICE_SESSION_TTL_MINUTES: i64 = 15;
pub const DEVICE_POLL_INTERVAL_SECS: u64 = 5;
pub const LINK_URL: &str = "https://www.onchain-ai.xyz/connect#agent-sync";
pub const MAX_BLUEPRINTS_PER_USER: i64 = 20;
pub const BLUEPRINT_MAX_NODES: usize = 120;
pub const BLUEPRINT_GRID: i32 = 8;
pub const BLUEPRINT_NODE_TOOL_HEIGHT: i32 = 64;
pub const AGENT_SESSION_START_X: i32 = 40;
pub const AGENT_SESSION_START_Y: i32 = 40;
pub const AGENT_NODE_STACK_GAP: i32 = 8;
pub const MAX_TOOL_NODE_CHAINS: usize = 8;
pub const MAX_CHAIN_ID_LEN: usize = 64;

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
