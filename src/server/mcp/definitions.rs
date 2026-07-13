//! MCP tools/list schema definitions.

use crate::server::tool_categories::PUBLIC_TOOL_CATEGORY_IDS;
use serde_json::{json, Value};

pub(crate) async fn tools_list(authenticated: bool) -> Result<Value, (i32, String)> {
    Ok(json!({ "tools": tool_definitions(authenticated) }))
}

pub(crate) fn tool_definitions(authenticated: bool) -> Vec<Value> {
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

pub(crate) fn search_tools_definition() -> Value {
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

pub(crate) fn get_tool_detail_definition() -> Value {
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

pub(crate) fn list_categories_definition() -> Value {
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
        "description": "Premium K2 x402 trust data: endpoint liveness, 30-day probe uptime, and last probe time for a listed x402 tool. On public POST /mcp requires ~$0.001 USDC (Base, CDP env gate) per call — HTTP 402 + PAYMENT-REQUIRED. Standard MCP clients (Claude Code, Cursor) cannot complete payment and may show a connection error — use free get_tool_detail for x402 verification flags, or GET /api/v2/premium/check-endpoint-health/{slug} with an x402-capable HTTP client. On POST /mcp/okx this is part of the OKX package rate.",
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
        "description": "Free discovery on public POST /mcp: compare 2–4 approved tools side-by-side on trust, install risk, chains, pricing, and x402 status. Alternative: call get_tool_detail for each slug. On POST /mcp/okx this tool is package-metered.",
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
        "description": "Premium: export a bundle of approved tools as JSON + markdown install kit for agents. Pass slugs or a function category id. Always paid on public POST /mcp — $0.01 USDC on Base (Axis B / site_settings; HTTP 402 + PAYMENT-REQUIRED). Not free discovery. On POST /mcp/okx uses the OKX package rate instead.",
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
        "description": "Premium: returns a single verified live x402 tool for a task. Probes top candidates on-demand for liveness and price honesty, then returns the best one with rejection reasons for the rest. Always paid on public POST /mcp — $0.01 USDC on Base (Axis B; HTTP 402 + PAYMENT-REQUIRED). Use free search_tools first to check if candidates exist. On POST /mcp/okx uses the OKX package rate instead.",
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
        "description": "Premium: decomposes a task intent into subgoals and maps each to OnchainAI catalog tools, surfacing gaps where no tools exist. Returns a subgoal table with covered (candidate slugs) or gap (manual research needed) status. Always paid on public POST /mcp — $0.01 USDC on Base (Axis B; HTTP 402 + PAYMENT-REQUIRED). Use free search_tools for simple lookups. On POST /mcp/okx uses the OKX package rate instead.",
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

pub(crate) fn get_install_guide_definition() -> Value {
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
