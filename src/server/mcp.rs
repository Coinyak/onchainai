//! MCP server — JSON-RPC 2.0 handler with 4 public tools at POST /mcp.

use crate::models::tool::sanitize_tool_for_public_response;
use crate::models::Tool;
use crate::server::functions::{clamp_dashboard_list_limit, fetch_public_dashboard_snapshot};
use crate::server::mcp_search::{
    mcp_search_tools, parse_search_cursor, parse_search_limit, McpSearchSort,
};
use crate::server::queries::{APPROVED_TOOL_BY_SLUG_SQL, CATEGORIES_WITH_COUNTS_SQL};
use crate::server::rate_limit::{check_mcp_ip_rate_limit, client_ip_from_parts};
use crate::server::tool_categories::PUBLIC_TOOL_CATEGORY_IDS;
use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;

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

async fn tools_list(authenticated: bool) -> Result<Value, (i32, String)> {
    Ok(json!({ "tools": tool_definitions(authenticated) }))
}

fn tool_definitions(authenticated: bool) -> Vec<Value> {
    let mut tools = vec![
        search_tools_definition(),
        get_tool_detail_definition(),
        list_categories_definition(),
        get_dashboard_snapshot_definition(),
        get_install_guide_definition(),
        check_endpoint_health_definition(),
        compare_tools_definition(),
        export_toolkit_definition(),
        recommend_verified_tool_definition(),
        gap_audit_definition(),
        get_price_history_definition(),
        get_x402_trends_definition(),
    ];
    if authenticated {
        tools.push(save_to_toolkit_definition());
        tools.push(save_stack_to_blueprint_definition());
        tools.push(link_status_definition());
    }
    tools
}

fn save_to_toolkit_definition() -> Value {
    json!({
        "name": "save_to_toolkit",
        "description": "Save a tool to the linked user's OnchainAI toolkit. Requires Agent Sync link (Bearer token). Use only when the user explicitly asks to save or add to toolkit.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "slug": {
                    "type": "string",
                    "description": "Tool slug from search_tools or get_tool_detail"
                },
                "note": {
                    "type": "string",
                    "description": "Optional short note (max 500 chars); does not overwrite existing user notes"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional tags (max 8)"
                }
            },
            "required": ["slug"]
        }
    })
}

fn save_stack_to_blueprint_definition() -> Value {
    json!({
        "name": "save_stack_to_blueprint",
        "description": "Save multiple tools to the linked user's toolkit and append them to today's agent session blueprint. Requires Agent Sync link. Use when the user explicitly asks to save a stack or workflow.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "slugs": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Tool slugs to save (max 25)"
                },
                "title": {
                    "type": "string",
                    "description": "Optional blueprint title; defaults to Agent session · {date}"
                }
            },
            "required": ["slugs"]
        }
    })
}

fn link_status_definition() -> Value {
    json!({
        "name": "link_status",
        "description": "Check whether the MCP client is linked to an OnchainAI account.",
        "inputSchema": { "type": "object", "properties": {} }
    })
}

fn search_tools_definition() -> Value {
    json!({
        "name": "search_tools",
        "description": "Search OnchainAI for crypto/onchain MCP, CLI, SDK, API, x402, and AI-agent tools by capability. Use when you need to find or compare tools for a task. Examples: bridge USDC to Base, Uniswap MCP server, Solana wallet SDK. For browsing by function, call list_categories first and pass the returned id as category.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural-language capability, package, protocol, or tool name to search for"
                },
                "category": {
                    "type": "string",
                    "enum": PUBLIC_TOOL_CATEGORY_IDS,
                    "description": "Optional OnchainAI function filter. Use list_categories ids (bridge, swap, wallet, payments, lending, staking, trading, nft, data, dev-tool, identity, governance, social, ai-agent)."
                },
                "chain": {
                    "type": "string",
                    "description": "Optional chain filter, such as base, ethereum, solana, arbitrum, or bitcoin"
                },
                "sort": {
                    "type": "string",
                    "enum": ["relevance", "trust", "stars", "recent"],
                    "description": "Ranking strategy. trust currently sorts by stars until a dedicated trust signal ships. Defaults to relevance."
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 25,
                    "description": "Maximum number of tools to return; defaults to 10"
                },
                "cursor": {
                    "type": "string",
                    "description": "Pagination offset string from the previous next_cursor (e.g. \"10\", \"20\"). Omit or pass \"0\" for the first page."
                }
            },
            "required": ["query"]
        }
    })
}

fn get_tool_detail_definition() -> Value {
    json!({
        "name": "get_tool_detail",
        "description": "Get full detail (install risk, x402 verification flags, chains, repo) for a tool by slug. Use the slug from search_tools results. Call before get_install_guide to verify trust status, x402, and install risk.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "slug": {
                    "type": "string",
                    "description": "Tool slug from search_tools results"
                }
            },
            "required": ["slug"]
        }
    })
}

fn list_categories_definition() -> Value {
    json!({
        "name": "list_categories",
        "description": "List all tool categories with counts. Use for browsing what exists on OnchainAI. Pass the returned id as search_tools category to filter by function.",
        "inputSchema": { "type": "object", "properties": {} }
    })
}

fn get_dashboard_snapshot_definition() -> Value {
    json!({
        "name": "get_dashboard_snapshot",
        "description": "Public no-login snapshot of OnchainAI tool coverage, categories, trust, x402, and featured tool lists",
        "inputSchema": {
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 12,
                    "description": "Maximum tools or buckets per section"
                }
            }
        }
    })
}

fn check_endpoint_health_definition() -> Value {
    json!({
        "name": "check_endpoint_health",
        "description": "Premium x402 trust data: endpoint liveness, 30-day probe uptime, and last probe time for a listed x402 tool. Requires x402 micropayment per call (HTTP 402 + PAYMENT-REQUIRED header). Standard MCP clients (Claude Code, Cursor) cannot complete payment and may show a connection error — use free get_tool_detail for x402 verification flags, or GET /api/v2/premium/check-endpoint-health/{slug} with an x402-capable HTTP client.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "slug": {
                    "type": "string",
                    "description": "Tool slug from search_tools — must be an x402-listed tool"
                }
            },
            "required": ["slug"]
        }
    })
}

fn compare_tools_definition() -> Value {
    json!({
        "name": "compare_tools",
        "description": "Compare 2–4 approved tools side-by-side on trust, install risk, chains, pricing, and x402 status. May require x402 payment per call when OnchainAI MCP premium is enabled — HTTP 402 returns PAYMENT-REQUIRED on POST /mcp. Free alternative: call get_tool_detail for each slug.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "slugs": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 2,
                    "maxItems": 4,
                    "description": "Tool slugs to compare (2–4 unique)"
                }
            },
            "required": ["slugs"]
        }
    })
}

fn export_toolkit_definition() -> Value {
    json!({
        "name": "export_toolkit",
        "description": "Export a bundle of approved tools as JSON + markdown install kit for agents. Pass slugs or a function category id. May require x402 payment per call when OnchainAI MCP premium is enabled — HTTP 402 returns PAYMENT-REQUIRED on POST /mcp.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "slugs": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Explicit tool slugs to export (max 25)"
                },
                "category": {
                    "type": "string",
                    "enum": PUBLIC_TOOL_CATEGORY_IDS,
                    "description": "Alternatively export top tools for a function category"
                }
            }
        }
    })
}

fn recommend_verified_tool_definition() -> Value {
    json!({
        "name": "recommend_verified_tool",
        "description": "Premium: returns a single verified live x402 tool for a task. Probes top candidates on-demand for liveness and price honesty, then returns the best one with rejection reasons for the rest. Requires x402 micropayment per call (HTTP 402 + PAYMENT-REQUIRED header). Use free search_tools first to check if candidates exist.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "intent": {
                    "type": "string",
                    "description": "Natural-language task intent (e.g. 'bridge USDC to Base', 'get Ethereum price data via x402')"
                },
                "chain": {
                    "type": "string",
                    "description": "Optional chain filter (e.g. base, ethereum, solana)"
                },
                "function": {
                    "type": "string",
                    "description": "Optional OnchainAI function filter (bridge, swap, wallet, payments, lending, staking, trading, nft, data, dev-tool, identity, governance, social, ai-agent)"
                }
            },
            "required": ["intent"]
        }
    })
}

fn gap_audit_definition() -> Value {
    json!({
        "name": "gap_audit",
        "description": "Premium: decomposes a task intent into subgoals and maps each to OnchainAI catalog tools, surfacing gaps where no tools exist. Returns a subgoal table with covered (candidate slugs) or gap (manual research needed) status. Requires x402 micropayment per call (HTTP 402 + PAYMENT-REQUIRED header). Use free search_tools for simple lookups.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "intent": {
                    "type": "string",
                    "description": "Natural-language task intent (e.g. 'bridge BTC to Base then swap to USDC and stake')"
                }
            },
            "required": ["intent"]
        }
    })
}

fn get_price_history_definition() -> Value {
    json!({
        "name": "get_price_history",
        "description": "Free discovery: x402 endpoint price and liveness history for a specific tool. Returns probe records (status, actual price, latency) over the specified time window. Use get_tool_detail for current x402 flags.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "slug": {
                    "type": "string",
                    "description": "Tool slug from search_tools — must be an x402-listed tool"
                },
                "days": {
                    "type": "integer",
                    "description": "Number of days of history (default 30, max 90)"
                }
            },
            "required": ["slug"]
        }
    })
}

fn get_x402_trends_definition() -> Value {
    json!({
        "name": "get_x402_trends",
        "description": "Free discovery: aggregated x402 ecosystem trends — live rate, probe counts, and latest prices for all x402 tools.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "days": {
                    "type": "integer",
                    "description": "Number of days to aggregate (default 30, max 90)"
                }
            }
        }
    })
}

fn get_install_guide_definition() -> Value {
    json!({
        "name": "get_install_guide",
        "description": "Get platform-specific install guide. Pass slug from search_tools or get_tool_detail and platform (claude, cursor, generic). If blocked=true or risk_level=critical, do not install.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "slug": {
                    "type": "string",
                    "description": "Tool slug from search_tools or get_tool_detail — do not guess slugs"
                },
                "platform": {
                    "type": "string",
                    "enum": ["claude", "cursor", "generic"],
                    "description": "Target agent environment for install steps"
                }
            },
            "required": ["slug", "platform"]
        }
    })
}

enum ToolsCallOutcome {
    Ok(Value),
    Http(Response),
    Err((i32, String)),
}

async fn tools_call(
    pool: &PgPool,
    params: Option<Value>,
    agent: Option<&crate::server::agent_sync::AgentAuth>,
    headers: &HeaderMap,
    okx_client: Option<&std::sync::Arc<x402_core::http::OkxHttpFacilitatorClient>>,
    okx_premium_gate_active: bool,
) -> ToolsCallOutcome {
    let request = match ToolCallRequest::parse(params) {
        Ok(req) => req,
        Err(err) => return ToolsCallOutcome::Err(err),
    };

    // OKX handler-level gate: when OKX is active, premium tools in OKX_GATED_ROUTES
    // are gated through OKX Broker on X Layer USDT0. Discovery tools stay on the
    // direct MCP path (currently free — operator may move any tool to premium).
    let okx_gated = okx_premium_gate_active
        && okx_client.is_some()
        && crate::server::okx_payment::OKX_GATED_ROUTES.contains(&request.name.as_str());

    if okx_gated {
        let client = okx_client.expect("okx_gated implies okx_client is Some");
        let description = tool_description_for_okx(&request.name);
        match crate::server::okx_payment::require_okx_payment(
            client,
            &request.name,
            description,
            headers,
        )
        .await
        {
            Ok(_settlement) => {}
            Err(response) => return ToolsCallOutcome::Http(response),
        }
    } else if crate::server::mcp_x402::is_premium_mcp_tool(&request.name) {
        // CDP/Base fallback: operator-toggled x402 via site_settings.
        // Discovery tools (compare_tools, search_tools, etc.) are currently free —
        // not in PREMIUM_MCP_TOOLS. Operator may move any tool to premium set.
        // Default disabled, so export_toolkit stays free until explicitly enabled.
        // Skipped when OKX is active for this tool (prevents double-charge).
        if !crate::server::okx_payment::should_skip_cdp_for_okx(
            okx_premium_gate_active,
            &request.name,
        ) {
            let config = match crate::server::mcp_x402::load_mcp_premium_config(pool).await {
                Ok(config) => config,
                Err(e) => {
                    return ToolsCallOutcome::Err((-32603, format!("settings load failed: {e}")))
                }
            };
            if config.is_active() {
                match crate::server::mcp_x402::require_axis_b_payment(
                    &config,
                    &request.name,
                    headers,
                )
                .await
                {
                    Ok(_settlement) => {}
                    Err(response) => return ToolsCallOutcome::Http(response),
                }
            }
        }
    }
    match dispatch_tool_call(
        pool,
        &request.name,
        &request.args,
        agent,
        headers,
        okx_gated,
    )
    .await
    {
        Ok(DispatchOutcome::Text(content)) => {
            ToolsCallOutcome::Ok(tool_call_text_response(content))
        }
        Ok(DispatchOutcome::Http(response)) => ToolsCallOutcome::Http(response),
        Err((code, msg)) => ToolsCallOutcome::Err((code, msg)),
    }
}

/// Human-readable description for each OKX-gated premium tool.
fn tool_description_for_okx(name: &str) -> &'static str {
    match name {
        "check_endpoint_health" => "x402 endpoint liveness probe with 30-day uptime",
        "export_toolkit" => "export selected tools as a portable toolkit JSON",
        "recommend_verified_tool" => "AI-verified tool recommendation for a given intent",
        "gap_audit" => "catalog gap audit: find missing crypto tool categories",
        _ => "OnchainAI premium MCP tool",
    }
}

enum DispatchOutcome {
    Text(String),
    Http(Response),
}

struct ToolCallRequest {
    name: String,
    args: Value,
}

impl ToolCallRequest {
    fn parse(params: Option<Value>) -> Result<Self, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".into()))?;
        let name = required_str(&params, "name", "Missing tool name")?.to_string();
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        Ok(Self { name, args })
    }
}

async fn dispatch_tool_call(
    pool: &PgPool,
    name: &str,
    args: &Value,
    agent: Option<&crate::server::agent_sync::AgentAuth>,
    headers: &HeaderMap,
    payment_already_gated: bool,
) -> Result<DispatchOutcome, (i32, String)> {
    match name {
        "search_tools" => call_search_tools(pool, args)
            .await
            .map(DispatchOutcome::Text),
        "get_tool_detail" => call_get_tool_detail(pool, args)
            .await
            .map(DispatchOutcome::Text),
        "list_categories" => call_list_categories(pool).await.map(DispatchOutcome::Text),
        "get_dashboard_snapshot" => call_dashboard_snapshot(pool, args)
            .await
            .map(DispatchOutcome::Text),
        "get_install_guide" => call_install_guide(pool, args)
            .await
            .map(DispatchOutcome::Text),
        "check_endpoint_health" => {
            call_check_endpoint_health(pool, args, headers, payment_already_gated).await
        }
        "compare_tools" => call_compare_tools(pool, args)
            .await
            .map(DispatchOutcome::Text),
        "export_toolkit" => call_export_toolkit(pool, args)
            .await
            .map(DispatchOutcome::Text),
        "recommend_verified_tool" => call_recommend_verified_tool(pool, args, headers).await,
        "gap_audit" => call_gap_audit(pool, args, headers).await,
        "get_price_history" => call_get_price_history(pool, args, headers).await,
        "get_x402_trends" => call_get_x402_trends(pool, args, headers).await,
        "save_to_toolkit" => call_save_to_toolkit(pool, args, agent)
            .await
            .map(DispatchOutcome::Text),
        "save_stack_to_blueprint" => call_save_stack_to_blueprint(pool, args, agent)
            .await
            .map(DispatchOutcome::Text),
        "link_status" => call_link_status(agent).await.map(DispatchOutcome::Text),
        other => Err((-32601, format!("Unknown tool: {other}"))),
    }
}

async fn call_check_endpoint_health(
    pool: &PgPool,
    args: &Value,
    headers: &HeaderMap,
    payment_already_gated: bool,
) -> Result<DispatchOutcome, (i32, String)> {
    use crate::server::x402_payment::{
        facilitator_client, payment_success_response, require_payment, PaymentSettlement,
        X402PaymentConfig,
    };
    use crate::server::x402_premium::{check_endpoint_health, PremiumDataError};

    let slug = required_str(args, "slug", "slug required")?;

    // Skip CDP gate when OKX handler-level gate already verified payment.
    let settlement = if payment_already_gated {
        PaymentSettlement {
            payer: None,
            transaction: None,
        }
    } else {
        let config = X402PaymentConfig::from_env();
        let resource_url = format!("/api/v2/premium/check-endpoint-health/{slug}");
        let requirements = config.requirement_for(
            &resource_url,
            "x402 endpoint liveness, 30-day uptime, and last probe timestamp",
            "application/json",
        );
        let client = facilitator_client();
        match require_payment(
            &client,
            &config,
            headers,
            requirements,
            Some(crate::server::x402_payment::CHECK_ENDPOINT_HEALTH_PAYMENT_HINT),
        )
        .await
        {
            Ok(s) => s,
            Err(resp) => return Ok(DispatchOutcome::Http(resp)),
        }
    };

    let report = match check_endpoint_health(pool, slug).await {
        Ok(report) => report,
        Err(PremiumDataError::NotFound) => {
            return Err((-32602, "tool not found".into()));
        }
        Err(PremiumDataError::NotX402) => {
            return Err((-32602, "tool is not an x402 endpoint listing".into()));
        }
        Err(PremiumDataError::MissingEndpoint) => {
            return Err((-32602, "tool has no x402 endpoint URL".into()));
        }
        Err(PremiumDataError::InvalidSlug) => {
            return Err((-32602, "slug is required".into()));
        }
        Err(PremiumDataError::Database(e)) => {
            return Err((-32000, format!("database error: {e}")));
        }
    };

    let price_display = if payment_already_gated {
        crate::server::okx_payment::okx_price_display()
    } else {
        X402PaymentConfig::from_env().price_display
    };
    let body = json!({
        "data": report,
        "payment": {
            "payer": settlement.payer,
            "transaction": settlement.transaction,
            "price": price_display,
        }
    });
    let response = payment_success_response(body.clone(), &settlement)
        .unwrap_or_else(|_| (StatusCode::OK, Json(body)).into_response());
    Ok(DispatchOutcome::Http(response))
}

/// Product A — `recommend_verified_tool`: Axis-B premium (same gate as export_toolkit).
/// Extracts candidates via free search, probes top N on-demand, returns the best verified tool.
async fn call_recommend_verified_tool(
    pool: &PgPool,
    args: &Value,
    _headers: &HeaderMap,
) -> Result<DispatchOutcome, (i32, String)> {
    use crate::server::mcp_search::{mcp_search_tools, McpSearchSort};
    use crate::server::product_a::{
        cache_get, cache_key, cache_set, recommend_verified_tool, validate_intent, ProductAError,
    };

    let intent_raw = required_str(args, "intent", "intent required")?;
    let intent = validate_intent(intent_raw).map_err(|e| (-32602, e.message().to_string()))?;
    let chain = optional_string(args, "chain");
    let function = optional_string(args, "function");

    // Payment gate runs in tools_call() before dispatch; cache hits reuse prior paid work.
    let now = chrono::Utc::now();
    let ckey = cache_key(&intent, chain.as_deref(), function.as_deref());
    if let Some(cached) = cache_get(&ckey, now) {
        let body = json!(cached);
        return Ok(DispatchOutcome::Text(
            serde_json::to_string(&body).map_err(|e| (-32000, format!("serialize error: {e}")))?,
        ));
    }

    // Step 1: free search to extract candidates (reuse mcp_search_tools).
    let search_page = mcp_search_tools(
        pool,
        &intent,
        function.clone(),
        chain.clone(),
        McpSearchSort::Trust,
        10,
        0,
    )
    .await?;

    let candidate_slugs: Vec<String> = search_page.tools.iter().map(|t| t.slug.clone()).collect();

    if candidate_slugs.is_empty() {
        let response = crate::server::product_a::ProductAResponse {
            verified_tool: None,
            rejected: vec![],
            disclaimer: crate::server::product_a::PRODUCT_A_DISCLAIMER,
            probed_at: now,
            cached: None,
        };
        cache_set(ckey, response.clone(), now);
        let body = json!(response);
        return Ok(DispatchOutcome::Text(
            serde_json::to_string(&body).map_err(|e| (-32000, format!("serialize error: {e}")))?,
        ));
    }

    // Step 2-7: rank, probe, select, explain_rejection.
    let result = recommend_verified_tool(pool, &candidate_slugs)
        .await
        .map_err(|e| match e {
            ProductAError::InvalidIntent => (-32602, e.message().to_string()),
            ProductAError::NoCandidates => (-32602, e.message().to_string()),
            ProductAError::Database(err) => (-32000, format!("database error: {err}")),
        })?;

    cache_set(ckey, result.clone(), now);
    let body = json!(result);
    Ok(DispatchOutcome::Text(
        serde_json::to_string(&body).map_err(|e| (-32000, format!("serialize error: {e}")))?,
    ))
}

/// S0 gap_audit — Axis-B premium (same gate as export_toolkit).
/// Decomposes intent into subgoals, maps each to catalog, surfaces gaps.
async fn call_gap_audit(
    pool: &PgPool,
    args: &Value,
    _headers: &HeaderMap,
) -> Result<DispatchOutcome, (i32, String)> {
    use crate::server::gap_audit::{
        gap_cache_get, gap_cache_key, gap_cache_set, run_gap_audit, validate_gap_audit_intent,
        GapAuditError,
    };

    let intent_raw = required_str(args, "intent", "intent required")?;
    let intent =
        validate_gap_audit_intent(intent_raw).map_err(|e| (-32602, e.message().to_string()))?;

    // Payment gate runs in tools_call() before dispatch; cache hits reuse prior paid work.
    let now = chrono::Utc::now();
    let ckey = gap_cache_key(&intent);
    if let Some(cached) = gap_cache_get(&ckey, now) {
        let body = json!(cached);
        return Ok(DispatchOutcome::Text(
            serde_json::to_string(&body).map_err(|e| (-32000, format!("serialize error: {e}")))?,
        ));
    }

    match run_gap_audit(pool, &intent).await {
        Ok(result) => {
            gap_cache_set(ckey, result.clone(), now);
            let body = json!(result);
            Ok(DispatchOutcome::Text(
                serde_json::to_string(&body)
                    .map_err(|e| (-32000, format!("serialize error: {e}")))?,
            ))
        }
        Err(GapAuditError::InvalidIntent) => {
            Err((-32602, GapAuditError::InvalidIntent.message().to_string()))
        }
        Err(GapAuditError::Database(msg)) => Err((-32000, format!("database error: {msg}"))),
    }
}

/// M3 get_price_history — free discovery/metadata (currently free, operator-discretion).
async fn call_get_price_history(
    pool: &PgPool,
    args: &Value,
    _headers: &HeaderMap,
) -> Result<DispatchOutcome, (i32, String)> {
    use crate::server::m3_analytics::{get_price_history, AnalyticsError};

    let slug = required_str(args, "slug", "slug required")?;
    let days = args.get("days").and_then(|v| v.as_i64());

    match get_price_history(pool, slug, days).await {
        Ok(result) => {
            let body = json!(result);
            Ok(DispatchOutcome::Text(
                serde_json::to_string(&body)
                    .map_err(|e| (-32000, format!("serialize error: {e}")))?,
            ))
        }
        Err(AnalyticsError::NotFound) => Err((-32602, "tool not found".into())),
        Err(AnalyticsError::NotX402) => Err((-32602, "tool is not an x402 listing".into())),
        Err(AnalyticsError::Database(e)) => Err((-32000, format!("database error: {e}"))),
    }
}

/// M3 get_x402_trends — free discovery/metadata (currently free, operator-discretion).
async fn call_get_x402_trends(
    pool: &PgPool,
    args: &Value,
    _headers: &HeaderMap,
) -> Result<DispatchOutcome, (i32, String)> {
    use crate::server::m3_analytics::{get_x402_trends, AnalyticsError};

    let days = args.get("days").and_then(|v| v.as_i64());

    match get_x402_trends(pool, days).await {
        Ok(result) => {
            let body = json!(result);
            Ok(DispatchOutcome::Text(
                serde_json::to_string(&body)
                    .map_err(|e| (-32000, format!("serialize error: {e}")))?,
            ))
        }
        Err(AnalyticsError::Database(e)) => Err((-32000, format!("database error: {e}"))),
        Err(e) => Err((-32000, e.message().to_string())),
    }
}

async fn call_compare_tools(pool: &PgPool, args: &Value) -> Result<String, (i32, String)> {
    let slugs = args
        .get("slugs")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|joined| !joined.is_empty())
        .ok_or((-32602, "slugs required (2–4 items)".into()))?;
    crate::server::mcp_premium_tools::mcp_compare_tools(pool, &slugs)
        .await
        .map_err(|msg| (-32000, msg))
}

async fn call_export_toolkit(pool: &PgPool, args: &Value) -> Result<String, (i32, String)> {
    let slugs = args.get("slugs").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect::<Vec<_>>()
    });
    let category = optional_string(args, "category");
    crate::server::mcp_premium_tools::mcp_export_toolkit(pool, slugs, category.as_deref())
        .await
        .map_err(|msg| (-32000, msg))
}

async fn call_save_to_toolkit(
    pool: &PgPool,
    args: &Value,
    agent: Option<&crate::server::agent_sync::AgentAuth>,
) -> Result<String, (i32, String)> {
    let Some(auth) = agent else {
        return Err((
            -32001,
            serde_json::to_string(&crate::server::agent_sync::link_required_payload())
                .unwrap_or_else(|_| "link_required".into()),
        ));
    };
    let slug = required_str(args, "slug", "slug required")?;
    let note = optional_string(args, "note");
    let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect::<Vec<_>>()
    });
    let req = crate::server::agent_sync::SyncToolRequest {
        slug: slug.to_string(),
        note,
        tags,
        source_client: Some("mcp".into()),
        idempotency_key: None,
    };
    let result = crate::server::agent_sync::sync_tool(pool, auth, req)
        .await
        .map_err(|e| (-32000, e.to_string()))?;
    serialize_tool_payload(&result)
}

async fn call_save_stack_to_blueprint(
    pool: &PgPool,
    args: &Value,
    agent: Option<&crate::server::agent_sync::AgentAuth>,
) -> Result<String, (i32, String)> {
    let Some(auth) = agent else {
        return Err((
            -32001,
            serde_json::to_string(&crate::server::agent_sync::link_required_payload())
                .unwrap_or_else(|_| "link_required".into()),
        ));
    };
    let slugs = args
        .get("slugs")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .ok_or((-32602, "slugs required".into()))?;
    let title = optional_string(args, "title");
    let result = crate::server::agent_sync::save_stack_to_blueprint(pool, auth, slugs, title)
        .await
        .map_err(|e| (-32000, e.to_string()))?;
    serialize_tool_payload(&result)
}

async fn call_link_status(
    agent: Option<&crate::server::agent_sync::AgentAuth>,
) -> Result<String, (i32, String)> {
    let Some(auth) = agent else {
        return serialize_tool_payload(&serde_json::json!({ "linked": false }));
    };
    serialize_tool_payload(&serde_json::json!({
        "linked": true,
        "user_id_prefix": &auth.user_id.to_string()[..8],
        "client": auth.client,
    }))
}

async fn call_search_tools(pool: &PgPool, args: &Value) -> Result<String, (i32, String)> {
    let query = required_str(args, "query", "query required")?;
    let category = optional_string(args, "category");
    let chain = optional_string(args, "chain");
    let sort = parse_mcp_sort(args)?;
    let limit = parse_search_limit(args.get("limit"));
    let cursor = parse_search_cursor(args.get("cursor"))?;
    let page = mcp_search_tools(pool, query, category, chain, sort, limit, cursor).await?;
    serialize_tool_payload(&page)
}

async fn call_get_tool_detail(pool: &PgPool, args: &Value) -> Result<String, (i32, String)> {
    use crate::models::tool::PublicTool;
    use crate::server::trust_probe_meta::PublicToolWithTrustProbe;
    let slug = required_str(args, "slug", "slug required")?;
    let tool = mcp_get_tool(pool, slug)
        .await
        .map_err(|msg| (-32000, msg))?;
    let trust_probe = crate::server::trust_probe_meta::stale_trust_badge_for_tool(pool, &tool)
        .await
        .map_err(|e| (-32000, format!("trust probe meta failed: {e}")))?;
    let official_links =
        crate::server::review_persistence::list_public_official_links(pool, tool.id)
            .await
            .map_err(|e| (-32000, format!("official links failed: {e}")))?;
    let payload = PublicToolWithTrustProbe {
        tool: PublicTool::from(tool),
        official_links,
        trust_probe,
    };
    serialize_tool_payload(&payload)
}

async fn call_list_categories(pool: &PgPool) -> Result<String, (i32, String)> {
    let categories = mcp_list_categories(pool).await?;
    serialize_tool_payload(&categories)
}

async fn call_dashboard_snapshot(pool: &PgPool, args: &Value) -> Result<String, (i32, String)> {
    let limit = args
        .get("limit")
        .and_then(|value| value.as_i64())
        .unwrap_or(6);
    let snapshot = fetch_public_dashboard_snapshot(pool, clamp_dashboard_list_limit(limit))
        .await
        .map_err(|e| (-32603, format!("dashboard snapshot failed: {e}")))?;
    serialize_tool_payload(&snapshot)
}

async fn call_install_guide(pool: &PgPool, args: &Value) -> Result<String, (i32, String)> {
    let slug = required_str(args, "slug", "slug required")?;
    let platform = args
        .get("platform")
        .and_then(|value| value.as_str())
        .unwrap_or("generic");
    let guide = mcp_install_guide(pool, slug, platform).await?;
    serialize_tool_payload(&guide)
}

fn parse_mcp_sort(args: &Value) -> Result<McpSearchSort, (i32, String)> {
    McpSearchSort::parse(
        args.get("sort")
            .and_then(|value| value.as_str())
            .unwrap_or(McpSearchSort::Relevance.as_str()),
    )
}

fn required_str<'a>(args: &'a Value, key: &str, message: &str) -> Result<&'a str, (i32, String)> {
    args.get(key)
        .and_then(|value| value.as_str())
        .ok_or((-32602, message.into()))
}

fn optional_string(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn serialize_tool_payload(payload: &impl Serialize) -> Result<String, (i32, String)> {
    serde_json::to_string(payload).map_err(|e| (-32603, format!("serialize error: {e}")))
}

fn tool_call_text_response(content: String) -> Value {
    json!({
        "content": [{ "type": "text", "text": content }],
        "isError": false
    })
}

async fn mcp_fetch_public_tool(pool: &PgPool, slug: &str) -> Result<Tool, String> {
    sqlx::query_as::<_, Tool>(APPROVED_TOOL_BY_SLUG_SQL)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("db error: {e}"))?
        .ok_or_else(|| format!("tool not found: {slug}"))
}

async fn mcp_get_tool(pool: &PgPool, slug: &str) -> Result<Tool, String> {
    let tool = mcp_fetch_public_tool(pool, slug).await?;
    Ok(sanitize_tool_for_public_response(tool))
}

#[derive(Serialize)]
struct CategoryMcp {
    id: String,
    label: String,
    icon: String,
    description: String,
    tool_count: i64,
}

async fn mcp_list_categories(pool: &PgPool) -> Result<Vec<CategoryMcp>, (i32, String)> {
    let rows = sqlx::query_as::<_, crate::server::functions::CategoryWithCount>(
        CATEGORIES_WITH_COUNTS_SQL,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| (-32603, format!("db error: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|r| CategoryMcp {
            id: r.id,
            label: r.label,
            icon: r.icon,
            description: r.description,
            tool_count: r.count,
        })
        .collect())
}

mod auth;
mod install_guide;

use auth::agent_from_authorization;
use install_guide::mcp_install_guide;

#[cfg(test)]
mod tests {
    use super::*;
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
