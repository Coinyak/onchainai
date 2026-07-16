//! MCP server — JSON-RPC 2.0 handler with public tools at POST /mcp.

use crate::server::rate_limit::{check_mcp_ip_rate_limit, client_ip_from_parts};
use crate::AppState;
use axum::http::StatusCode;
use axum::{extract::State, http::Request, response::IntoResponse, Json};
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

/// Billing surface for MCP JSON-RPC.
///
/// Hybrid policy:
/// - [`McpBillingMode::Public`] — site / Claude / Cursor / plugin on `POST /mcp`.
///   Discovery tools are free. Premium trio (`export_toolkit`,
///   `recommend_verified_tool`, `gap_audit`) is always paid ($0.01 USDC Base /
///   Axis B, else OKX fallback, else 503). `check_endpoint_health` uses K2
///   ~$0.001 USDC (CDP env).
/// - [`McpBillingMode::OkxPackage`] — OKX marketplace A2MCP on `POST /mcp/okx`.
///   Every `tools/call` is $0.1 X Layer USDT0 when OKX credentials are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpBillingMode {
    Public,
    OkxPackage,
}

/// Axum handler for free public `POST /mcp` (site Connect hub, plugin, agents).
pub async fn handle_mcp(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
) -> impl IntoResponse {
    handle_mcp_with_mode(state, req, McpBillingMode::Public).await
}

/// Axum handler for paid OKX marketplace package `POST /mcp/okx`.
pub async fn handle_mcp_okx(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
) -> impl IntoResponse {
    handle_mcp_with_mode(state, req, McpBillingMode::OkxPackage).await
}

async fn handle_mcp_with_mode(
    state: AppState,
    req: Request<axum::body::Body>,
    billing: McpBillingMode,
) -> axum::response::Response {
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

    let okx_package_mode = matches!(billing, McpBillingMode::OkxPackage);

    let result = match rpc_req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": negotiate_protocol_version(rpc_req.params.as_ref()),
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": if okx_package_mode { "onchainai-okx" } else { "onchainai" },
                "version": env!("CARGO_PKG_VERSION")
            }
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
                okx_package_mode,
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
    mcp_info_response(McpBillingMode::Public)
}

/// GET `/mcp/okx` — x402 402 challenge when the OKX package gate is active.
///
/// OKX's ASP endpoint review probes the listed URL and flags a 200 answer as
/// "not a valid x402 service" (ASP #4609 rejection, 2026-07-16), so the paid
/// package path must answer plain GETs with the same PAYMENT-REQUIRED
/// challenge as an unpaid `tools/call`. MCP clients are unaffected — they
/// POST JSON-RPC. Falls back to the discovery document when the gate is
/// inactive (local dev without OKX credentials).
pub async fn handle_mcp_okx_info(State(state): State<AppState>) -> impl IntoResponse {
    mcp_okx_get_response(state.okx_premium_gate_active)
}

fn mcp_okx_get_response(okx_gate_active: bool) -> axum::response::Response {
    if okx_gate_active {
        if let Some(challenge) = crate::server::okx_payment::okx_package_probe_response() {
            return challenge;
        }
    }
    mcp_info_response(McpBillingMode::OkxPackage)
}

fn mcp_info_response(billing: McpBillingMode) -> axum::response::Response {
    let tools: Vec<Value> = tool_definitions(false)
        .into_iter()
        .map(|def| {
            json!({
                "name": def.get("name").cloned().unwrap_or(Value::Null),
                "description": def.get("description").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    let (name, description, endpoint, billing_note, billing_detail) = match billing {
        McpBillingMode::Public => (
            "onchainai",
            "OnchainAI public MCP — free discovery for site/Claude/Cursor/plugin agents. search_tools, get_tool_detail, get_install_guide, list_categories, compare_tools, dashboard, get_price_history, get_x402_trends are free. Premium always paid: export_toolkit / recommend_verified_tool / gap_audit = $0.01 USDC Base (Axis B); check_endpoint_health ≈ $0.001 USDC (K2). This path is not a full paywall. For OKX marketplace package pricing use POST /mcp/okx.",
            format!("{}/mcp", crate::config::SITE_ORIGIN),
            "free_discovery",
            json!({
                "mode": "public",
                "discovery": "free",
                "premium_tools": {
                    "names": ["export_toolkit", "recommend_verified_tool", "gap_audit"],
                    "price": crate::server::mcp_x402::DEFAULT_MCP_PREMIUM_PRICE,
                    "network": "eip155:8453",
                    "asset": "USDC",
                    "rail": "axis_b_or_okx_fallback_or_503",
                },
                "check_endpoint_health": {
                    "price": "~$0.001",
                    "asset": "USDC",
                    "rail": "k2_cdp_env",
                },
                "okx_package_endpoint": format!("{}/mcp/okx", crate::config::SITE_ORIGIN),
            }),
        ),
        McpBillingMode::OkxPackage => (
            "onchainai-okx",
            "OnchainAI OKX A2MCP package — same tool set as public MCP; every tools/call is pay-per-call ($0.1 USDT0 on X Layer) via OKX Broker when the gate is active. Site/plugin agents should use free POST /mcp instead.",
            format!("{}/mcp/okx", crate::config::SITE_ORIGIN),
            "okx_package_pay_per_call",
            json!({
                "mode": "okx_package",
                "every_tools_call": {
                    "price": "$0.1",
                    "network": "eip155:196",
                    "asset": "USDT0",
                    "rail": "okx_broker_when_active",
                },
                "public_free_endpoint": format!("{}/mcp", crate::config::SITE_ORIGIN),
            }),
        ),
    };
    (
        StatusCode::OK,
        Json(json!({
            "name": name,
            "version": env!("CARGO_PKG_VERSION"),
            "description": description,
            "protocolVersion": DEFAULT_PROTOCOL_VERSION,
            "endpoint": endpoint,
            "billing": billing_note,
            "billing_detail": billing_detail,
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
mod tests;
