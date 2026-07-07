//! W8: Stale Trust Badge (free discovery) + Probe Receipt (paid K2) + K1 attribution anchor.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::tool::PublicTool;
use crate::models::Tool;
use crate::server::x402_verify::{price_matches_advertised, ProbeOutcome};

/// MCP/REST tool detail — PublicTool fields at root plus optional W8 trust_probe meta.
#[derive(Debug, Clone, Serialize)]
pub struct PublicToolWithTrustProbe {
    #[serde(flatten)]
    pub tool: PublicTool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_probe: Option<StaleTrustBadge>,
}

pub const STALE_THRESHOLD_HOURS: i64 = 24;
pub const PROBE_COST_USD: &str = "0.001";
pub const ESTIMATED_DEAD_CALL_LOSS_USD: &str = "10";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkipCostExposure {
    pub probe_cost_usd: String,
    pub estimated_dead_call_loss_usd: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaleTrustBadge {
    pub last_probe_at: Option<DateTime<Utc>>,
    pub live: bool,
    pub stale: bool,
    pub stale_threshold_hours: i64,
    pub latest_probe_status: Option<String>,
    pub skip_cost: SkipCostExposure,
    pub fresh_probe_tool: String,
    pub k2_conversion_reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AttributionAnchor {
    pub anchor_type: &'static str,
    pub tool_slug: String,
    pub tool_id: Uuid,
    pub receipt_id: String,
    pub probed_at: DateTime<Utc>,
    pub endpoint_hash: String,
    pub note: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProbeReceipt {
    pub receipt_id: String,
    pub probed_at: DateTime<Utc>,
    pub endpoint_hash: String,
    pub live: bool,
    pub price_match: bool,
    pub advertised_price: Option<String>,
    pub actual_price: Option<String>,
    pub attribution_anchor: AttributionAnchor,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct LatestProbeRow {
    pub(crate) tool_id: Uuid,
    pub(crate) probed_at: DateTime<Utc>,
    pub(crate) status: String,
}

pub fn endpoint_hash(endpoint: &str) -> String {
    let digest = Sha256::digest(endpoint.trim().as_bytes());
    digest.iter().take(16).map(|b| format!("{b:02x}")).collect()
}

pub fn is_probe_stale(last_probe_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    match last_probe_at {
        None => true,
        Some(at) => now - at > Duration::hours(STALE_THRESHOLD_HOURS),
    }
}

pub fn probe_status_is_live(status: Option<&str>) -> bool {
    matches!(status, Some("live"))
}

pub fn build_skip_cost_exposure() -> SkipCostExposure {
    SkipCostExposure {
        probe_cost_usd: PROBE_COST_USD.into(),
        estimated_dead_call_loss_usd: ESTIMATED_DEAD_CALL_LOSS_USD.into(),
        message: format!(
            "Skipping a ${PROBE_COST_USD} pre-flight probe risks ~${ESTIMATED_DEAD_CALL_LOSS_USD} on a dead x402 endpoint."
        ),
    }
}

pub fn build_k2_conversion_reason(stale: bool, live: bool) -> String {
    if stale || !live {
        format!(
            "Last probe is stale or endpoint not LIVE — run check_endpoint_health (${PROBE_COST_USD}) before calling the third-party endpoint."
        )
    } else {
        format!(
            "Probe data is fresh — optional check_endpoint_health (${PROBE_COST_USD}) for attestation before third-party call."
        )
    }
}

pub fn build_stale_trust_badge(
    tool: &Tool,
    last_probe_at: Option<DateTime<Utc>>,
    latest_status: Option<String>,
    now: DateTime<Utc>,
) -> Option<StaleTrustBadge> {
    if tool.pricing != "x402" && tool.x402_endpoint.is_none() {
        return None;
    }

    let effective_last = last_probe_at.or(tool.x402_last_checked_at);
    let stale = is_probe_stale(effective_last, now);
    let live = probe_status_is_live(latest_status.as_deref())
        || (!stale && tool.x402_endpoint_verified && latest_status.is_none());

    Some(StaleTrustBadge {
        last_probe_at: effective_last,
        live,
        stale,
        stale_threshold_hours: STALE_THRESHOLD_HOURS,
        latest_probe_status: latest_status,
        skip_cost: build_skip_cost_exposure(),
        fresh_probe_tool: "check_endpoint_health".into(),
        k2_conversion_reason: build_k2_conversion_reason(stale, live),
    })
}

pub(crate) async fn load_latest_probes_by_tool_ids(
    pool: &PgPool,
    tool_ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, LatestProbeRow>, sqlx::Error> {
    if tool_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let rows = sqlx::query_as::<_, LatestProbeRow>(
        r#"
        SELECT DISTINCT ON (tool_id)
            tool_id,
            probed_at,
            status
        FROM x402_probe_history
        WHERE tool_id = ANY($1)
        ORDER BY tool_id, probed_at DESC
        "#,
    )
    .bind(tool_ids)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| (row.tool_id, row)).collect())
}

pub async fn stale_trust_badge_for_tool(
    pool: &PgPool,
    tool: &Tool,
) -> Result<Option<StaleTrustBadge>, sqlx::Error> {
    let probes = load_latest_probes_by_tool_ids(pool, std::slice::from_ref(&tool.id)).await?;
    let latest = probes.get(&tool.id);
    Ok(build_stale_trust_badge(
        tool,
        latest.map(|row| row.probed_at),
        latest.map(|row| row.status.clone()),
        Utc::now(),
    ))
}

pub fn build_probe_receipt(
    tool: &Tool,
    endpoint: &str,
    probed_at: DateTime<Utc>,
    outcome: &ProbeOutcome,
    advertised_price: Option<&str>,
) -> ProbeReceipt {
    let receipt_id = Uuid::new_v4().to_string();
    let endpoint_hash = endpoint_hash(endpoint);
    let (live, actual_price, price_match) = match outcome {
        ProbeOutcome::Verified { amount, .. } => {
            let match_ok = amount
                .as_deref()
                .zip(advertised_price)
                .is_some_and(|(probed, advertised)| price_matches_advertised(probed, advertised));
            (true, amount.clone(), match_ok)
        }
        _ => (false, None, false),
    };

    let attribution_anchor = AttributionAnchor {
        anchor_type: "k1_probe_receipt",
        tool_slug: tool.slug.clone(),
        tool_id: tool.id,
        receipt_id: receipt_id.clone(),
        probed_at,
        endpoint_hash: endpoint_hash.clone(),
        note: "Attach before third-party x402 call; strengthens attribution evidence, not automatic settlement.",
    };

    ProbeReceipt {
        receipt_id,
        probed_at,
        endpoint_hash,
        live,
        price_match,
        advertised_price: advertised_price.map(str::to_string),
        actual_price,
        attribution_anchor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::default_review_fields;

    fn sample_x402_tool() -> Tool {
        let review = default_review_fields();
        Tool {
            id: Uuid::new_v4(),
            name: "Probe Demo".into(),
            slug: "probe-demo".into(),
            description: None,
            function: "dev-tool".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "x402".into(),
            repo_url: None,
            homepage: None,
            npm_package: None,
            install_command: None,
            mcp_endpoint: None,
            chains: vec!["base".into()],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "approved".into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: review.crypto_relevance_score,
            crypto_relevance_reasons: review.crypto_relevance_reasons,
            relevance_status: review.relevance_status,
            install_risk_level: review.install_risk_level,
            install_risk_reasons: review.install_risk_reasons,
            requires_secret: review.requires_secret,
            safe_copy_command: review.safe_copy_command,
            quarantined_at: review.quarantined_at,
            last_reviewed_at: review.last_reviewed_at,
            review_policy_version: review.review_policy_version,
            claim_state: "unclaimed".into(),
            license: None,
            pricing: "x402".into(),
            x402_price: Some("0.001 usdc".into()),
            stars: 0,
            last_commit_at: None,
            source: "bazaar".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
            payment_verified: false,
            x402_endpoint_verified: true,
            price_verified: true,
            x402_endpoint: Some("https://pay.example.com/mcp".into()),
            x402_check_failures: 0,
            x402_last_checked_at: Some(Utc::now() - Duration::hours(1)),
            referral_enabled: false,
            referral_bps: None,
            referral_model: None,
            referral_payout_address: None,
            x402_builder_code: None,
            x402_pay_to_address: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn endpoint_hash_is_stable_and_truncated() {
        let h1 = endpoint_hash("https://pay.example.com/a");
        let h2 = endpoint_hash("https://pay.example.com/a");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);
    }

    #[test]
    fn stale_after_24h_threshold() {
        let now = Utc::now();
        let fresh = now - Duration::hours(12);
        let old = now - Duration::hours(25);
        assert!(!is_probe_stale(Some(fresh), now));
        assert!(is_probe_stale(Some(old), now));
        assert!(is_probe_stale(None, now));
    }

    #[test]
    fn stale_badge_marks_stale_dead_endpoint() {
        let tool = sample_x402_tool();
        let badge = build_stale_trust_badge(
            &tool,
            Some(Utc::now() - Duration::hours(30)),
            Some("dead".into()),
            Utc::now(),
        )
        .expect("badge");
        assert!(badge.stale);
        assert!(!badge.live);
        assert!(badge.k2_conversion_reason.contains("stale"));
        assert!(badge.skip_cost.message.contains(PROBE_COST_USD));
    }

    #[test]
    fn probe_receipt_reflects_live_price_match() {
        let tool = sample_x402_tool();
        let outcome = ProbeOutcome::Verified {
            amount: Some("1000".into()),
            asset: Some("usdc".into()),
        };
        let receipt = build_probe_receipt(
            &tool,
            "https://pay.example.com/mcp",
            Utc::now(),
            &outcome,
            Some("$1,000 USDC"),
        );
        assert!(receipt.live);
        assert!(receipt.price_match);
        assert_eq!(receipt.attribution_anchor.anchor_type, "k1_probe_receipt");
        assert_eq!(receipt.attribution_anchor.tool_slug, "probe-demo");
    }
}
