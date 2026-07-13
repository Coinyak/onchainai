//! Public install guide assembly.
use crate::install_safety::{
    blocks_structured_config, claude_mcp_config, install_warning_text,
};
use crate::models::Tool;

use super::commands::*;
use super::types::*;

pub fn build_public_install_guide(
    tool: &Tool,
    slug: &str,
    platform: InstallPlatform,
) -> PublicInstallGuide {
    if tool.install_risk_level == "critical" {
        return blocked_guide(tool, slug, platform);
    }

    if tool.tool_type == "skill" && !is_mcp_catalog_tool(tool) {
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
