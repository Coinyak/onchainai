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

/// Map guide copy labels to stable accessible button names.
pub fn copy_label_aria(copy_label: &str) -> &'static str {
    match copy_label {
        "Copy config" => "Copy config",
        "Copy command" => "Copy command",
        "Copy blocked" => "Copy blocked",
        _ => "Copy to clipboard",
    }
}

