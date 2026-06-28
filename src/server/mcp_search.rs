//! MCP search paging and ranking helpers.

use crate::models::tool::sanitize_tools_for_public_response;
use crate::models::Tool;
use crate::server::queries::{push_bind_clause, MCP_SEARCH_TOOLS_BASE_SQL};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

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

    fn order_clause(self) -> &'static str {
        match self {
            Self::Relevance => {
                " ORDER BY ts_rank_cd(to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')), plainto_tsquery('english', $1)) DESC, trust_score DESC, stars DESC, updated_at DESC"
            }
            Self::Trust => " ORDER BY trust_score DESC, stars DESC, updated_at DESC",
            Self::Stars => " ORDER BY stars DESC, trust_score DESC, updated_at DESC",
            Self::Recent => " ORDER BY updated_at DESC, stars DESC",
        }
    }
}

#[derive(Serialize)]
pub(crate) struct McpSearchPage {
    tools: Vec<Tool>,
    next_cursor: Option<String>,
    limit: i64,
    sort: &'static str,
}

pub(crate) fn parse_search_limit(value: Option<&Value>) -> i64 {
    value.and_then(|v| v.as_i64()).unwrap_or(10).clamp(1, 25)
}

pub(crate) fn parse_search_cursor(value: Option<&Value>) -> Result<i64, (i32, String)> {
    match value {
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
    let limit = limit.clamp(1, 25);
    let fetch_limit = limit + 1;
    let mut sql = MCP_SEARCH_TOOLS_BASE_SQL.to_string();
    let mut idx = 2;
    if category.is_some() {
        push_bind_clause(&mut sql, "AND function =", idx);
        idx += 1;
    }
    if chain.is_some() {
        push_bind_clause(&mut sql, "AND", idx);
        sql.push_str(" = ANY(chains)");
        idx += 1;
    }
    sql.push_str(sort.order_clause());
    let limit_idx = idx;
    idx += 1;
    let cursor_idx = idx;
    sql.push_str(&format!(" LIMIT ${limit_idx} OFFSET ${cursor_idx}"));

    let mut q = sqlx::query_as::<_, Tool>(&sql).bind(query);
    if let Some(c) = &category {
        q = q.bind(c);
    }
    if let Some(ch) = &chain {
        q = q.bind(ch);
    }
    q = q.bind(fetch_limit).bind(cursor);

    let mut tools = q
        .fetch_all(pool)
        .await
        .map_err(|e| (-32603, format!("db error: {e}")))?;
    let next_cursor = if tools.len() as i64 > limit {
        tools.truncate(limit as usize);
        Some((cursor + limit).to_string())
    } else {
        None
    };
    Ok(McpSearchPage {
        tools: sanitize_tools_for_public_response(tools),
        next_cursor,
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
}
