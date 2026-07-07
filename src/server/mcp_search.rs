//! MCP search paging and ranking helpers.

use crate::models::tool::PublicToolSummary;
use crate::models::Tool;
use crate::server::functions::{
    append_tool_filters, intent_to_tool_filters, push_list_query_filter, resolve_list_search_match,
};
use crate::server::queries::{COUNT_APPROVED_TOOLS_SQL, PUBLIC_TOOL_WHERE};
use crate::server::tool_categories::is_public_tool_category;
use crate::server::tool_search::{
    fts_query_bind, resolve_search_intent, ToolSearchMatch, TOOL_SEARCH_VECTOR,
};
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
}

/// Slim search hit for MCP agents — enough to compare candidates before detail.
pub(crate) type McpToolSummary = PublicToolSummary;

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

fn validate_category(category: Option<&str>) -> Result<(), (i32, String)> {
    let Some(category) = category.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    if is_public_tool_category(category) {
        Ok(())
    } else {
        Err((-32602, format!("invalid category: {category}")))
    }
}

fn validate_chain(chain: Option<&str>) -> Result<(), (i32, String)> {
    let Some(chain) = chain.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
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
    Ok(())
}

fn push_fts_rank_order<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    search_text: &'qb str,
    match_mode: ToolSearchMatch,
) {
    let Some(bind_value) = fts_query_bind(match_mode, search_text) else {
        query.push(" ORDER BY metadata_quality DESC, stars DESC, updated_at DESC");
        return;
    };
    query.push(" ORDER BY ts_rank_cd(");
    query.push(TOOL_SEARCH_VECTOR);
    query.push(", ");
    match match_mode {
        ToolSearchMatch::And => {
            query.push("plainto_tsquery('english', ");
            query.push_bind(bind_value);
            query.push(")");
        }
        ToolSearchMatch::Prefix => {
            query.push("to_tsquery('english', ");
            query.push_bind(bind_value);
            query.push(")");
        }
        ToolSearchMatch::Or => {
            query.push("to_tsquery('english', replace(plainto_tsquery('english', ");
            query.push_bind(bind_value);
            query.push(")::text, ' & ', ' | '))");
        }
    }
    query.push(") DESC, metadata_quality DESC, stars DESC, updated_at DESC");
}

fn push_mcp_sort_order<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    sort: McpSearchSort,
    search_text: Option<&'qb str>,
    match_mode: ToolSearchMatch,
) {
    if let Some(text) = search_text.filter(|value| !value.is_empty()) {
        if matches!(sort, McpSearchSort::Relevance) {
            push_fts_rank_order(query, text, match_mode);
            return;
        }
    }
    match sort {
        McpSearchSort::Recent => {
            query.push(" ORDER BY updated_at DESC, metadata_quality DESC, stars DESC");
        }
        _ => {
            query.push(" ORDER BY metadata_quality DESC, stars DESC, updated_at DESC");
        }
    }
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
    let intent = resolve_search_intent(&query, category, chain);
    validate_category(intent.function.as_deref())?;
    validate_chain(intent.chain.as_deref())?;

    let filters = intent_to_tool_filters(&intent);
    let search_text = intent.query.trim();
    let has_fts = !search_text.is_empty();
    let match_mode = if has_fts {
        resolve_list_search_match(pool, search_text, &filters)
            .await
            .map_err(|e| (-32603, format!("db error: {e}")))?
    } else {
        ToolSearchMatch::And
    };

    let limit = limit.clamp(1, 25);
    let fetch_limit = limit + 1;

    let mut count_q = sqlx::QueryBuilder::new(COUNT_APPROVED_TOOLS_SQL);
    if has_fts {
        push_list_query_filter(&mut count_q, Some(search_text), match_mode);
    }
    append_tool_filters(&mut count_q, &filters);

    let mut list_q = sqlx::QueryBuilder::new(
        "SELECT *, \
         ((CASE WHEN repo_url IS NOT NULL THEN 1 ELSE 0 END) \
         + (CASE WHEN stars > 0 THEN 1 ELSE 0 END) \
         + (CASE WHEN coalesce(array_length(chains, 1), 0) > 0 THEN 1 ELSE 0 END) \
         + (CASE WHEN install_command IS NOT NULL THEN 1 ELSE 0 END) \
         + (CASE WHEN last_commit_at IS NOT NULL THEN 1 ELSE 0 END))::int AS metadata_quality \
         FROM tools WHERE ",
    );
    list_q.push(PUBLIC_TOOL_WHERE);
    if has_fts {
        push_list_query_filter(&mut list_q, Some(search_text), match_mode);
    }
    append_tool_filters(&mut list_q, &filters);
    // Axis-token extraction (e.g. "base mcp" -> chain=base, tool_type=mcp) can
    // empty `search_text` entirely. Fall back to ranking (not filtering) by
    // the untouched raw query so a literal name match like "Base MCP" still
    // surfaces first instead of losing all relevance signal.
    let (rank_text, rank_match_mode) = if has_fts {
        (Some(search_text), match_mode)
    } else {
        let raw_query = intent.raw_query.trim();
        if raw_query.is_empty() {
            (None, match_mode)
        } else {
            (Some(raw_query), ToolSearchMatch::And)
        }
    };
    push_mcp_sort_order(&mut list_q, sort, rank_text, rank_match_mode);
    list_q
        .push(" LIMIT ")
        .push_bind(fetch_limit)
        .push(" OFFSET ")
        .push_bind(cursor);

    let (mut tools, total_count) = tokio::try_join!(
        async {
            list_q
                .build_query_as::<Tool>()
                .fetch_all(pool)
                .await
                .map_err(|e| (-32603, format!("db error: {e}")))
        },
        async {
            count_q
                .build_query_as::<(i64,)>()
                .fetch_one(pool)
                .await
                .map_err(|e| (-32603, format!("db error: {e}")))
                .map(|row| row.0)
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
    let has_more = next_cursor.is_some();
    let summaries = tools.into_iter().map(PublicToolSummary::from).collect();

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
        assert!(validate_category(Some("dev-tool")).is_ok());
        assert!(validate_category(Some("bad category")).is_err());
        assert!(validate_chain(Some("Base")).is_ok());
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
                install_risk_level: "low".into(),
                status: "official".into(),
                stars: 100,
                pricing: "free".into(),
                claim_state: "claimed".into(),
                payment_verified: false,
                x402_endpoint_verified: false,
                referral_enabled: false,
                logo_url: None,
                install_command: None,
                safe_copy_command: None,
                official_team: None,
                source: "github".into(),
                license: None,
                x402_price: None,
                logo_monogram: None,
                last_commit_at: None,
                updated_at: chrono::Utc::now(),
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
        assert_eq!(tool["slug"], "uniswap");
        assert_eq!(json["total_count"], 42);
        assert_eq!(json["has_more"], true);
        assert_eq!(json["next_cursor"], "10");
    }
}
