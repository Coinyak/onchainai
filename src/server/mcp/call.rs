//! MCP tools/call dispatch and handlers.

use crate::models::tool::sanitize_tool_for_public_response;
use crate::models::Tool;
use crate::server::functions::{clamp_dashboard_list_limit, fetch_public_dashboard_snapshot};
use crate::server::mcp_search::{
    mcp_search_tools, parse_search_cursor, parse_search_limit, McpSearchSort,
};
use crate::server::queries::{APPROVED_TOOL_BY_SLUG_SQL, CATEGORIES_WITH_COUNTS_SQL};
use crate::server::tool_categories::PUBLIC_TOOL_CATEGORY_IDS;
use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use super::install_guide::mcp_install_guide;

pub(crate) enum ToolsCallOutcome {
    Ok(Value),
    Http(Response),
    Err((i32, String)),
}

async fn gate_tool_payment(
    pool: &PgPool,
    tool_name: &str,
    headers: &HeaderMap,
    okx_client: Option<&std::sync::Arc<x402_core::http::OkxHttpFacilitatorClient>>,
    okx_premium_gate_active: bool,
    // True only on `POST /mcp/okx` (OKX marketplace package). Public `/mcp` is free discovery.
    okx_package_mode: bool,
) -> Result<bool, ToolsCallOutcome> {
    // Hybrid: full OKX $0.1 package applies only on `/mcp/okx`.
    // Premium tools are always paid on every path (never free when config is off).
    let okx_gated = okx_package_mode
        && okx_premium_gate_active
        && okx_client.is_some()
        && crate::server::okx_payment::is_okx_package_tool(tool_name);
    if okx_gated {
        let client = okx_client.expect("okx_gated implies okx_client is Some");
        let description = tool_description_for_okx(tool_name);
        match crate::server::okx_payment::require_okx_payment(
            client,
            tool_name,
            description,
            headers,
        )
        .await
        {
            Ok(_settlement) => return Ok(true),
            Err(response) => return Err(ToolsCallOutcome::Http(response)),
        }
    }

    // Axis B premium (export_toolkit / recommend_verified_tool / gap_audit): always paid.
    // Public /mcp never serves these unpaid — even if admin toggle is off.
    // (On /mcp/okx, the package gate above already charged and returned.)
    if crate::server::mcp_x402::is_premium_mcp_tool(tool_name)
        && !crate::server::okx_payment::should_skip_cdp_for_okx(
            okx_package_mode,
            okx_premium_gate_active,
            tool_name,
        )
    {
        let config = match crate::server::mcp_x402::load_mcp_premium_config(pool).await {
            Ok(config) => config,
            Err(e) => {
                return Err(ToolsCallOutcome::Err((
                    -32603,
                    format!("settings load failed: {e}"),
                )))
            }
        };

        if config.is_active() {
            match crate::server::mcp_x402::require_axis_b_payment(&config, tool_name, headers).await
            {
                Ok(_settlement) => return Ok(false),
                Err(response) => return Err(ToolsCallOutcome::Http(response)),
            }
        }

        // Axis B off: fall back to OKX USDT0 so premium stays paid when OKX creds exist.
        // resource.url must match this public surface (not /mcp/okx).
        if okx_premium_gate_active {
            if let Some(client) = okx_client {
                let description = tool_description_for_okx(tool_name);
                let public_mcp = format!("{}/mcp", crate::config::SITE_ORIGIN);
                match crate::server::okx_payment::require_okx_payment_for_resource(
                    client,
                    tool_name,
                    description,
                    headers,
                    &public_mcp,
                )
                .await
                {
                    Ok(_settlement) => return Ok(true),
                    Err(response) => return Err(ToolsCallOutcome::Http(response)),
                }
            }
        }

        // No payment rail configured — refuse free execution (do not open premium).
        let body = serde_json::json!({
            "error": "mcp_premium_misconfigured",
            "message": format!(
                "Premium tool '{tool_name}' requires payment. Enable MCP premium in admin (pay_to + price) or configure OKX A2MCP."
            ),
        });
        return Err(ToolsCallOutcome::Http(
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(body),
            )
                .into_response(),
        ));
    }
    Ok(false)
}

pub(crate) async fn tools_call(
    pool: &PgPool,
    params: Option<Value>,
    agent: Option<&crate::server::agent_sync::AgentAuth>,
    headers: &HeaderMap,
    okx_client: Option<&std::sync::Arc<x402_core::http::OkxHttpFacilitatorClient>>,
    okx_premium_gate_active: bool,
    okx_package_mode: bool,
) -> ToolsCallOutcome {
    let request = match ToolCallRequest::parse(params) {
        Ok(req) => req,
        Err(err) => return ToolsCallOutcome::Err(err),
    };

    if !is_known_mcp_tool(&request.name) {
        return ToolsCallOutcome::Err((-32601, format!("Unknown tool: {}", request.name)));
    }
    if requires_agent_auth(&request.name) && agent.is_none() {
        return ToolsCallOutcome::Err((
            -32001,
            serde_json::to_string(&crate::server::agent_sync::link_required_payload())
                .unwrap_or_else(|_| "link_required".into()),
        ));
    }

    let okx_gated = match gate_tool_payment(
        pool,
        &request.name,
        headers,
        okx_client,
        okx_premium_gate_active,
        okx_package_mode,
    )
    .await
    {
        Ok(gated) => gated,
        Err(outcome) => return outcome,
    };

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

fn is_known_mcp_tool(name: &str) -> bool {
    matches!(
        name,
        "search_tools"
            | "get_tool_detail"
            | "get_install_guide"
            | "list_categories"
            | "get_dashboard_snapshot"
            | "compare_tools"
            | "get_price_history"
            | "get_x402_trends"
            | "check_endpoint_health"
            | "export_toolkit"
            | "recommend_verified_tool"
            | "gap_audit"
            | "save_to_toolkit"
            | "save_stack_to_blueprint"
            | "link_status"
    )
}

fn requires_agent_auth(name: &str) -> bool {
    matches!(
        name,
        "save_to_toolkit" | "save_stack_to_blueprint" | "link_status"
    )
}

/// Human-readable description for each OKX-gated MCP tool (bundled package).
fn tool_description_for_okx(name: &str) -> &'static str {
    match name {
        "search_tools" => "search crypto tools by keyword, chain, category",
        "get_tool_detail" => "detailed tool info with trust and install safety data",
        "get_install_guide" => "install command and risk assessment for a tool",
        "list_categories" => "list all tool categories in the directory",
        "get_dashboard_snapshot" => "catalog overview: tool counts, x402 stats, chains",
        "compare_tools" => "side-by-side comparison of 2-4 tools",
        "get_price_history" => "x402 pricing history for a tool",
        "get_x402_trends" => "x402 ecosystem trends and statistics",
        "check_endpoint_health" => "x402 endpoint liveness probe with 30-day uptime",
        "export_toolkit" => "export selected tools as a portable toolkit JSON",
        "recommend_verified_tool" => "AI-verified tool recommendation for a given intent",
        "gap_audit" => "catalog gap audit: find missing crypto tool categories",
        "save_to_toolkit" => "save a tool to your personal toolkit",
        "save_stack_to_blueprint" => "save multiple tools to a blueprint",
        "link_status" => "check agent sync connection status",
        _ => "OnchainAI MCP tool",
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

pub(crate) async fn mcp_fetch_public_tool(pool: &PgPool, slug: &str) -> Result<Tool, String> {
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
