//! Pure operator review transitions — claim state, listing status, gates, verdict fields.

use crate::models::Tool;
use crate::server::review_persistence::InsertOperatorVerdictInput;
use crate::trust_verification::identity_cluster_aligned;

/// Gate enforced in `review_tool` before applying the effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorReviewGate {
    None,
    PublicationApproval,
    MarkOfficial,
    DemoteVerified,
    DemoteOfficial,
}

/// SQL fields to update on `tools` for one operator action.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolReviewSqlUpdate {
    pub approval_status: Option<String>,
    pub rejection_reason: Option<Option<String>>,
    pub relevance_status: Option<String>,
    pub listing_status: Option<String>,
    pub quarantine: bool,
    pub claim_state: Option<String>,
    pub touch_last_reviewed: bool,
}

/// Full effect of an operator review action (pure plan).
#[derive(Debug, Clone, PartialEq)]
pub struct OperatorReviewEffect {
    pub gate: OperatorReviewGate,
    pub tool_update: ToolReviewSqlUpdate,
    pub verdict: InsertOperatorVerdictInput,
    pub legacy_audit_before: String,
    pub legacy_audit_after: String,
}

/// Override reason is required when approving despite rejected relevance or critical install risk.
pub fn review_override_required(tool: &Tool) -> bool {
    tool.relevance_status == "rejected" || tool.install_risk_level == "critical"
}

fn tool_has_trustworthy_url(tool: &Tool) -> bool {
    let valid_url = |value: &Option<String>| {
        value.as_ref().is_some_and(|u| {
            let trimmed = u.trim();
            trimmed.starts_with("https://") || trimmed.starts_with("http://")
        })
    };
    valid_url(&tool.repo_url)
        || valid_url(&tool.homepage)
        || tool
            .npm_package
            .as_ref()
            .is_some_and(|p| !p.trim().is_empty())
        || valid_url(&tool.mcp_endpoint)
}

/// Validate approval gates without touching the database.
pub fn validate_review_approval_gate(
    tool: &Tool,
    override_reason: Option<&str>,
) -> Result<(), &'static str> {
    if !tool_has_trustworthy_url(tool) {
        return Err("approval requires a repo, homepage, npm package, or MCP endpoint");
    }
    if review_override_required(tool) {
        if override_reason.map(str::trim).is_none_or(str::is_empty) {
            return Err(
                "override reason required when overriding rejected relevance or critical install risk",
            );
        }
        return Ok(());
    }
    if tool.relevance_status == "needs_review" {
        return Ok(());
    }
    if tool.relevance_status != "accepted" {
        return Err("approval requires relevance status accepted");
    }
    Ok(())
}

/// Gate for demoting a verified listing back to community.
pub fn validate_demote_verified_gate(tool: &Tool) -> Result<(), &'static str> {
    if tool.status == "verified" {
        Ok(())
    } else {
        Err("demote_verified requires listing status verified")
    }
}

/// Gate for demoting an official listing back to community.
pub fn validate_demote_official_gate(tool: &Tool) -> Result<(), &'static str> {
    if tool.status == "official" {
        Ok(())
    } else {
        Err("demote_official requires listing status official")
    }
}

/// Audit before/after labels for legacy `tool_review_events`.
pub fn review_audit_statuses(tool: &Tool, action: &str) -> (String, String) {
    match action {
        "mark_verified" => (tool.status.clone(), "verified".into()),
        "mark_official" => (tool.status.clone(), "official".into()),
        "demote_verified" | "demote_official" => (tool.status.clone(), "community".into()),
        "quarantine" => (
            if tool.quarantined_at.is_some() {
                "quarantined".into()
            } else {
                "active".into()
            },
            "quarantined".into(),
        ),
        "needs_info" => (tool.approval_status.clone(), "needs_info".into()),
        other => (tool.approval_status.clone(), other.to_string()),
    }
}

/// Claim lifecycle transitions encoded by operator actions.
pub fn resolve_claim_state_transition(tool: &Tool, action: &str) -> Option<String> {
    match (action, tool.claim_state.as_str()) {
        ("approved", "claim_pending") => Some("claimed".into()),
        ("rejected", "claim_pending") => Some("unclaimed".into()),
        _ => None,
    }
}

fn relevance_status_on_approval(tool: &Tool, action: &str) -> Option<String> {
    if action == "approved"
        && (tool.relevance_status == "needs_review" || review_override_required(tool))
    {
        Some("accepted".into())
    } else {
        None
    }
}

fn tool_update_for_action(tool: &Tool, action: &str, reason: &str) -> ToolReviewSqlUpdate {
    let claim_state = resolve_claim_state_transition(tool, action);
    match action {
        "approved" | "rejected" | "pending" => ToolReviewSqlUpdate {
            approval_status: Some(action.into()),
            rejection_reason: if action == "rejected" {
                Some(Some(reason.trim().to_string()))
            } else {
                Some(None)
            },
            relevance_status: relevance_status_on_approval(tool, action),
            listing_status: None,
            quarantine: false,
            claim_state,
            touch_last_reviewed: true,
        },
        "needs_info" => ToolReviewSqlUpdate {
            approval_status: Some("pending".into()),
            rejection_reason: Some(None),
            relevance_status: None,
            listing_status: None,
            quarantine: false,
            claim_state: None,
            touch_last_reviewed: true,
        },
        "quarantine" => ToolReviewSqlUpdate {
            approval_status: None,
            rejection_reason: None,
            relevance_status: None,
            listing_status: None,
            quarantine: true,
            claim_state: None,
            touch_last_reviewed: true,
        },
        "mark_verified" => ToolReviewSqlUpdate {
            approval_status: None,
            rejection_reason: None,
            relevance_status: None,
            listing_status: Some("verified".into()),
            quarantine: false,
            claim_state: None,
            touch_last_reviewed: true,
        },
        "mark_official" => ToolReviewSqlUpdate {
            approval_status: None,
            rejection_reason: None,
            relevance_status: None,
            listing_status: Some("official".into()),
            quarantine: false,
            claim_state: None,
            touch_last_reviewed: true,
        },
        "demote_verified" | "demote_official" => ToolReviewSqlUpdate {
            approval_status: None,
            rejection_reason: None,
            relevance_status: None,
            listing_status: Some("community".into()),
            quarantine: false,
            claim_state: None,
            touch_last_reviewed: true,
        },
        _ => ToolReviewSqlUpdate {
            approval_status: None,
            rejection_reason: None,
            relevance_status: None,
            listing_status: None,
            quarantine: false,
            claim_state: None,
            touch_last_reviewed: false,
        },
    }
}

fn gate_for_action(action: &str) -> OperatorReviewGate {
    match action {
        "approved" => OperatorReviewGate::PublicationApproval,
        "mark_official" => OperatorReviewGate::MarkOfficial,
        "demote_verified" => OperatorReviewGate::DemoteVerified,
        "demote_official" => OperatorReviewGate::DemoteOfficial,
        _ => OperatorReviewGate::None,
    }
}

/// Plan one operator review action from the current tool row.
pub fn plan_operator_review(
    tool: &Tool,
    action: &str,
    reason: &str,
    harness_run_id: Option<uuid::Uuid>,
) -> OperatorReviewEffect {
    let (legacy_audit_before, legacy_audit_after) = review_audit_statuses(tool, action);
    let to_claim_state = resolve_claim_state_transition(tool, action);
    let tool_update = tool_update_for_action(tool, action, reason);

    OperatorReviewEffect {
        gate: gate_for_action(action),
        tool_update,
        verdict: InsertOperatorVerdictInput {
            tool_id: tool.id,
            review_run_id: harness_run_id,
            action: action.into(),
            from_status: legacy_audit_before.clone(),
            to_status: legacy_audit_after.clone(),
            from_claim_state: tool.claim_state.clone(),
            to_claim_state: to_claim_state.clone(),
            reason_codes: vec![],
            note: Some(reason.trim().to_string()),
        },
        legacy_audit_before,
        legacy_audit_after,
    }
}

// ── 3-tier auto-approval model ──

/// Auto-approval tier assigned when a tool passes automated gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoApprovalTier {
    /// Not eligible for auto-approval — needs human review.
    Manual,
    /// Auto-approved with no badge (safe + relevant, but unverified).
    Community,
    /// Auto-approved with verified badge (identity aligned + prior publisher approvals).
    Verified,
    // Official is never auto-assigned — always requires human action.
}

impl AutoApprovalTier {
    /// True when this tier represents an auto-actionable approval.
    pub fn is_auto_actionable(self) -> bool {
        matches!(self, Self::Community | Self::Verified)
    }
}

/// Result of evaluating a tool against auto-approval gates.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoApprovalResult {
    pub tier: AutoApprovalTier,
    pub reason: String,
}

/// Evaluate a pending tool against 3-tier auto-approval gates.
///
/// - `prior_approvals_by_publisher` = count of already-approved tools from the same
///   identity cluster (GitHub org / npm scope / homepage domain).  0 means new publisher.
///
/// Returns `Manual` when any blocking condition is hit (critical risk, rejected
/// relevance, no trustworthy URL).  Returns `Community` when safe but the publisher is
/// new or identity is not aligned.  Returns `Verified` only when identity is aligned
/// **and** the publisher already has at least one prior approval.
pub fn evaluate_auto_approval(
    tool: &Tool,
    prior_approvals_by_publisher: i64,
) -> AutoApprovalResult {
    if tool.install_risk_level == "critical" {
        return AutoApprovalResult {
            tier: AutoApprovalTier::Manual,
            reason: "critical install risk requires manual review".into(),
        };
    }
    if tool.relevance_status == "rejected" {
        return AutoApprovalResult {
            tier: AutoApprovalTier::Manual,
            reason: "rejected crypto relevance requires manual review".into(),
        };
    }
    if tool.relevance_status == "needs_review" {
        return AutoApprovalResult {
            tier: AutoApprovalTier::Manual,
            reason: "relevance needs human review before auto-approval".into(),
        };
    }
    if !tool_has_trustworthy_url(tool) {
        return AutoApprovalResult {
            tier: AutoApprovalTier::Manual,
            reason: "no trustworthy URL (repo, homepage, npm, or MCP endpoint)".into(),
        };
    }

    // Base gate passed — at minimum this tool can be auto-approved as community.
    if prior_approvals_by_publisher > 0 && identity_aligned(tool) {
        return AutoApprovalResult {
            tier: AutoApprovalTier::Verified,
            reason: "identity aligned and publisher has prior approvals".into(),
        };
    }

    AutoApprovalResult {
        tier: AutoApprovalTier::Community,
        reason: "safe and relevant but no prior publisher approvals or identity not aligned".into(),
    }
}

/// Check whether GitHub org, npm scope, and homepage domain refer to the same identity.
fn identity_aligned(tool: &Tool) -> bool {
    let repo = tool.repo_url.as_deref().unwrap_or("");
    let homepage = tool.homepage.as_deref().unwrap_or("");
    let npm = tool.npm_package.as_deref().unwrap_or("");
    identity_cluster_aligned(repo, homepage, npm)
}

/// Build the `OperatorReviewEffect` for an auto-approval action.
///
/// `tier` must be `Community` or `Verified` (never `Manual` or `Official`).
pub fn plan_auto_approval(
    tool: &Tool,
    result: &AutoApprovalResult,
) -> Option<OperatorReviewEffect> {
    let listing_status = match result.tier {
        AutoApprovalTier::Manual => return None,
        AutoApprovalTier::Community => "community",
        AutoApprovalTier::Verified => "verified",
    };

    let tool_update = ToolReviewSqlUpdate {
        approval_status: Some("approved".into()),
        rejection_reason: Some(None),
        relevance_status: None,
        listing_status: Some(listing_status.into()),
        quarantine: false,
        claim_state: None,
        touch_last_reviewed: true,
    };

    Some(OperatorReviewEffect {
        gate: OperatorReviewGate::None,
        tool_update,
        verdict: InsertOperatorVerdictInput {
            tool_id: tool.id,
            review_run_id: None,
            action: "auto_approved".into(),
            from_status: tool.status.clone(),
            to_status: listing_status.into(),
            from_claim_state: tool.claim_state.clone(),
            to_claim_state: None,
            reason_codes: vec![],
            note: Some(result.reason.clone()),
        },
        legacy_audit_before: tool.approval_status.clone(),
        legacy_audit_after: "approved".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::default_review_fields;
    use crate::models::ToolOfficialLink;
    use crate::trust_verification::{official_promotion_allowed, verify_tool_trust};

    fn sample_tool(claim_state: &str, status: &str, approval: &str) -> Tool {
        let review = default_review_fields();
        Tool {
            id: uuid::Uuid::new_v4(),
            name: "Demo".into(),
            slug: "demo".into(),
            description: None,
            function: "dev-tool".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "mcp".into(),
            repo_url: Some("https://github.com/org/repo".into()),
            homepage: Some("https://example.com".into()),
            npm_package: Some("@org/pkg".into()),
            install_command: Some("npx @org/pkg".into()),
            mcp_endpoint: None,
            chains: vec![],
            status: status.into(),
            official_team: None,
            trust_score: 0,
            approval_status: approval.into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: 80,
            crypto_relevance_reasons: vec![],
            relevance_status: "accepted".into(),
            install_risk_level: "low".into(),
            install_risk_reasons: vec![],
            requires_secret: false,
            safe_copy_command: None,
            quarantined_at: None,
            last_reviewed_at: None,
            review_policy_version: review.review_policy_version,
            claim_state: claim_state.into(),
            license: None,
            pricing: "free".into(),
            x402_price: None,
            referral_enabled: false,
            referral_bps: None,
            referral_payout_address: None,
            referral_model: None,
            x402_pay_to_address: None,
            x402_builder_code: None,
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
            x402_endpoint: None,
            x402_last_checked_at: None,
            x402_check_failures: 0,
            stars: 0,
            last_commit_at: Some(chrono::Utc::now()),
            source: "manual".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn strong_link(link_type: &str) -> ToolOfficialLink {
        ToolOfficialLink {
            id: uuid::Uuid::new_v4(),
            tool_id: uuid::Uuid::nil(),
            link_type: link_type.into(),
            url: "https://example.com".into(),
            display_label: "Official".into(),
            verification_status: "verified".into(),
            official_badge_allowed: true,
            evidence_strength: "strong".into(),
            verification_method: Some("operator_review".into()),
            discovered_from: None,
            verified_by: None,
            verified_at: None,
            notes: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn approved_claim_pending_transitions_to_claimed() {
        let tool = sample_tool("claim_pending", "community", "pending");
        let effect = plan_operator_review(&tool, "approved", "claim proof verified", None);
        assert_eq!(effect.tool_update.claim_state.as_deref(), Some("claimed"));
        assert_eq!(effect.verdict.to_claim_state.as_deref(), Some("claimed"));
        assert_eq!(effect.verdict.from_claim_state, "claim_pending");
        assert_eq!(effect.gate, OperatorReviewGate::PublicationApproval);
    }

    #[test]
    fn claim_pending_to_claimed_to_mark_official_chain() {
        let pending = sample_tool("claim_pending", "community", "pending");
        let approve = plan_operator_review(&pending, "approved", "proof ok", None);
        assert_eq!(approve.verdict.to_claim_state.as_deref(), Some("claimed"));

        let mut claimed = pending.clone();
        claimed.claim_state = "claimed".into();
        claimed.approval_status = "approved".into();
        let links = vec![strong_link("github"), strong_link("website")];
        let trust = verify_tool_trust(&claimed, &links);
        assert!(official_promotion_allowed(&claimed, &links, &trust));

        let official = plan_operator_review(&claimed, "mark_official", "two verified links", None);
        assert_eq!(official.gate, OperatorReviewGate::MarkOfficial);
        assert_eq!(
            official.tool_update.listing_status.as_deref(),
            Some("official")
        );
        assert!(official.tool_update.claim_state.is_none());
    }

    #[test]
    fn mark_official_gate_key_without_claim_transition() {
        let tool = sample_tool("unclaimed", "verified", "approved");
        let effect = plan_operator_review(&tool, "mark_official", "not allowed yet", None);
        assert_eq!(effect.gate, OperatorReviewGate::MarkOfficial);
        assert!(effect.tool_update.claim_state.is_none());
    }

    #[test]
    fn rejected_claim_pending_resets_to_unclaimed() {
        let tool = sample_tool("claim_pending", "community", "pending");
        let effect = plan_operator_review(&tool, "rejected", "proof insufficient", None);
        assert_eq!(effect.tool_update.claim_state.as_deref(), Some("unclaimed"));
        assert_eq!(effect.verdict.to_claim_state.as_deref(), Some("unclaimed"));
    }

    #[test]
    fn approval_gate_rejects_non_http_urls() {
        let mut tool = sample_tool("unclaimed", "community", "pending");
        tool.repo_url = Some("javascript:alert(1)".into());
        tool.homepage = None;
        tool.npm_package = None;
        assert_eq!(
            validate_review_approval_gate(&tool, None),
            Err("approval requires a repo, homepage, npm package, or MCP endpoint")
        );
    }

    #[test]
    fn mark_verified_updates_listing_status_only() {
        let tool = sample_tool("unclaimed", "community", "approved");
        let effect = plan_operator_review(&tool, "mark_verified", "repo checked", None);
        assert_eq!(
            effect.tool_update.listing_status.as_deref(),
            Some("verified")
        );
        assert_eq!(effect.gate, OperatorReviewGate::None);
        assert_eq!(effect.legacy_audit_before, "community");
        assert_eq!(effect.legacy_audit_after, "verified");
    }

    #[test]
    fn demote_verified_updates_listing_status_to_community() {
        let tool = sample_tool("claimed", "verified", "approved");
        let effect = plan_operator_review(&tool, "demote_verified", "trust revoked", None);
        assert_eq!(
            effect.tool_update.listing_status.as_deref(),
            Some("community")
        );
        assert_eq!(effect.gate, OperatorReviewGate::DemoteVerified);
        assert_eq!(effect.legacy_audit_before, "verified");
        assert_eq!(effect.legacy_audit_after, "community");
    }

    #[test]
    fn demote_official_updates_listing_status_to_community() {
        let tool = sample_tool("claimed", "official", "approved");
        let effect = plan_operator_review(&tool, "demote_official", "badge revoked", None);
        assert_eq!(
            effect.tool_update.listing_status.as_deref(),
            Some("community")
        );
        assert_eq!(effect.gate, OperatorReviewGate::DemoteOfficial);
        assert_eq!(effect.legacy_audit_before, "official");
        assert_eq!(effect.legacy_audit_after, "community");
    }

    #[test]
    fn demote_gates_require_matching_listing_status() {
        let verified = sample_tool("claimed", "verified", "approved");
        assert!(validate_demote_verified_gate(&verified).is_ok());
        assert!(validate_demote_official_gate(&verified).is_err());

        let official = sample_tool("claimed", "official", "approved");
        assert!(validate_demote_official_gate(&official).is_ok());
        assert!(validate_demote_verified_gate(&official).is_err());

        let community = sample_tool("unclaimed", "community", "approved");
        assert!(validate_demote_verified_gate(&community).is_err());
        assert!(validate_demote_official_gate(&community).is_err());
    }

    // ── 3-tier auto-approval tests ──

    #[test]
    fn auto_approval_community_passes_safe_tool() {
        let tool = sample_tool("unclaimed", "community", "pending");
        let result = evaluate_auto_approval(&tool, 0);
        assert_eq!(result.tier, AutoApprovalTier::Community);
    }

    #[test]
    fn auto_approval_rejects_critical_install_risk() {
        let mut tool = sample_tool("unclaimed", "community", "pending");
        tool.install_risk_level = "critical".into();
        let result = evaluate_auto_approval(&tool, 0);
        assert_eq!(result.tier, AutoApprovalTier::Manual);
        assert!(result.reason.contains("critical"));
    }

    #[test]
    fn auto_approval_rejects_rejected_relevance() {
        let mut tool = sample_tool("unclaimed", "community", "pending");
        tool.relevance_status = "rejected".into();
        let result = evaluate_auto_approval(&tool, 0);
        assert_eq!(result.tier, AutoApprovalTier::Manual);
        assert!(result.reason.contains("relevance"));
    }

    #[test]
    fn auto_approval_rejects_needs_review_relevance() {
        let mut tool = sample_tool("unclaimed", "community", "pending");
        tool.relevance_status = "needs_review".into();
        let result = evaluate_auto_approval(&tool, 0);
        assert_eq!(result.tier, AutoApprovalTier::Manual);
        assert!(result.reason.contains("human review"));
    }

    #[test]
    fn auto_approval_rejects_no_trustworthy_url() {
        let mut tool = sample_tool("unclaimed", "community", "pending");
        tool.repo_url = None;
        tool.homepage = None;
        tool.npm_package = None;
        tool.mcp_endpoint = None;
        let result = evaluate_auto_approval(&tool, 0);
        assert_eq!(result.tier, AutoApprovalTier::Manual);
        assert!(result.reason.contains("trustworthy"));
    }

    #[test]
    fn auto_approval_verified_requires_identity_and_prior_approval() {
        let mut tool = sample_tool("unclaimed", "community", "pending");
        tool.homepage = Some("https://org.dev".into());
        let result = evaluate_auto_approval(&tool, 1);
        assert_eq!(result.tier, AutoApprovalTier::Verified);
    }

    #[test]
    fn auto_approval_verified_blocked_without_prior_approvals() {
        let mut tool = sample_tool("unclaimed", "community", "pending");
        tool.homepage = Some("https://org.dev".into());
        let result = evaluate_auto_approval(&tool, 0);
        assert_eq!(result.tier, AutoApprovalTier::Community);
    }

    #[test]
    fn auto_approval_verified_blocked_when_identity_not_aligned() {
        let mut tool = sample_tool("unclaimed", "community", "pending");
        tool.repo_url = Some("https://github.com/org-a/repo".into());
        tool.homepage = Some("https://different-site.com".into());
        tool.npm_package = Some("@other-scope/pkg".into());
        let result = evaluate_auto_approval(&tool, 5);
        assert_eq!(result.tier, AutoApprovalTier::Community);
    }

    #[test]
    fn auto_approval_never_returns_official() {
        let mut tool = sample_tool("unclaimed", "community", "pending");
        tool.homepage = Some("https://org.dev".into());
        let result = evaluate_auto_approval(&tool, 100);
        assert_eq!(result.tier, AutoApprovalTier::Verified);
    }
}
