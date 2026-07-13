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
mod tests;
