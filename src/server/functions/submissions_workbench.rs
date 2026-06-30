use super::*;

/// Scanned intake metadata attached to a submission row.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmissionScanResult {
    pub crypto_relevance_score: i32,
    pub relevance_status: String,
    pub install_risk_level: String,
}

const SUBMIT_TOOL_TYPES: &[&str] = &["mcp", "cli", "sdk", "api", "skill", "x402"];
const SUBMIT_FUNCTIONS: &[&str] = &[
    "bridge",
    "swap",
    "wallet",
    "payments",
    "lending",
    "staking",
    "trading",
    "nft",
    "data",
    "dev-tool",
    "identity",
    "governance",
    "social",
    "ai-agent",
];

mod reports_claims;
mod submission_intake;
mod workbench;

pub use reports_claims::*;
pub use submission_intake::*;
pub use workbench::*;
