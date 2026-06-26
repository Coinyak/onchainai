//! Submission, report, and claim models — operator intake tables.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A row from `tool_submissions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ToolSubmission {
    pub id: Uuid,
    pub submitted_by: Option<Uuid>,
    pub status: String,
    pub payload: serde_json::Value,
    pub crypto_relevance_score: i32,
    pub relevance_status: String,
    pub install_risk_level: String,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User-submitted tool suggestion stored in `tool_submissions.payload`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSubmissionPayload {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: String,
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
    pub npm_package: Option<String>,
    pub mcp_endpoint: Option<String>,
    pub install_command: Option<String>,
    pub chains: Vec<String>,
    pub category_suggestion: Option<String>,
    pub official_team_claim: bool,
    pub verification_note: Option<String>,
    pub slug: String,
}

/// A row from `tool_reports`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ToolReport {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub reported_by: Option<Uuid>,
    pub reason: String,
    pub details: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// A row from `tool_claim_requests`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ToolClaimRequest {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub requested_by: Uuid,
    pub verification_note: String,
    pub contact_email: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Valid report reasons (matches DB constraint).
pub const TOOL_REPORT_REASONS: &[(&str, &str)] = &[
    ("scam_phishing", "Scam or phishing"),
    ("unsafe_install", "Unsafe install command"),
    ("wrong_category", "Wrong category"),
    ("not_crypto_related", "Not crypto-related"),
    ("broken_link", "Broken link"),
    ("duplicate_listing", "Duplicate listing"),
];

/// Valid claim states on `tools.claim_state`.
pub const CLAIM_STATES: &[&str] = &[
    "unclaimed",
    "claim_pending",
    "claimed",
    "disputed",
    "revoked",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submission_payload_serializes_type_field() {
        let payload = ToolSubmissionPayload {
            name: "Bridge MCP".into(),
            description: "Ethereum bridge MCP server for agents.".into(),
            tool_type: "mcp".into(),
            function: "bridge".into(),
            repo_url: Some("https://github.com/example/bridge".into()),
            homepage: None,
            npm_package: None,
            mcp_endpoint: None,
            install_command: Some("npm i @example/bridge-mcp".into()),
            chains: vec!["ethereum".into()],
            category_suggestion: None,
            official_team_claim: false,
            verification_note: None,
            slug: "bridge-mcp".into(),
        };
        let json = serde_json::to_value(&payload).expect("serialize");
        assert_eq!(json["type"], "mcp");
        assert_eq!(json["slug"], "bridge-mcp");
    }

    #[test]
    fn report_reasons_cover_all_db_values() {
        assert_eq!(TOOL_REPORT_REASONS.len(), 6);
        assert!(TOOL_REPORT_REASONS
            .iter()
            .any(|(k, _)| *k == "scam_phishing"));
    }
}
