use super::mcp_fetch_public_tool;
use crate::install_safety::{blocks_structured_config, claude_mcp_config, install_warning_text};
use crate::models::Tool;
use serde::Serialize;
use serde_json::json;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x402_notice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referral: Option<ReferralMetadata>,
    pub steps: Vec<String>,
}

struct InstallGuideContext<'a> {
    tool: &'a Tool,
    slug: &'a str,
    platform: InstallPlatform,
    command: String,
    risk_level: String,
    risk_reasons: Vec<String>,
    warning: Option<String>,
}

impl<'a> InstallGuideContext<'a> {
    fn new(tool: &'a Tool, slug: &'a str, platform: InstallPlatform) -> Self {
        let risk_level = tool.install_risk_level.clone();
        Self {
            tool,
            slug,
            platform,
            command: install_command_for(tool),
            risk_reasons: tool.install_risk_reasons.clone(),
            warning: install_warning_text(&risk_level).map(str::to_string),
            risk_level,
        }
    }

    fn blocked(&self) -> bool {
        self.risk_level == "critical"
    }

    fn config_blocked(&self) -> bool {
        blocks_structured_config(&self.risk_level)
    }

    fn x402_notice(&self) -> Option<String> {
        x402_notice_for_tool(self.tool)
    }

    fn referral(&self) -> Option<ReferralMetadata> {
        referral_metadata_for_tool(self.tool)
    }
}

#[derive(Clone, Copy)]
enum InstallPlatform {
    Claude,
    Cursor,
    Generic,
}

impl InstallPlatform {
    fn parse(raw: &str) -> Result<Self, (i32, String)> {
        match raw {
            "claude" => Ok(Self::Claude),
            "cursor" => Ok(Self::Cursor),
            "generic" => Ok(Self::Generic),
            other => Err((-32602, format!("invalid platform: {other}"))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::Generic => "generic",
        }
    }
}

struct X402ToolNotice<'a>(&'a Tool);

impl X402ToolNotice<'_> {
    fn render(&self) -> Option<String> {
        self.has_payment_surface().then(|| self.message())
    }

    fn has_payment_surface(&self) -> bool {
        self.0.pricing == "x402" || self.0.x402_price.is_some() || self.0.referral_enabled
    }

    fn message(&self) -> String {
        format!(
            "This tool may request x402 payment ({}). Connect an agent wallet before calling it. {}",
            self.price_label(),
            self.verification_label()
        )
    }

    fn price_label(&self) -> &str {
        self.0
            .x402_price
            .as_deref()
            .filter(|price| !price.trim().is_empty())
            .unwrap_or("the provider's x402 price")
    }

    fn verification_label(&self) -> &'static str {
        if self.payment_details_verified() {
            "Payment details are operator verified."
        } else {
            "Payment details are not operator verified yet."
        }
    }

    fn payment_details_verified(&self) -> bool {
        self.0.payment_verified && self.0.x402_endpoint_verified && self.0.price_verified
    }
}

fn x402_notice_for_tool(tool: &Tool) -> Option<String> {
    X402ToolNotice(tool).render()
}

pub(crate) fn referral_metadata_for_tool(tool: &Tool) -> Option<ReferralMetadata> {
    tool.referral_enabled.then(|| ReferralMetadata {
        enabled: tool.referral_enabled,
        bps: tool.referral_bps,
        payout_address: tool.referral_payout_address.clone(),
        model: tool.referral_model.clone(),
        builder_code: tool.x402_builder_code.clone(),
        payment_verified: tool.payment_verified,
        x402_endpoint_verified: tool.x402_endpoint_verified,
        price_verified: tool.price_verified,
    })
}

async fn record_referral_event(pool: &PgPool, tool: &Tool, event_type: &str, platform: &str) {
    if !records_referral_event(tool) {
        return;
    }
    let metadata = json!({
        "platform": platform,
        "source": "mcp_install_guide",
        "builder_code": tool.x402_builder_code,
    });
    if let Err(error) = insert_referral_event(pool, tool, event_type, metadata).await {
        tracing::warn!(
            tool_id = %tool.id,
            event_type,
            "failed to record referral event: {error}"
        );
    }
}

fn records_referral_event(tool: &Tool) -> bool {
    tool.referral_enabled || tool.pricing == "x402"
}

async fn insert_referral_event(
    pool: &PgPool,
    tool: &Tool,
    event_type: &str,
    metadata: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO referral_events (tool_id, event_type, metadata)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(tool.id)
    .bind(event_type)
    .bind(metadata)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn mcp_install_guide(
    pool: &PgPool,
    slug: &str,
    platform: &str,
) -> Result<InstallGuide, (i32, String)> {
    let platform = InstallPlatform::parse(platform)?;
    let tool = mcp_fetch_public_tool(pool, slug)
        .await
        .map_err(|m| (-32000, m))?;
    record_referral_event(pool, &tool, "install_guide", platform.as_str()).await;
    build_install_guide(InstallGuideContext::new(&tool, slug, platform))
}

fn build_install_guide(context: InstallGuideContext<'_>) -> Result<InstallGuide, (i32, String)> {
    if context.blocked() {
        return Ok(blocked_install_guide(&context));
    }
    Ok(platform_install_guide(&context, platform_config(&context)))
}

fn blocked_install_guide(context: &InstallGuideContext<'_>) -> InstallGuide {
    InstallGuide {
        command: context.command.clone(),
        risk_level: context.risk_level.clone(),
        risk_reasons: context.risk_reasons.clone(),
        warning: Some("Install guidance blocked: critical risk pending operator review.".into()),
        blocked: true,
        config_json: None,
        x402_notice: context.x402_notice(),
        referral: context.referral(),
        steps: vec![
            "This tool has a critical-risk install command.".into(),
            "Public install guidance is withheld until an operator reviews the listing.".into(),
            "Contact the project directly or wait for operator approval.".into(),
        ],
    }
}

fn platform_install_guide(
    context: &InstallGuideContext<'_>,
    platform: PlatformInstallConfig,
) -> InstallGuide {
    InstallGuide {
        command: context.command.clone(),
        risk_level: context.risk_level.clone(),
        risk_reasons: context.risk_reasons.clone(),
        warning: context.warning.clone(),
        blocked: false,
        config_json: platform.config_json,
        x402_notice: context.x402_notice(),
        referral: context.referral(),
        steps: platform.steps,
    }
}

struct PlatformInstallConfig {
    config_json: Option<String>,
    steps: Vec<String>,
}

fn platform_config(context: &InstallGuideContext<'_>) -> PlatformInstallConfig {
    match context.platform {
        InstallPlatform::Claude => claude_install_config(context),
        InstallPlatform::Cursor => cursor_install_config(context),
        InstallPlatform::Generic => generic_install_config(),
    }
}

fn claude_install_config(context: &InstallGuideContext<'_>) -> PlatformInstallConfig {
    PlatformInstallConfig {
        config_json: claude_config_json(context),
        steps: vec![
            "Open Claude Desktop settings.".into(),
            claude_config_step(context.config_blocked()),
            "Restart Claude to load the tool.".into(),
        ],
    }
}

fn claude_config_json(context: &InstallGuideContext<'_>) -> Option<String> {
    (!context.config_blocked()).then_some(())?;
    claude_mcp_config(context.slug, &context.command, &context.risk_level)
}

fn claude_config_step(config_blocked: bool) -> String {
    if config_blocked {
        "Structured config is unavailable for high-risk commands; use generic install only if you trust the source."
    } else {
        "Paste the structured MCP config JSON into your Claude settings."
    }
    .into()
}

fn cursor_install_config(context: &InstallGuideContext<'_>) -> PlatformInstallConfig {
    PlatformInstallConfig {
        config_json: cursor_config_json(context),
        steps: vec![
            "Open Cursor MCP settings.".into(),
            cursor_config_step(context.config_blocked()),
            "Reload MCP servers.".into(),
        ],
    }
}

fn cursor_config_json(context: &InstallGuideContext<'_>) -> Option<String> {
    (!context.config_blocked()).then(|| {
        let mcp_url = crate::config::mcp_remote_url_from_command(crate::config::MCP_ENDPOINT_CMD);
        json!({
            "mcpServers": {
                context.slug: {
                    "command": "npx",
                    "args": ["mcp-remote", mcp_url]
                }
            }
        })
        .to_string()
    })
}

fn cursor_config_step(config_blocked: bool) -> String {
    if config_blocked {
        "High-risk install: do not paste raw shell wrappers. Add manually only if you trust the source."
    } else {
        "Paste the config JSON or use the install command."
    }
    .into()
}

fn generic_install_config() -> PlatformInstallConfig {
    PlatformInstallConfig {
        config_json: None,
        steps: vec!["Run the install command in your terminal.".into()],
    }
}

fn install_command_for(tool: &Tool) -> String {
    tool.safe_copy_command
        .clone()
        .or(tool.install_command.clone())
        .unwrap_or_else(|| "No install command available.".into())
}
