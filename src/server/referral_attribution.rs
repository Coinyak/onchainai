//! Shared install-guide attribution recording (MCP + web X4).

use crate::models::Tool;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Site-level referral defaults from `site_settings` (id = 1).
#[derive(Debug, Clone)]
pub struct ReferralSiteDefaults {
    pub bps: Option<i32>,
    pub payout_address: Option<String>,
    pub builder_code: Option<String>,
}

pub async fn fetch_site_referral_defaults(pool: &PgPool) -> Option<ReferralSiteDefaults> {
    let row = sqlx::query_as::<_, (Option<i32>, Option<String>, Option<String>)>(
        r#"
        SELECT default_referral_bps, default_referral_payout_address, x402_builder_code
        FROM site_settings
        WHERE id = 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;
    Some(ReferralSiteDefaults {
        bps: row.0,
        payout_address: row.1,
        builder_code: row.2,
    })
}

pub fn resolved_builder_code(
    tool: &Tool,
    defaults: Option<&ReferralSiteDefaults>,
) -> Option<String> {
    tool.x402_builder_code
        .clone()
        .or_else(|| defaults.and_then(|d| d.builder_code.clone()))
}

pub fn records_referral_event(tool: &Tool) -> bool {
    tool.referral_enabled || tool.pricing == "x402"
}

#[derive(Debug, Clone)]
pub struct ReferralMetadata {
    pub enabled: bool,
    pub bps: Option<i32>,
    pub payout_address: Option<String>,
    pub model: Option<String>,
    pub builder_code: Option<String>,
    pub payment_verified: bool,
    pub x402_endpoint_verified: bool,
    pub price_verified: bool,
}

pub fn referral_metadata_for_tool(
    tool: &Tool,
    defaults: Option<&ReferralSiteDefaults>,
) -> Option<ReferralMetadata> {
    tool.referral_enabled.then(|| ReferralMetadata {
        enabled: tool.referral_enabled,
        bps: tool.referral_bps.or_else(|| defaults.and_then(|d| d.bps)),
        payout_address: tool
            .referral_payout_address
            .clone()
            .or_else(|| defaults.and_then(|d| d.payout_address.clone())),
        model: tool.referral_model.clone(),
        builder_code: resolved_builder_code(tool, defaults),
        payment_verified: tool.payment_verified,
        x402_endpoint_verified: tool.x402_endpoint_verified,
        price_verified: tool.price_verified,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstallGuideAttributionRequest {
    pub platform: String,
    #[serde(default)]
    pub attribution_session: Option<String>,
}

pub struct InstallGuideAttributionInput<'a> {
    pub tool: &'a Tool,
    pub platform: &'a str,
    pub source: &'a str,
    pub attribution_session: Option<&'a str>,
    pub site_defaults: Option<&'a ReferralSiteDefaults>,
}

pub enum InstallGuideAttributionOutcome {
    Recorded,
    SkippedNotBillable,
    SkippedDuplicate,
}

pub async fn record_install_guide_attribution(
    pool: &PgPool,
    input: InstallGuideAttributionInput<'_>,
) -> Result<InstallGuideAttributionOutcome, sqlx::Error> {
    if !records_referral_event(input.tool) {
        return Ok(InstallGuideAttributionOutcome::SkippedNotBillable);
    }

    let session = normalize_attribution_session(input.attribution_session);
    if attribution_recently_recorded(pool, input.tool.id, &session).await? {
        return Ok(InstallGuideAttributionOutcome::SkippedDuplicate);
    }

    let metadata = serde_json::json!({
        "platform": input.platform,
        "source": input.source,
        "builder_code": resolved_builder_code(input.tool, input.site_defaults),
    });
    insert_referral_event(
        pool,
        input.tool.id,
        "install_guide",
        Some(session),
        metadata,
    )
    .await?;
    Ok(InstallGuideAttributionOutcome::Recorded)
}

pub async fn record_mcp_install_guide_attribution(
    pool: &PgPool,
    tool: &Tool,
    platform: &str,
    defaults: Option<&ReferralSiteDefaults>,
) {
    let input = InstallGuideAttributionInput {
        tool,
        platform,
        source: "mcp_install_guide",
        attribution_session: None,
        site_defaults: defaults,
    };
    if let Err(error) = record_install_guide_attribution(pool, input).await {
        tracing::warn!(
            tool_id = %tool.id,
            "failed to record MCP install guide attribution: {error}"
        );
    }
}

fn normalize_attribution_session(raw: Option<&str>) -> String {
    let trimmed = raw.unwrap_or("").trim();
    if trimmed.is_empty() {
        return "anonymous".into();
    }
    if trimmed.len() > 128 {
        return trimmed.chars().take(128).collect();
    }
    trimmed.to_string()
}

async fn attribution_recently_recorded(
    pool: &PgPool,
    tool_id: Uuid,
    attribution_session: &str,
) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM referral_events
            WHERE tool_id = $1
              AND event_type = 'install_guide'
              AND COALESCE(attribution_session, '') = $2
              AND created_at > now() - interval '1 hour'
        )
        "#,
    )
    .bind(tool_id)
    .bind(attribution_session)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

async fn insert_referral_event(
    pool: &PgPool,
    tool_id: Uuid,
    event_type: &str,
    attribution_session: Option<String>,
    metadata: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO referral_events (tool_id, event_type, attribution_session, metadata)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(tool_id)
    .bind(event_type)
    .bind(attribution_session)
    .bind(metadata)
    .execute(pool)
    .await?;
    Ok(())
}

pub fn validate_attribution_platform(platform: &str) -> Result<(), &'static str> {
    let platform = platform.trim();
    if platform.is_empty() {
        return Err("platform is required");
    }
    if platform.len() > 64 {
        return Err("platform must be at most 64 characters");
    }
    if !platform
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err("platform contains invalid characters");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_referral_event_for_x402_or_enabled() {
        let review = crate::models::tool::default_review_fields();
        let mut row = crate::models::Tool {
            id: Uuid::new_v4(),
            name: "T".into(),
            slug: "t".into(),
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
            claim_state: "unclaimed".into(),
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
            last_commit_at: None,
            source: "manual".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert!(!records_referral_event(&row));
        row.referral_enabled = true;
        assert!(records_referral_event(&row));
        row.referral_enabled = false;
        row.pricing = "x402".into();
        assert!(records_referral_event(&row));
    }

    #[test]
    fn validate_attribution_platform_rejects_empty_and_invalid() {
        assert!(validate_attribution_platform("").is_err());
        assert!(validate_attribution_platform("cursor").is_ok());
        assert!(validate_attribution_platform("cli_sdk").is_ok());
        assert!(validate_attribution_platform("bad platform").is_err());
    }
}
