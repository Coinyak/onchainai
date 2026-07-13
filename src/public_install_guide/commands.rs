//! Install command / config builders.
use crate::install_safety::{
    blocks_structured_config, claude_mcp_config, install_warning_text,
};
use crate::models::Tool;

use super::types::*;


pub fn http_mcp_universal_install_command(endpoint: &str) -> Option<String> {
    let endpoint = endpoint.trim();
    let parsed = url::Url::parse(endpoint).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    parsed.host_str()?;
    if endpoint.chars().any(|c| {
        matches!(
            c,
            ';' | '&'
                | '|'
                | '`'
                | '$'
                | '('
                | ')'
                | '<'
                | '>'
                | '\n'
                | '\r'
                | '\''
                | '\\'
                | '"'
                | ' '
                | '\t'
        )
    }) {
        return None;
    }
    Some(format!("npx add-mcp {}", parsed.as_str()))
}

pub(crate) fn generic_mcp_remote_command(endpoint: &str) -> Option<String> {
    http_mcp_universal_install_command(endpoint)
}

pub(crate) fn primary_install_command(tool: &Tool) -> Option<String> {
    tool.safe_copy_command
        .clone()
        .or_else(|| tool.install_command.clone())
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            if tool.tool_type == "skill" {
                return None;
            }
            tool.mcp_endpoint
                .as_deref()
                .and_then(generic_mcp_remote_command)
        })
}

pub(crate) fn npm_package_url(package: Option<&str>) -> Option<String> {
    let package = package?.trim();
    if package.is_empty() || package.starts_with("http://") || package.starts_with("https://") {
        return None;
    }
    Some(format!("https://www.npmjs.com/package/{package}"))
}

pub(crate) fn docs_links_for_tool(tool: &Tool) -> Vec<GuideLink> {
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

pub(crate) fn structured_config_json(slug: &str, command: &str, risk_level: &str) -> Option<String> {
    claude_mcp_config(slug, command, risk_level)
}

pub(crate) fn blocked_guide(tool: &Tool, slug: &str, platform: InstallPlatform) -> PublicInstallGuide {
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

pub(crate) fn is_mcp_catalog_tool(tool: &Tool) -> bool {
    tool.tool_type == "mcp" || tool.tool_type == "x402" || tool.mcp_endpoint.is_some()
}

pub(crate) fn command_only_steps_without_command() -> Vec<String> {
    vec![
        "No install command is listed for this tool.".into(),
        "Use the repository or docs links below for setup.".into(),
    ]
}

pub(crate) fn command_only_guide(
    tool: &Tool,
    slug: &str,
    platform: InstallPlatform,
    steps: Vec<String>,
) -> PublicInstallGuide {
    let risk_level = tool.install_risk_level.clone();
    let copy_gate = CopyGate::for_risk(&risk_level);
    let command = primary_install_command(tool);
    let steps = if command.is_some() {
        steps
    } else {
        command_only_steps_without_command()
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

