//! Unit tests for review_tool verdict wiring via operator_review_transition.

use onchainai::models::tool::default_review_fields;
use onchainai::models::{Tool, ToolOfficialLink};
use onchainai::server::functions::review_audit_statuses;
use onchainai::server::operator_review_transition::{
    plan_operator_review, resolve_claim_state_transition, OperatorReviewGate,
};
use onchainai::server::review_persistence::validate_mark_official_gate;
use onchainai::trust_verification::{official_promotion_allowed, verify_tool_trust};

fn sample_tool(status: &str, claim_state: &str, approval_status: &str) -> Tool {
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
        approval_status: approval_status.into(),
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

fn strong_verified_link(link_type: &str) -> ToolOfficialLink {
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
fn mark_official_gate_blocks_before_review_tool_update() {
    let tool = sample_tool("community", "unclaimed", "approved");
    let links = vec![
        strong_verified_link("github"),
        strong_verified_link("website"),
    ];
    assert!(validate_mark_official_gate(&tool, &links).is_err());
}

#[test]
fn mark_official_gate_allows_claimed_tool_with_two_verified_links() {
    let tool = sample_tool("community", "claimed", "approved");
    let links = vec![
        strong_verified_link("github"),
        strong_verified_link("website"),
    ];
    let trust = verify_tool_trust(&tool, &links);
    assert!(official_promotion_allowed(&tool, &links, &trust));
}

#[test]
fn review_tool_effect_matches_audit_statuses_for_mark_official() {
    let tool = sample_tool("verified", "claimed", "approved");
    let (before, after) = review_audit_statuses(&tool, "mark_official");
    let effect = plan_operator_review(&tool, "mark_official", "two verified official links", None);

    assert_eq!(effect.gate, OperatorReviewGate::MarkOfficial);
    assert_eq!(effect.verdict.tool_id, tool.id);
    assert_eq!(effect.verdict.from_status, before);
    assert_eq!(effect.verdict.to_status, after);
    assert_eq!(effect.verdict.from_claim_state, "claimed");
    assert_eq!(
        effect.tool_update.listing_status.as_deref(),
        Some("official")
    );
}

#[test]
fn review_tool_effect_approves_claim_pending_into_claimed() {
    let tool = sample_tool("community", "claim_pending", "pending");
    let effect = plan_operator_review(&tool, "approved", "claim proof accepted", None);

    assert_eq!(
        resolve_claim_state_transition(&tool, "approved").as_deref(),
        Some("claimed")
    );
    assert_eq!(effect.tool_update.claim_state.as_deref(), Some("claimed"));
    assert_eq!(effect.verdict.to_claim_state.as_deref(), Some("claimed"));
    assert_eq!(effect.gate, OperatorReviewGate::PublicationApproval);
}

#[test]
fn review_tool_effect_captures_quarantine_audit_trail() {
    let tool = sample_tool("community", "unclaimed", "approved");
    let (before, after) = review_audit_statuses(&tool, "quarantine");
    let effect = plan_operator_review(&tool, "quarantine", "curl pipe bash install", None);

    assert_eq!(before.as_str(), "active");
    assert_eq!(after.as_str(), "quarantined");
    assert_eq!(effect.verdict.to_status, "quarantined");
    assert!(effect.tool_update.quarantine);
}
