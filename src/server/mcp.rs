//! MCP server — JSON-RPC 2.0 handler with 4 public tools at POST /mcp.

use crate::models::Tool;
use crate::server::queries::TOOLS_APPROVED_WHERE;
use crate::AppState;
use axum::{
    extract::State,
    http::StatusCode,
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
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let id = req.id.clone().unwrap_or(Value::Null);

    if req.jsonrpc.as_deref() != Some("2.0") {
        return (
            StatusCode::OK,
            Json(error_response(id, -32600, "Invalid Request")),
        );
    }

    let result = match req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "onchainai", "version": "0.1.0" }
        })),
        "notifications/initialized" => Ok(json!({})),
        "tools/list" => tools_list().await,
        "tools/call" => tools_call(&state.pool, req.params).await,
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
            let category = args.get("category").and_then(|v| v.as_str()).map(str::to_string);
            let chain = args.get("chain").and_then(|v| v.as_str()).map(str::to_string);
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
    let mut sql = format!(
        r#"
        SELECT * FROM tools
        WHERE {TOOLS_APPROVED_WHERE}
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
              @@ plainto_tsquery('english', $1)
        "#
    );
    if category.is_some() {
        sql.push_str(" AND function = $2");
    }
    if chain.is_some() {
        sql.push_str(" AND $3 = ANY(chains)");
    }
    sql.push_str(" ORDER BY stars DESC LIMIT 50");

    let mut q = sqlx::query_as::<_, Tool>(&sql).bind(query);
    if let Some(c) = &category {
        q = q.bind(c);
    }
    if let Some(ch) = &chain {
        q = q.bind(ch);
    }

    q.fetch_all(pool)
        .await
        .map_err(|e| (-32603, format!("db error: {e}")))
}

async fn mcp_get_tool(pool: &PgPool, slug: &str) -> Result<Tool, String> {
    let sql = format!("SELECT * FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}");
    sqlx::query_as::<_, Tool>(&sql)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("db error: {e}"))?
        .ok_or_else(|| format!("tool not found: {slug}"))
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
    let sql = format!(
        r#"
        SELECT c.id, c.label, c.icon, c.description, c.sort_order,
               COUNT(t.id) AS count
        FROM categories c
        LEFT JOIN tools t ON t.function = c.id AND t.{TOOLS_APPROVED_WHERE}
        GROUP BY c.id, c.label, c.icon, c.description, c.sort_order
        ORDER BY c.sort_order ASC
        "#
    );
    let rows = sqlx::query_as::<_, crate::server::functions::CategoryWithCount>(&sql)
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
struct InstallGuide {
    command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_json: Option<String>,
    steps: Vec<String>,
}

async fn mcp_install_guide(
    pool: &PgPool,
    slug: &str,
    platform: &str,
) -> Result<InstallGuide, (i32, String)> {
    if !matches!(platform, "claude" | "cursor" | "generic") {
        return Err((-32602, format!("invalid platform: {platform}")));
    }

    let tool = mcp_get_tool(pool, slug)
        .await
        .map_err(|m| (-32000, m))?;
    let install = tool
        .install_command
        .clone()
        .unwrap_or_else(|| "No install command available.".into());

    let (command, config_json, steps) = match platform {
        "claude" => (
            format!("claude mcp add onchainai -- {install}"),
            None,
            vec![
                "Open Claude Desktop settings.".into(),
                "Add the MCP server with the command above.".into(),
                "Restart Claude to load the tool.".into(),
            ],
        ),
        "cursor" => (
            install.clone(),
            Some(
                json!({
                    "mcpServers": {
                        "onchainai": { "command": "npx", "args": ["mcp-remote", "onchainai.xyz/mcp"] }
                    }
                })
                .to_string(),
            ),
            vec![
                "Open Cursor MCP settings.".into(),
                "Paste the config JSON.".into(),
                "Reload MCP servers.".into(),
            ],
        ),
        _ => (
            install.clone(),
            None,
            vec!["Run the install command in your terminal.".into()],
        ),
    };

    Ok(InstallGuide {
        command,
        config_json,
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_has_four_tools() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let value = rt.block_on(tools_list()).unwrap();
        let tools = value["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
    }

}