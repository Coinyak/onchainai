//! Review harness models — official links, review runs, entries, operator verdicts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A row from `tool_official_links`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ToolOfficialLink {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub link_type: String,
    pub url: String,
    pub display_label: String,
    pub verification_status: String,
    pub official_badge_allowed: bool,
    pub evidence_strength: String,
    pub verification_method: Option<String>,
    pub discovered_from: Option<String>,
    pub verified_by: Option<Uuid>,
    pub verified_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A row from `review_runs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ReviewRun {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub queue: Option<String>,
    pub runner_name: String,
    pub prompt_version: Option<String>,
    pub snapshot_version: String,
    pub status: String,
    pub summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
}

/// A row from `review_entries`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ReviewEntry {
    pub id: Uuid,
    pub review_run_id: Uuid,
    pub entry_type: String,
    pub role: String,
    pub agent_label: Option<String>,
    pub recommended_action: Option<String>,
    pub confidence: Option<f32>,
    pub rationale: Option<String>,
    pub supporting_evidence_json: serde_json::Value,
    pub dissent_json: serde_json::Value,
    pub missing_proofs_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// A row from `operator_verdicts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct OperatorVerdict {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub review_run_id: Option<Uuid>,
    pub action: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub from_claim_state: Option<String>,
    pub to_claim_state: Option<String>,
    pub reason_codes: Vec<String>,
    pub note: Option<String>,
    pub operator_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Public display label for an official link based on verification status.
pub fn official_link_display_label(link: &ToolOfficialLink) -> String {
    if link.verification_status == "verified" && link.official_badge_allowed {
        match link.link_type.as_str() {
            "github" => "Official GitHub".into(),
            "website" => "Official Website".into(),
            "x" => "Official X".into(),
            _ => link.display_label.clone(),
        }
    } else {
        match link.link_type.as_str() {
            "github" => "GitHub".into(),
            "website" => "Website".into(),
            "x" => "X profile".into(),
            _ => link.display_label.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_link_verification_status_round_trips() {
        let row = ToolOfficialLink {
            id: Uuid::nil(),
            tool_id: Uuid::nil(),
            link_type: "github".into(),
            url: "https://github.com/bob-collective/bob".into(),
            display_label: "Official GitHub".into(),
            verification_status: "verified".into(),
            official_badge_allowed: true,
            evidence_strength: "strong".into(),
            verification_method: Some("site_backlink".into()),
            discovered_from: Some("crawler:npm".into()),
            verified_by: None,
            verified_at: None,
            notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&row).expect("serialize official link");
        let round_trip: ToolOfficialLink =
            serde_json::from_str(&json).expect("deserialize official link");
        assert_eq!(round_trip.link_type, "github");
        assert!(round_trip.official_badge_allowed);
    }

    #[test]
    fn review_entry_preserves_role_and_action() {
        let row = ReviewEntry {
            id: Uuid::nil(),
            review_run_id: Uuid::nil(),
            entry_type: "agent_review".into(),
            role: "critic".into(),
            agent_label: Some("codex-critic-1".into()),
            recommended_action: Some("request_claim_proof".into()),
            confidence: Some(0.74),
            rationale: Some("Official X proof missing".into()),
            supporting_evidence_json: serde_json::json!([{
                "source": "website",
                "detail": "No backlink to X"
            }]),
            dissent_json: serde_json::json!([]),
            missing_proofs_json: serde_json::json!(["site backlink to x.com handle"]),
            created_at: Utc::now(),
        };

        let json = serde_json::to_value(&row).expect("serialize review entry");
        assert_eq!(json["role"], "critic");
        assert_eq!(json["recommended_action"], "request_claim_proof");
    }

    #[test]
    fn official_link_display_label_uses_neutral_when_unverified() {
        let link = ToolOfficialLink {
            id: Uuid::nil(),
            tool_id: Uuid::nil(),
            link_type: "github".into(),
            url: "https://github.com/example/repo".into(),
            display_label: "Official GitHub".into(),
            verification_status: "candidate".into(),
            official_badge_allowed: false,
            evidence_strength: "weak".into(),
            verification_method: None,
            discovered_from: None,
            verified_by: None,
            verified_at: None,
            notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(official_link_display_label(&link), "GitHub");
    }
}
