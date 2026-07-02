use super::*;

/// Public trust view for tool detail — facts only, no raw scores.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolTrustView {
    pub tool: Tool,
    pub official_links: Vec<ToolOfficialLink>,
    pub trust_facts: Vec<TrustFact>,
}

/// Operator workbench bundle for a selected tool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminToolWorkbenchView {
    pub tool: Tool,
    pub official_links: Vec<ToolOfficialLink>,
    pub trust: TrustVerificationResult,
    pub timeline: Vec<ReviewEntry>,
    pub verdicts: Vec<OperatorVerdict>,
    pub official_promotion_allowed: bool,
}

/// Workbench summary counts for top promotion rail.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminWorkbenchSummary {
    pub cards: Vec<WorkbenchSummaryCard>,
}

/// Payload to verify an official link independently.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerifyOfficialLinkPayload {
    pub link_id: uuid::Uuid,
    pub verification_status: String,
    pub evidence_strength: String,
    pub official_badge_allowed: bool,
    pub verification_method: Option<String>,
    pub notes: Option<String>,
}
