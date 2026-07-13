//! MCP server — JSON-RPC 2.0 handler with public tools at POST /mcp.

use crate::AppState;
use crate::server::rate_limit::{check_mcp_ip_rate_limit, client_ip_from_parts};
use axum::{
    extract::State,
    http::Request,
    response::IntoResponse,
    Json,
};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

mod auth;
mod call;
mod definitions;
mod install_guide;

use auth::agent_from_authorization;
use call::{mcp_fetch_public_tool, tools_call, ToolsCallOutcome};
use definitions::{tool_definitions, tools_list};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    jsonrpc: Option<String>,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpErrorObj>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct McpErrorObj {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Axum handler for `POST /mcp`.
pub async fn handle_mcp(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
) -> impl IntoResponse {
    let (parts, body) = req.into_parts();
    let client_ip = client_ip_from_parts(&parts);
    if let Err(limit) = check_mcp_ip_rate_limit(&client_ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(error_response(Value::Null, -32099, &limit.to_string())),
        )
            .into_response();
    }

    let body_bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(error_response(Value::Null, -32700, "Parse error")),
            )
                .into_response();
        }
    };
    let rpc_req: JsonRpcRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(error_response(Value::Null, -32700, "Parse error")),
            )
                .into_response();
        }
    };

    let id = rpc_req.id.clone().unwrap_or(Value::Null);

    if rpc_req.jsonrpc.as_deref() != Some("2.0") {
        return (
            StatusCode::OK,
            Json(error_response(id, -32600, "Invalid Request")),
        )
            .into_response();
    }

    let result = match rpc_req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": negotiate_protocol_version(rpc_req.params.as_ref()),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "onchainai", "version": env!("CARGO_PKG_VERSION") }
        })),
        "notifications/initialized" => Ok(json!({})),
        "tools/list" => {
            let agent = agent_from_authorization(
                &state.pool,
                parts
                    .headers
                    .get(axum::http::header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok()),
            )
            .await;
            tools_list(agent.is_some()).await
        }
        "tools/call" => {
            let agent = agent_from_authorization(
                &state.pool,
                parts
                    .headers
                    .get(axum::http::header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok()),
            )
            .await;
            return match tools_call(
                &state.pool,
                rpc_req.params,
                agent.as_ref(),
                &parts.headers,
                state.okx_client.as_ref(),
                state.okx_premium_gate_active,
            )
            .await
            {
                ToolsCallOutcome::Ok(value) => {
                    (StatusCode::OK, Json(ok_response(id, value))).into_response()
                }
                ToolsCallOutcome::Http(response) => response,
                ToolsCallOutcome::Err((code, msg)) => {
                    (StatusCode::OK, Json(error_response(id, code, &msg))).into_response()
                }
            };
        }
        other => Err((-32601, format!("Method not found: {other}"))),
    };

    match result {
        Ok(value) => (StatusCode::OK, Json(ok_response(id, value))).into_response(),
        Err((code, msg)) => (StatusCode::OK, Json(error_response(id, code, &msg))).into_response(),
    }
}

/// MCP protocol version this server implements and defaults to.
const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";

/// Protocol versions the server is wire-compatible with (tools-only surface).
/// The `initialize` response echoes the client's requested version when it is
/// on this list, otherwise falls back to [`DEFAULT_PROTOCOL_VERSION`].
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2024-11-05", "2025-03-26", "2025-06-18"];

/// Negotiate the `protocolVersion` for an `initialize` response: echo the
/// client-requested version if supported, else the server default.
fn negotiate_protocol_version(params: Option<&Value>) -> &'static str {
    let requested = params
        .and_then(|p| p.get("protocolVersion"))
        .and_then(|v| v.as_str());
    match requested {
        Some(req) => SUPPORTED_PROTOCOL_VERSIONS
            .iter()
            .find(|&&v| v == req)
            .copied()
            .unwrap_or(DEFAULT_PROTOCOL_VERSION),
        None => DEFAULT_PROTOCOL_VERSION,
    }
}

/// GET `/mcp` — human/crawler-friendly discovery response. Standard MCP clients
/// POST JSON-RPC here; a plain browser GET returns a 200 describing the server
/// instead of a bare 405, so anyone who opens the URL understands it instantly.
pub async fn handle_mcp_info() -> impl IntoResponse {
    let tools: Vec<Value> = tool_definitions(false)
        .into_iter()
        .map(|def| {
            json!({
                "name": def.get("name").cloned().unwrap_or(Value::Null),
                "description": def.get("description").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    (
        StatusCode::OK,
        Json(json!({
            "name": "onchainai",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "OnchainAI MCP server — discover, compare, and install crypto/onchain AI tools. POST JSON-RPC 2.0 to this endpoint from an MCP client.",
            "protocolVersion": DEFAULT_PROTOCOL_VERSION,
            "endpoint": format!("{}/mcp", crate::config::SITE_ORIGIN),
            "transport": "streamable-http",
            "docs": format!("{}/connect", crate::config::SITE_ORIGIN),
            "tools": tools,
        })),
    )
        .into_response()
}

fn ok_response(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        result: Some(result),
        error: None,
        id,
    }
}

fn error_response(id: Value, code: i32, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        result: None,
        error: Some(McpErrorObj {
            code,
            message: message.to_string(),
            data: None,
        }),
        id,
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Tool;
    use crate::server::queries::{APPROVED_TOOL_BY_SLUG_SQL, CATEGORIES_WITH_COUNTS_SQL};
    use definitions::{
        get_install_guide_definition, get_tool_detail_definition, list_categories_definition,
        search_tools_definition,
    };
    use crate::server::mcp::install_guide::{
        referral_metadata_for_tool, InstallGuide, ReferralMetadata,
    };
    use crate::server::queries::MCP_SEARCH_TOOLS_BASE_SQL;

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
        let tools = value["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 12);
        assert!(tools
            .iter()
            .all(|t| t["name"].is_string() && t["description"].is_string()));
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
        assert!(
            crate::server::queries::MCP_SEARCH_TOOLS_COUNT_SQL.contains("quarantined_at IS NULL")
        );
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
}
