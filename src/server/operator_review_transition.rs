//! Pure operator review transitions — claim state, listing status, gates, verdict fields.

use crate::models::Tool;
use crate::server::review_persistence::InsertOperatorVerdictInput;

/// Gate enforced in `review_tool` before applying the effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorReviewGate {
    None,
    PublicationApproval,
    MarkOfficial,
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
    fn valid_url(url: &Option<String>) -> bool {
        url.as_ref().is_some_and(|u| !u.trim().is_empty())
    }
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

/// Audit before/after labels for legacy `tool_review_events`.
pub fn review_audit_statuses(tool: &Tool, action: &str) -> (String, String) {
    match action {
        "mark_verified" => (tool.status.clone(), "verified".into()),
        "mark_official" => (tool.status.clone(), "official".into()),
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
}
