//! Normalizer — source data → [`Tool`] normalization + 3-axis classification.
//!
//! See `docs/MVP_DESIGN.md` section 3 for keyword rules. Classification logic
//! is implemented in a later milestone; this module exposes the trait shape so
//! the crawler compiles.

use serde::{Deserialize, Serialize};

/// Raw tool as produced by a source crawler, before normalization.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTool {
    pub name: String,
    pub description: Option<String>,
    pub tool_type: String,
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
    pub npm_package: Option<String>,
    pub install_command: Option<String>,
    pub mcp_endpoint: Option<String>,
    pub chains: Vec<String>,
    pub stars: i32,
    pub last_commit_at: Option<chrono::DateTime<chrono::Utc>>,
    pub source: String,
    pub source_url: Option<String>,
    pub license: Option<String>,
}

/// Classify the `function` axis from text. Default: `dev-tool`.
#[allow(dead_code)]
pub fn classify_function(_text: &str) -> &'static str {
    "dev-tool"
}

/// Classify the `asset_class` axis. Default: `crypto`.
#[allow(dead_code)]
pub fn classify_asset_class(_text: &str) -> &'static str {
    "crypto"
}

/// Classify the `actor` axis. Default: `human`.
#[allow(dead_code)]
pub fn classify_actor(_text: &str) -> &'static str {
    "human"
}
