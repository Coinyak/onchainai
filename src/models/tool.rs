//! Tool model — maps the `tools` table.
//!
//! The DB column is named `type` (a Rust keyword), so the struct field is
//! `tool_type` with `#[sqlx(rename = "type")]` to map correctly.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A crypto tool row from the `tools` table.
///
/// See `migrations/001_init.sql` for the full column list. All fields match
/// the DB schema exactly; the `type` column is renamed to `tool_type` because
/// `type` is a reserved Rust keyword.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Tool {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,

    // Classification (3-axis + type)
    pub function: String,
    pub asset_class: String,
    pub actor: String,
    /// Maps to the DB column `type`. Renamed because `type` is a Rust keyword.
    #[cfg_attr(feature = "ssr", sqlx(rename = "type"))]
    #[serde(rename = "type")]
    pub tool_type: String,

    // Connections
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
    pub npm_package: Option<String>,
    pub install_command: Option<String>,
    pub mcp_endpoint: Option<String>,

    // Chain support
    pub chains: Vec<String>,

    // Trust
    pub status: String,
    pub official_team: Option<String>,
    pub trust_score: i32,

    // Approval (admin panel)
    pub approval_status: String,
    pub submitted_by: Option<Uuid>,
    pub rejection_reason: Option<String>,

    // Meta
    pub license: Option<String>,
    pub pricing: String,
    pub x402_price: Option<String>,
    pub stars: i32,
    pub last_commit_at: Option<DateTime<Utc>>,

    // Source
    pub source: String,
    pub source_url: Option<String>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_serde_renames_type_field() {
        // The JSON key must be "type" so API/MCP responses match the DB column
        // name expected by clients and agents.
        let tool = Tool {
            id: Uuid::nil(),
            name: "Test".into(),
            slug: "test".into(),
            description: None,
            function: "dev-tool".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "mcp".into(),
            repo_url: None,
            homepage: None,
            npm_package: None,
            install_command: None,
            mcp_endpoint: None,
            chains: vec![],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "approved".into(),
            submitted_by: None,
            rejection_reason: None,
            license: None,
            pricing: "free".into(),
            x402_price: None,
            stars: 0,
            last_commit_at: None,
            source: "manual".into(),
            source_url: None,
            created_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let json = serde_json::to_string(&tool).expect("serialize tool");
        assert!(
            json.contains("\"type\":\"mcp\""),
            "serde should emit the `type` key, got: {json}"
        );
        assert!(
            !json.contains("tool_type"),
            "serde should NOT emit `tool_type`, got: {json}"
        );

        // Round-trip back into the struct.
        let back: Tool = serde_json::from_str(&json).expect("deserialize tool");
        assert_eq!(back.tool_type, "mcp");
    }
}
