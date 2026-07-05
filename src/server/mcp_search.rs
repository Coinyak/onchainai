//! MCP search paging and ranking helpers.

use crate::models::tool::sanitize_tools_for_public_response;
use crate::models::Tool;
use crate::server::queries::{
    MCP_SEARCH_TOOLS_COUNT_SQL, MCP_SEARCH_TOOLS_RECENT_SQL, MCP_SEARCH_TOOLS_RELEVANCE_SQL,
    MCP_SEARCH_TOOLS_STARS_SQL, MCP_SEARCH_TOOLS_TRUST_SQL,
};
use crate::server::tool_categories::is_public_tool_category;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

const MAX_SEARCH_QUERY_LEN: usize = 200;
const MAX_FILTER_LEN: usize = 64;
const MAX_CURSOR_OFFSET: i64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum McpSearchSort {
    Relevance,
    Trust,
    Stars,
    Recent,
}

impl McpSearchSort {
    pub(crate) fn parse(value: &str) -> Result<Self, (i32, String)> {
        match value {
            "relevance" => Ok(Self::Relevance),
            "trust" => Ok(Self::Trust),
            "stars" => Ok(Self::Stars),
            "recent" => Ok(Self::Recent),
            other => Err((
                -32602,
                format!("invalid sort: {other}; expected relevance, trust, stars, or recent"),
            )),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Trust => "trust",
            Self::Stars => "stars",
            Self::Recent => "recent",
        }
    }

    fn query_sql(self) -> &'static str {
        match self {
            Self::Relevance => MCP_SEARCH_TOOLS_RELEVANCE_SQL,
            Self::Trust => MCP_SEARCH_TOOLS_TRUST_SQL,
            Self::Stars => MCP_SEARCH_TOOLS_STARS_SQL,
            Self::Recent => MCP_SEARCH_TOOLS_RECENT_SQL,
        }
    }
}

/// Slim search hit for MCP agents — enough to compare candidates before detail.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct McpToolSummary {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: String,
    pub chains: Vec<String>,
    pub trust_score: i32,
    pub install_risk_level: String,
    pub status: String,
    pub stars: i32,
    pub pricing: String,
    pub claim_state: String,
    pub payment_verified: bool,
    pub x402_endpoint_verified: bool,
    pub referral_enabled: bool,
}

impl From<Tool> for McpToolSummary {
    fn from(tool: Tool) -> Self {
        Self {
            slug: tool.slug,
            name: tool.name,
            description: tool.description,
            tool_type: tool.tool_type,
            function: tool.function,
            chains: tool.chains,
            trust_score: tool.trust_score,
            install_risk_level: tool.install_risk_level,
            status: tool.status,
            stars: tool.stars,
            pricing: tool.pricing,
            claim_state: tool.claim_state,
            payment_verified: tool.payment_verified,
            x402_endpoint_verified: tool.x402_endpoint_verified,
            referral_enabled: tool.referral_enabled,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct McpSearchPage {
    tools: Vec<McpToolSummary>,
    next_cursor: Option<String>,
    has_more: bool,
    total_count: i64,
    limit: i64,
    sort: &'static str,
}

pub(crate) fn parse_search_limit(value: Option<&Value>) -> i64 {
    value.and_then(|v| v.as_i64()).unwrap_or(10).clamp(1, 25)
}

pub(crate) fn parse_search_cursor(value: Option<&Value>) -> Result<i64, (i32, String)> {
    let cursor = match value {
        None | Some(Value::Null) => Ok(0),
        Some(Value::Number(n)) => n
            .as_i64()
            .filter(|offset| *offset >= 0)
            .ok_or_else(|| (-32602, "cursor must be a non-negative offset".to_string())),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(0),
        Some(Value::String(s)) => s
            .parse::<i64>()
            .ok()
            .filter(|offset| *offset >= 0)
            .ok_or_else(|| (-32602, "cursor must be a non-negative offset".to_string())),
        Some(_) => Err((-32602, "cursor must be a string offset".to_string())),
    }?;
    if cursor > MAX_CURSOR_OFFSET {
        return Err((-32602, format!("cursor must be <= {MAX_CURSOR_OFFSET}")));
    }
    Ok(cursor)
}

fn validate_query(query: &str) -> Result<String, (i32, String)> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err((-32602, "query must not be empty".to_string()));
    }
    if trimmed.chars().count() > MAX_SEARCH_QUERY_LEN {
        return Err((
            -32602,
            format!("query must be <= {MAX_SEARCH_QUERY_LEN} characters"),
        ));
    }
    Ok(trimmed.to_string())
}

fn validate_category(category: Option<&str>) -> Result<Option<String>, (i32, String)> {
    let Some(category) = category.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if is_public_tool_category(category) {
        Ok(Some(category.to_string()))
    } else {
        Err((-32602, format!("invalid category: {category}")))
    }
}

fn validate_chain(chain: Option<&str>) -> Result<Option<String>, (i32, String)> {
    let Some(chain) = chain.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if chain.len() > MAX_FILTER_LEN {
        return Err((
            -32602,
            format!("chain must be <= {MAX_FILTER_LEN} characters"),
        ));
    }
    if !chain
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err((-32602, "chain contains unsupported characters".to_string()));
    }
    Ok(Some(chain.to_ascii_lowercase()))
}

pub(crate) async fn mcp_search_tools(
    pool: &PgPool,
    query: &str,
    category: Option<String>,
    chain: Option<String>,
    sort: McpSearchSort,
    limit: i64,
    cursor: i64,
) -> Result<McpSearchPage, (i32, String)> {
    let query = validate_query(query)?;
    let category = validate_category(category.as_deref())?;
    let chain = validate_chain(chain.as_deref())?;
    let limit = limit.clamp(1, 25);
    let fetch_limit = limit + 1;

    let (mut tools, total_count) = tokio::try_join!(
        async {
            sqlx::query_as::<_, Tool>(sort.query_sql())
                .bind(&query)
                .bind(category.as_deref())
                .bind(chain.as_deref())
                .bind(fetch_limit)
                .bind(cursor)
                .fetch_all(pool)
                .await
                .map_err(|e| (-32603, format!("db error: {e}")))
        },
        async {
            sqlx::query_scalar::<_, i64>(MCP_SEARCH_TOOLS_COUNT_SQL)
                .bind(&query)
                .bind(category.as_deref())
                .bind(chain.as_deref())
                .fetch_one(pool)
                .await
                .map_err(|e| (-32603, format!("db error: {e}")))
        }
    )?;

    let fetched_more = tools.len() as i64 > limit;
    if fetched_more {
        tools.truncate(limit as usize);
    }
    let next_cursor = if fetched_more {
        cursor
            .checked_add(limit)
            .filter(|next| *next <= MAX_CURSOR_OFFSET)
            .map(|next| next.to_string())
    } else {
        None
    };
    // Align with next_cursor: if offset cap blocks another page, do not claim has_more.
    let has_more = next_cursor.is_some();
    let summaries = sanitize_tools_for_public_response(tools)
        .into_iter()
        .map(McpToolSummary::from)
        .collect();

    Ok(McpSearchPage {
        tools: summaries,
        next_cursor,
        has_more,
        total_count,
        limit,
        sort: sort.as_str(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_search_limit_clamps_to_public_bounds() {
        assert_eq!(parse_search_limit(Some(&json!(0))), 1);
        assert_eq!(parse_search_limit(Some(&json!(12))), 12);
        assert_eq!(parse_search_limit(Some(&json!(99))), 25);
        assert_eq!(parse_search_limit(None), 10);
    }

    #[test]
    fn parse_search_cursor_accepts_string_or_number_offsets() {
        assert_eq!(parse_search_cursor(Some(&json!("20"))), Ok(20));
        assert_eq!(parse_search_cursor(Some(&json!(30))), Ok(30));
        assert!(parse_search_cursor(Some(&json!("-1"))).is_err());
        assert!(parse_search_cursor(Some(&json!(5001))).is_err());
        assert!(parse_search_cursor(Some(&json!({ "bad": true }))).is_err());
    }

    #[test]
    fn search_sort_parses_known_values() {
        assert_eq!(
            McpSearchSort::parse("relevance"),
            Ok(McpSearchSort::Relevance)
        );
        assert_eq!(McpSearchSort::parse("trust"), Ok(McpSearchSort::Trust));
        assert!(McpSearchSort::parse("unknown").is_err());
    }

    #[test]
    fn validate_search_inputs_bounds_external_values() {
        assert_eq!(validate_query(" bridge ").unwrap(), "bridge");
        assert!(validate_query("").is_err());
        assert!(validate_query(&"x".repeat(MAX_SEARCH_QUERY_LEN + 1)).is_err());
        assert_eq!(
            validate_category(Some("dev-tool")).unwrap(),
            Some("dev-tool".to_string())
        );
        assert!(validate_category(Some("bad category")).is_err());
        assert_eq!(
            validate_chain(Some("Base")).unwrap(),
            Some("base".to_string())
        );
        assert!(validate_chain(Some("base/mainnet")).is_err());
    }

    #[test]
    fn mcp_tool_summary_uses_shared_category_whitelist() {
        use crate::server::tool_categories::PUBLIC_TOOL_CATEGORY_IDS;
        assert_eq!(PUBLIC_TOOL_CATEGORY_IDS.len(), 14);
        for category in PUBLIC_TOOL_CATEGORY_IDS {
            assert!(validate_category(Some(category)).is_ok());
        }
    }

    #[test]
    fn mcp_search_page_serializes_slim_fields() {
        let page = McpSearchPage {
            tools: vec![McpToolSummary {
                slug: "uniswap".into(),
                name: "Uniswap".into(),
                description: Some("DEX".into()),
                tool_type: "mcp".into(),
                function: "swap".into(),
                chains: vec!["ethereum".into()],
                trust_score: 80,
                install_risk_level: "low".into(),
                status: "official".into(),
                stars: 100,
                pricing: "free".into(),
                claim_state: "claimed".into(),
                payment_verified: false,
                x402_endpoint_verified: false,
                referral_enabled: false,
            }],
            next_cursor: Some("10".into()),
            has_more: true,
            total_count: 42,
            limit: 10,
            sort: "relevance",
        };
        let json = serde_json::to_value(&page).unwrap();
        let tool = &json["tools"][0];
        assert!(tool.get("id").is_none());
        assert!(tool.get("mcp_endpoint").is_none());
        assert!(tool.get("install_command").is_none());
        assert_eq!(tool["slug"], "uniswap");
        assert_eq!(json["total_count"], 42);
        assert_eq!(json["has_more"], true);
        assert_eq!(json["next_cursor"], "10");
    }
}
