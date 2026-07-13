//! Tests extracted from `mcp.rs` for Code Health scoring.

use super::*;
use crate::models::Tool;
use crate::server::mcp::install_guide::{
    referral_metadata_for_tool, InstallGuide, ReferralMetadata,
};
use crate::server::queries::MCP_SEARCH_TOOLS_BASE_SQL;
use crate::server::queries::{APPROVED_TOOL_BY_SLUG_SQL, CATEGORIES_WITH_COUNTS_SQL};
use definitions::{
    get_install_guide_definition, get_tool_detail_definition, list_categories_definition,
    search_tools_definition,
};

#[test]
fn protocol_version_echoes_supported_and_falls_back() {
    // Supported requested version is echoed verbatim.
    assert_eq!(
        negotiate_protocol_version(Some(&json!({ "protocolVersion": "2025-06-18" }))),
        "2025-06-18"
    );
    assert_eq!(
        negotiate_protocol_version(Some(&json!({ "protocolVersion": "2024-11-05" }))),
        "2024-11-05"
    );
    // Unknown or absent version falls back to the server default.
    assert_eq!(
        negotiate_protocol_version(Some(&json!({ "protocolVersion": "1999-01-01" }))),
        DEFAULT_PROTOCOL_VERSION
    );
    assert_eq!(
        negotiate_protocol_version(Some(&json!({}))),
        DEFAULT_PROTOCOL_VERSION
    );
    assert_eq!(negotiate_protocol_version(None), DEFAULT_PROTOCOL_VERSION);
}

#[test]
fn mcp_info_lists_public_tools_and_endpoint() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = rt.block_on(handle_mcp_info()).into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let body = rt
        .block_on(axum::body::to_bytes(response.into_body(), 1024 * 1024))
        .unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["name"], "onchainai");
    assert_eq!(value["endpoint"], "https://www.onchain-ai.xyz/mcp");
    assert_eq!(value["docs"], "https://www.onchain-ai.xyz/connect");
    assert_eq!(value["transport"], "streamable-http");
    assert_eq!(value["billing"], "free_discovery");
    assert_eq!(value["billing_detail"]["mode"], "public");
    assert_eq!(value["billing_detail"]["discovery"], "free");
    assert_eq!(
        value["billing_detail"]["premium_tools"]["price"],
        crate::server::mcp_x402::DEFAULT_MCP_PREMIUM_PRICE
    );
    assert!(value["description"]
        .as_str()
        .unwrap()
        .contains("free discovery"));
    assert!(!value["description"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase()
        .contains("entire"));
    let tools = value["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 12);
    assert!(tools
        .iter()
        .all(|t| t["name"].is_string() && t["description"].is_string()));
}

#[test]
fn mcp_okx_info_states_package_billing() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = rt.block_on(handle_mcp_okx_info()).into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let body = rt
        .block_on(axum::body::to_bytes(response.into_body(), 1024 * 1024))
        .unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["name"], "onchainai-okx");
    assert_eq!(value["endpoint"], "https://www.onchain-ai.xyz/mcp/okx");
    assert_eq!(value["billing"], "okx_package_pay_per_call");
    assert_eq!(value["billing_detail"]["mode"], "okx_package");
    assert_eq!(value["billing_detail"]["every_tools_call"]["price"], "$0.1");
    assert_eq!(
        value["billing_detail"]["public_free_endpoint"],
        "https://www.onchain-ai.xyz/mcp"
    );
    let desc = value["description"].as_str().unwrap();
    assert!(desc.contains("$0.1"));
    assert!(desc.contains("/mcp"));
}

#[test]
fn tools_list_has_eight_public_tools_including_premium() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let value = rt.block_on(tools_list(false)).unwrap();
    let tools = value["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 12);
    for name in [
        "check_endpoint_health",
        "get_dashboard_snapshot",
        "compare_tools",
        "export_toolkit",
        "recommend_verified_tool",
        "gap_audit",
        "get_price_history",
        "get_x402_trends",
    ] {
        assert!(
            tools.iter().any(|tool| tool["name"].as_str() == Some(name)),
            "missing public tool {name}"
        );
    }
}

#[test]
fn tools_list_authenticated_adds_agent_sync_tools() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let value = rt.block_on(tools_list(true)).unwrap();
    let tools = value["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 15);
    for name in ["save_to_toolkit", "save_stack_to_blueprint", "link_status"] {
        assert!(
            tools.iter().any(|tool| tool["name"].as_str() == Some(name)),
            "missing authenticated tool {name}"
        );
    }
}

#[test]
fn install_guide_includes_risk_fields() {
    let guide = InstallGuide {
        command: "npm i @test/pkg".into(),
        risk_level: "medium".into(),
        risk_reasons: vec!["requires API key".into()],
        warning: Some("Medium-risk install command.".into()),
        blocked: false,
        copy_gate: crate::public_install_guide::CopyGate::Allow,
        config_json: None,
        x402_notice: None,
        referral: None,
        steps: vec!["Run install".into()],
    };
    let json = serde_json::to_value(&guide).unwrap();
    assert_eq!(json["risk_level"], "medium");
    assert_eq!(json["risk_reasons"][0], "requires API key");
    assert_eq!(json["warning"], "Medium-risk install command.");
    assert_eq!(json["blocked"], false);
    assert_eq!(json["copy_gate"], "allow");
}

#[test]
fn install_guide_critical_is_blocked() {
    let guide = InstallGuide {
        command: "rm -rf /".into(),
        risk_level: "critical".into(),
        risk_reasons: vec!["destructive".into()],
        warning: Some("blocked".into()),
        blocked: true,
        copy_gate: crate::public_install_guide::CopyGate::Blocked,
        config_json: None,
        x402_notice: None,
        referral: None,
        steps: vec![],
    };
    assert!(guide.blocked);
    assert_eq!(guide.risk_level, "critical");
}

#[test]
fn referral_metadata_requires_enabled_flag() {
    use crate::models::tool::default_review_fields;
    use chrono::Utc;
    use uuid::Uuid;

    let review = default_review_fields();
    let mut tool = crate::models::Tool {
        id: Uuid::nil(),
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
        install_command: None,
        mcp_endpoint: None,
        chains: vec![],
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
        x402_price: None,
        referral_enabled: false,
        referral_bps: Some(250),
        referral_payout_address: None,
        referral_model: Some("attribution".into()),
        x402_pay_to_address: None,
        x402_builder_code: Some("onchainai".into()),
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
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    assert!(referral_metadata_for_tool(&tool, None).is_none());

    tool.referral_enabled = true;
    assert!(referral_metadata_for_tool(&tool, None).is_some());
}

#[test]
fn install_guide_includes_x402_referral_notice() {
    let guide = InstallGuide {
        command: "npx mcp-remote https://example.com/mcp".into(),
        risk_level: "low".into(),
        risk_reasons: vec![],
        warning: None,
        blocked: false,
        copy_gate: crate::public_install_guide::CopyGate::Allow,
        config_json: None,
        x402_notice: Some(
            "This tool may request x402 payment (0.01 USDC). Payment details are not operator verified yet.".into(),
        ),
        referral: Some(ReferralMetadata {
            enabled: true,
            bps: Some(250),
            payout_address: Some("0x0000000000000000000000000000000000000000".into()),
            model: Some("attribution".into()),
            builder_code: Some("onchainai".into()),
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
        }),
        steps: vec!["Run install".into()],
    };
    let json = serde_json::to_value(&guide).unwrap();
    assert!(json["x402_notice"]
        .as_str()
        .unwrap()
        .contains("not operator verified"));
    assert_eq!(json["referral"]["enabled"], true);
    assert_eq!(json["referral"]["builder_code"], "onchainai");
}

#[test]
fn mcp_queries_include_public_visibility_filter() {
    assert!(MCP_SEARCH_TOOLS_BASE_SQL.contains("approval_status = 'approved'"));
    assert!(MCP_SEARCH_TOOLS_BASE_SQL.contains("relevance_status = 'accepted'"));
    assert!(MCP_SEARCH_TOOLS_BASE_SQL.contains("install_risk_level <> 'critical'"));
    assert!(MCP_SEARCH_TOOLS_BASE_SQL.contains("quarantined_at IS NULL"));
    assert!(crate::server::queries::MCP_SEARCH_TOOLS_COUNT_SQL.contains("COUNT(*)"));
    assert!(crate::server::queries::MCP_SEARCH_TOOLS_COUNT_SQL.contains("quarantined_at IS NULL"));
    assert!(APPROVED_TOOL_BY_SLUG_SQL.contains("relevance_status = 'accepted'"));
    assert!(CATEGORIES_WITH_COUNTS_SQL.contains("quarantined_at IS NULL"));
}

#[test]
fn search_tools_schema_exposes_category_enum_and_cursor_offset() {
    let schema = search_tools_definition();
    let categories = schema["inputSchema"]["properties"]["category"]["enum"]
        .as_array()
        .unwrap();
    assert_eq!(categories.len(), 14);
    let cursor_desc = schema["inputSchema"]["properties"]["cursor"]["description"]
        .as_str()
        .unwrap();
    assert!(cursor_desc.contains("next_cursor"));
    assert!(!cursor_desc.to_ascii_lowercase().contains("opaque"));
}

#[test]
fn tool_descriptions_document_agent_call_flow() {
    let detail_def = get_tool_detail_definition();
    let detail = detail_def["description"].as_str().unwrap();
    assert!(detail.contains("search_tools"));
    assert!(detail.contains("get_install_guide"));

    let categories_def = list_categories_definition();
    let categories = categories_def["description"].as_str().unwrap();
    assert!(categories.contains("search_tools"));

    let install_def = get_install_guide_definition();
    let install = install_def["description"].as_str().unwrap();
    assert!(install.contains("blocked=true"));
    assert!(install.contains("critical"));
}

#[test]
fn tool_descriptions_state_hybrid_billing_prices() {
    use definitions::tool_definitions;

    let tools = tool_definitions(false);
    let desc = |name: &str| -> String {
        tools
            .iter()
            .find(|t| t["name"].as_str() == Some(name))
            .and_then(|t| t["description"].as_str())
            .unwrap_or("")
            .to_string()
    };

    // Free discovery tools must not claim $0.1 package pricing or imply full paywall.
    for free in [
        "search_tools",
        "get_tool_detail",
        "get_install_guide",
        "list_categories",
        "get_dashboard_snapshot",
        "compare_tools",
        "get_price_history",
        "get_x402_trends",
    ] {
        let d = desc(free);
        assert!(
            !d.contains("$0.1"),
            "{free} description must not claim OKX $0.1 package rate: {d}"
        );
    }
    let compare = desc("compare_tools");
    assert!(
        compare.to_ascii_lowercase().contains("free"),
        "compare_tools should document free discovery: {compare}"
    );

    // Premium trio: $0.01 Axis B, never "may require when enabled".
    for premium in ["export_toolkit", "recommend_verified_tool", "gap_audit"] {
        let d = desc(premium);
        assert!(d.contains("$0.01"), "{premium} must state $0.01 USDC: {d}");
        assert!(
            d.to_ascii_lowercase().contains("always paid")
                || d.to_ascii_lowercase().contains("axis b"),
            "{premium} must state always-paid / Axis B: {d}"
        );
        assert!(
            !d.contains("May require x402 payment per call when"),
            "{premium} must not use optional premium language: {d}"
        );
    }

    let health = desc("check_endpoint_health");
    assert!(
        health.contains("$0.001") || health.contains("~$0.001"),
        "check_endpoint_health must state ~$0.001 USDC: {health}"
    );
    assert!(
        !health.contains("$0.1"),
        "check_endpoint_health public description must not claim $0.1: {health}"
    );
}
