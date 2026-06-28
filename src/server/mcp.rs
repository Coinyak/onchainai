//! MCP server — JSON-RPC 2.0 handler with 4 public tools at POST /mcp.

use crate::install_safety::{blocks_structured_config, claude_mcp_config, install_warning_text};
use crate::models::tool::{sanitize_tool_for_public_response, sanitize_tools_for_public_response};
use crate::models::Tool;
use crate::server::functions::{clamp_dashboard_list_limit, fetch_public_dashboard_snapshot};
use crate::server::queries::{
    push_bind_clause, APPROVED_TOOL_BY_SLUG_SQL, CATEGORIES_WITH_COUNTS_SQL,
    MCP_SEARCH_TOOLS_BASE_SQL,
};
use crate::server::rate_limit::{check_mcp_ip_rate_limit, client_ip_from_parts};
use crate::AppState;
use axum::{
    extract::State,
    http::{Request, StatusCode},
    response::IntoResponse,
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
        );
    }

    let body_bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(error_response(Value::Null, -32700, "Parse error")),
            );
        }
    };
    let rpc_req: JsonRpcRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(error_response(Value::Null, -32700, "Parse error")),
            );
        }
    };

    let id = rpc_req.id.clone().unwrap_or(Value::Null);

    if rpc_req.jsonrpc.as_deref() != Some("2.0") {
        return (
            StatusCode::OK,
            Json(error_response(id, -32600, "Invalid Request")),
        );
    }

    let result = match rpc_req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "onchainai", "version": "0.1.0" }
        })),
        "notifications/initialized" => Ok(json!({})),
        "tools/list" => tools_list().await,
        "tools/call" => tools_call(&state.pool, rpc_req.params).await,
        other => Err((-32601, format!("Method not found: {other}"))),
    };

    match result {
        Ok(value) => (StatusCode::OK, Json(ok_response(id, value))),
        Err((code, msg)) => (StatusCode::OK, Json(error_response(id, code, &msg))),
    }
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

async fn tools_list() -> Result<Value, (i32, String)> {
    Ok(json!({
        "tools": [
            {
                "name": "search_tools",
                "description": "Search crypto MCP/CLI/SDK/API tools",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "category": { "type": "string" },
                        "chain": { "type": "string" }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "get_tool_detail",
                "description": "Get detailed info for a tool by slug",
                "inputSchema": {
                    "type": "object",
                    "properties": { "slug": { "type": "string" } },
                    "required": ["slug"]
                }
            },
            {
                "name": "list_categories",
                "description": "List all tool categories with counts",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
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
            },
            {
                "name": "get_install_guide",
                "description": "Platform-specific install guide",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "slug": { "type": "string" },
                        "platform": { "type": "string", "enum": ["claude", "cursor", "generic"] }
                    },
                    "required": ["slug", "platform"]
                }
            }
        ]
    }))
}

async fn tools_call(pool: &PgPool, params: Option<Value>) -> Result<Value, (i32, String)> {
    let params = params.ok_or((-32602, "Missing params".into()))?;
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or((-32602, "Missing tool name".into()))?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let content = match name {
        "search_tools" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or((-32602, "query required".into()))?;
            let category = args
                .get("category")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let chain = args
                .get("chain")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let tools = mcp_search_tools(pool, query, category, chain).await?;
            serde_json::to_string_pretty(&tools)
                .map_err(|e| (-32603, format!("serialize error: {e}")))?
        }
        "get_tool_detail" => {
            let slug = args
                .get("slug")
                .and_then(|v| v.as_str())
                .ok_or((-32602, "slug required".into()))?;
            match mcp_get_tool(pool, slug).await {
                Ok(tool) => serde_json::to_string_pretty(&tool)
                    .map_err(|e| (-32603, format!("serialize error: {e}")))?,
                Err(msg) => return Err((-32000, msg)),
            }
        }
        "list_categories" => {
            let cats = mcp_list_categories(pool).await?;
            serde_json::to_string_pretty(&cats)
                .map_err(|e| (-32603, format!("serialize error: {e}")))?
        }
        "get_dashboard_snapshot" => {
            let limit = args
                .get("limit")
                .and_then(|value| value.as_i64())
                .unwrap_or(6);
            let snapshot = fetch_public_dashboard_snapshot(pool, clamp_dashboard_list_limit(limit))
                .await
                .map_err(|e| (-32603, format!("dashboard snapshot failed: {e}")))?;
            serde_json::to_string_pretty(&snapshot)
                .map_err(|e| (-32603, format!("serialize error: {e}")))?
        }
        "get_install_guide" => {
            let slug = args
                .get("slug")
                .and_then(|v| v.as_str())
                .ok_or((-32602, "slug required".into()))?;
            let platform = args
                .get("platform")
                .and_then(|v| v.as_str())
                .unwrap_or("generic");
            let guide = mcp_install_guide(pool, slug, platform).await?;
            serde_json::to_string_pretty(&guide)
                .map_err(|e| (-32603, format!("serialize error: {e}")))?
        }
        other => return Err((-32601, format!("Unknown tool: {other}"))),
    };

    Ok(json!({
        "content": [{ "type": "text", "text": content }],
        "isError": false
    }))
}

async fn mcp_search_tools(
    pool: &PgPool,
    query: &str,
    category: Option<String>,
    chain: Option<String>,
) -> Result<Vec<Tool>, (i32, String)> {
    let mut sql = MCP_SEARCH_TOOLS_BASE_SQL.to_string();
    let mut idx = 2;
    if category.is_some() {
        push_bind_clause(&mut sql, "AND function =", idx);
        idx += 1;
    }
    if chain.is_some() {
        push_bind_clause(&mut sql, "AND", idx);
        sql.push_str(" = ANY(chains)");
    }
    sql.push_str(" ORDER BY stars DESC LIMIT 50");

    let mut q = sqlx::query_as::<_, Tool>(&sql).bind(query);
    if let Some(c) = &category {
        q = q.bind(c);
    }
    if let Some(ch) = &chain {
        q = q.bind(ch);
    }

    let tools = q
        .fetch_all(pool)
        .await
        .map_err(|e| (-32603, format!("db error: {e}")))?;
    Ok(sanitize_tools_for_public_response(tools))
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

#[derive(Serialize)]
struct ReferralMetadata {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    bps: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payout_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    builder_code: Option<String>,
    payment_verified: bool,
    x402_endpoint_verified: bool,
    price_verified: bool,
}

#[derive(Serialize)]
struct InstallGuide {
    command: String,
    risk_level: String,
    risk_reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    x402_notice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    referral: Option<ReferralMetadata>,
    steps: Vec<String>,
}

fn x402_notice_for_tool(tool: &Tool) -> Option<String> {
    if tool.pricing != "x402" && tool.x402_price.is_none() && !tool.referral_enabled {
        return None;
    }
    let price = tool
        .x402_price
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or("the provider's x402 price");
    let verification =
        if tool.payment_verified && tool.x402_endpoint_verified && tool.price_verified {
            "Payment details are operator verified."
        } else {
            "Payment details are not operator verified yet."
        };
    Some(format!(
        "This tool may request x402 payment ({price}). Connect an agent wallet before calling it. {verification}"
    ))
}

fn referral_metadata_for_tool(tool: &Tool) -> Option<ReferralMetadata> {
    if !tool.referral_enabled {
        return None;
    }
    Some(ReferralMetadata {
        enabled: tool.referral_enabled,
        bps: tool.referral_bps,
        payout_address: tool.referral_payout_address.clone(),
        model: tool.referral_model.clone(),
        builder_code: tool.x402_builder_code.clone(),
        payment_verified: tool.payment_verified,
        x402_endpoint_verified: tool.x402_endpoint_verified,
        price_verified: tool.price_verified,
    })
}

async fn record_referral_event(pool: &PgPool, tool: &Tool, event_type: &str, platform: &str) {
    if !tool.referral_enabled && tool.pricing != "x402" {
        return;
    }
    let metadata = json!({
        "platform": platform,
        "source": "mcp_install_guide",
        "builder_code": tool.x402_builder_code,
    });
    if let Err(error) = sqlx::query(
        r#"
        INSERT INTO referral_events (tool_id, event_type, metadata)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(tool.id)
    .bind(event_type)
    .bind(metadata)
    .execute(pool)
    .await
    {
        tracing::warn!(
            tool_id = %tool.id,
            event_type,
            "failed to record referral event: {error}"
        );
    }
}

async fn mcp_install_guide(
    pool: &PgPool,
    slug: &str,
    platform: &str,
) -> Result<InstallGuide, (i32, String)> {
    if !matches!(platform, "claude" | "cursor" | "generic") {
        return Err((-32602, format!("invalid platform: {platform}")));
    }

    let tool = mcp_fetch_public_tool(pool, slug)
        .await
        .map_err(|m| (-32000, m))?;
    record_referral_event(pool, &tool, "install_guide", platform).await;

    let risk_level = tool.install_risk_level.clone();
    let risk_reasons = tool.install_risk_reasons.clone();
    let warning = install_warning_text(&risk_level).map(str::to_string);
    let blocked = risk_level == "critical";

    let install = tool
        .safe_copy_command
        .clone()
        .or(tool.install_command.clone())
        .unwrap_or_else(|| "No install command available.".into());

    if blocked {
        let x402_notice = x402_notice_for_tool(&tool);
        let referral = referral_metadata_for_tool(&tool);
        return Ok(InstallGuide {
            command: install,
            risk_level,
            risk_reasons,
            warning: Some(
                "Install guidance blocked: critical risk pending operator review.".into(),
            ),
            blocked: true,
            config_json: None,
            x402_notice,
            referral,
            steps: vec![
                "This tool has a critical-risk install command.".into(),
                "Public install guidance is withheld until an operator reviews the listing.".into(),
                "Contact the project directly or wait for operator approval.".into(),
            ],
        });
    }

    let config_blocked = blocks_structured_config(&risk_level);

    let (command, config_json, steps) = match platform {
        "claude" => {
            let config = if config_blocked {
                None
            } else {
                tool.install_command
                    .as_deref()
                    .and_then(|cmd| claude_mcp_config(slug, cmd, &risk_level))
            };
            (
                install.clone(),
                config,
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
        "cursor" => (
            install.clone(),
            if config_blocked {
                None
            } else {
                Some(
                    json!({
                        "mcpServers": {
                            slug: {
                                "command": "npx",
                                "args": ["mcp-remote", "www.onchain-ai.xyz/mcp"]
                            }
                        }
                    })
                    .to_string(),
                )
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
        ),
        _ => (
            install.clone(),
            None,
            vec!["Run the install command in your terminal.".into()],
        ),
    };
    let x402_notice = x402_notice_for_tool(&tool);
    let referral = referral_metadata_for_tool(&tool);

    Ok(InstallGuide {
        command,
        risk_level,
        risk_reasons,
        warning,
        blocked: false,
        config_json,
        x402_notice,
        referral,
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_has_five_tools() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let value = rt.block_on(tools_list()).unwrap();
        let tools = value["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
        assert!(tools
            .iter()
            .any(|tool| tool["name"].as_str() == Some("get_dashboard_snapshot")));
    }

    #[test]
    fn install_guide_includes_risk_fields() {
        let guide = InstallGuide {
            command: "npm i @test/pkg".into(),
            risk_level: "medium".into(),
            risk_reasons: vec!["requires API key".into()],
            warning: Some("Medium-risk install command.".into()),
            blocked: false,
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
    }

    #[test]
    fn install_guide_critical_is_blocked() {
        let guide = InstallGuide {
            command: "rm -rf /".into(),
            risk_level: "critical".into(),
            risk_reasons: vec!["destructive".into()],
            warning: Some("blocked".into()),
            blocked: true,
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
            stars: 0,
            last_commit_at: None,
            source: "manual".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(referral_metadata_for_tool(&tool).is_none());

        tool.referral_enabled = true;
        assert!(referral_metadata_for_tool(&tool).is_some());
    }

    #[test]
    fn install_guide_includes_x402_referral_notice() {
        let guide = InstallGuide {
            command: "npx mcp-remote https://example.com/mcp".into(),
            risk_level: "low".into(),
            risk_reasons: vec![],
            warning: None,
            blocked: false,
            config_json: None,
            x402_notice: Some(
                "This tool may request x402 payment (0.01 USDC). Connect an agent wallet before calling it. Payment details are not operator verified yet.".into(),
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
        assert!(APPROVED_TOOL_BY_SLUG_SQL.contains("relevance_status = 'accepted'"));
        assert!(CATEGORIES_WITH_COUNTS_SQL.contains("quarantined_at IS NULL"));
    }
}
