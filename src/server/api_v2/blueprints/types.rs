//! Blueprint API shared types and limits.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::super::error::ApiError;

pub const MAX_BLUEPRINTS_PER_USER: i64 = 20;
pub const MAX_NODES: usize = 120;
pub const MAX_EDGES: usize = 120;
pub const COORD_MAX: i32 = 4000;
pub const MAX_NOTE_TEXT: usize = 2000;
pub const MAX_TITLE_LEN: usize = 200;
pub const MAX_CHAIN_ID_LEN: usize = 64;
pub const MAX_TOOL_NODE_CHAINS: usize = 8;
pub const MAX_EDGE_LABEL_LEN: usize = 40;
pub const NODE_MIN_W: i32 = 160;
pub const NODE_MAX_W: i32 = 520;
pub const NODE_MIN_H: i32 = 72;
pub const NODE_MAX_H: i32 = 420;
pub const NODE_MAX_STEP: i32 = 99;
pub const NODE_MAX_STEPS_PER_NODE: usize = 8;
pub const AGENT_EXPORT_FILENAME: &str = "blueprint-agent.md";

pub const AGENT_EXPORT_TASK_TEMPLATE: &str = r#"## Your task

1. Read the attached blueprint PNG together with this prompt (export PNG separately from the editor Share dock).
2. For each slug in ## Tools, call OnchainAI MCP `get_install_guide` (platform: {platform}).
3. Summarize install risk; do not install critical-risk tools.
4. When ## Order is present, treat it as the owner's step sequence; otherwise follow ## Flow. If you edited Flow/Order, prefer the user's wording.
5. Ask before changing my toolkit or installing anything."#;

pub fn db_internal(action: &str, err: impl std::fmt::Display) -> ApiError {
    tracing::error!("blueprint {action} failed: {err}");
    ApiError::Internal(format!("blueprint {action} failed"))
}
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BlueprintRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub nodes: Value,
    pub edges: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BlueprintListRow {
    pub id: Uuid,
    pub title: String,
    pub node_count: i32,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct BlueprintView {
    pub id: Uuid,
    pub title: String,
    pub nodes: Value,
    pub edges: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBlueprintBody {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub nodes: Option<Value>,
    #[serde(default)]
    pub edges: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBlueprintBody {
    pub title: Option<String>,
    pub nodes: Option<Value>,
    pub edges: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct AgentExportResponse {
    pub title: String,
    pub markdown: String,
    pub slugs: Vec<String>,
    pub filename: String,
}

#[derive(Debug, Clone)]
pub struct ExportNode {
    pub id: String,
    pub kind: String,
    pub slug: Option<String>,
    pub chain_id: Option<String>,
    pub text: Option<String>,
    pub chains: Vec<String>,
    pub steps: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct ToolExportMeta {
    pub name: String,
    pub install_risk_level: String,
}

#[derive(Debug, Deserialize)]
pub struct BlueprintNodeInput {
    pub id: String,
    pub kind: String,
    pub slug: Option<String>,
    #[serde(rename = "chainId")]
    pub chain_id: Option<String>,
    pub text: Option<String>,
    #[serde(default)]
    pub chains: Option<Vec<String>>,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub w: Option<i32>,
    #[serde(default)]
    pub h: Option<i32>,
    #[serde(default)]
    pub step: Option<i32>,
    #[serde(default)]
    pub steps: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
pub struct BlueprintEdgeInput {
    pub id: String,
    #[serde(rename = "fromId")]
    pub from_id: String,
    #[serde(rename = "toId")]
    pub to_id: String,
    pub style: String,
    pub color: String,
    #[serde(default)]
    pub dashed: Option<bool>,
    #[serde(default)]
    pub label: Option<String>,
}
