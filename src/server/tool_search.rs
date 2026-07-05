//! Shared full-text search semantics for public tool discovery (web + MCP).

use sqlx::PgPool;

/// Document vector used for public tool text search (name + description only).
pub const TOOL_SEARCH_VECTOR: &str =
    "to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))";

/// AND match — every non-stop-word token must appear (Postgres `plainto_tsquery`).
pub const FTS_AND_MATCH: &str =
    "to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')) @@ plainto_tsquery('english', $1)";

/// OR fallback — any token may match (used only when AND returns zero rows).
pub const FTS_OR_MATCH: &str =
    "to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')) @@ to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | '))";

pub const TS_RANK_AND: &str =
    "ts_rank_cd(to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')), plainto_tsquery('english', $1))";

pub const TS_RANK_OR: &str =
    "ts_rank_cd(to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')), to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | ')))";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSearchMatch {
    And,
    Or,
}

impl ToolSearchMatch {
    pub fn fts_match(self) -> &'static str {
        match self {
            Self::And => FTS_AND_MATCH,
            Self::Or => FTS_OR_MATCH,
        }
    }

    pub fn ts_rank(self) -> &'static str {
        match self {
            Self::And => TS_RANK_AND,
            Self::Or => TS_RANK_OR,
        }
    }
}

/// Whether OR fallback is enabled (default on; set `TOOL_SEARCH_OR_FALLBACK=0` to disable).
pub fn or_fallback_enabled() -> bool {
    !matches!(
        std::env::var("TOOL_SEARCH_OR_FALLBACK").ok().as_deref(),
        Some("0") | Some("false") | Some("FALSE") | Some("off") | Some("OFF")
    )
}

/// OR fallback only when the query has at least two whitespace-separated tokens.
pub fn should_try_or_fallback(query: &str) -> bool {
    or_fallback_enabled() && meaningful_token_count(query) >= 2
}

pub fn meaningful_token_count(query: &str) -> usize {
    query.split_whitespace().filter(|t| !t.is_empty()).count()
}

/// Count approved tools matching the FTS predicate (AND or OR).
pub async fn count_fts_matches(
    pool: &PgPool,
    count_sql: &str,
    query: &str,
    category: Option<&str>,
    chain: Option<&str>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(count_sql)
        .bind(query)
        .bind(category)
        .bind(chain)
        .fetch_one(pool)
        .await
}

/// Resolve AND vs OR: try AND count first; fall back to OR when zero and query is multi-token.
/// Probes the OR count SQL before selecting OR mode so invalid `to_tsquery` inputs stay on AND
/// (empty results) instead of surfacing as 500s.
pub async fn resolve_search_match(
    pool: &PgPool,
    count_sql_and: &str,
    count_sql_or: &str,
    query: &str,
    category: Option<&str>,
    chain: Option<&str>,
) -> Result<ToolSearchMatch, sqlx::Error> {
    let and_count = count_fts_matches(pool, count_sql_and, query, category, chain).await?;
    if and_count == 0 && should_try_or_fallback(query) {
        match count_fts_matches(pool, count_sql_or, query, category, chain).await {
            Ok(_) => Ok(ToolSearchMatch::Or),
            Err(_) => Ok(ToolSearchMatch::And),
        }
    } else {
        Ok(ToolSearchMatch::And)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn or_fallback_requires_multiple_tokens() {
        assert!(!should_try_or_fallback("bridge"));
        assert!(should_try_or_fallback("bridge USDC"));
        assert!(should_try_or_fallback("bridge USDC to Base"));
    }

    #[test]
    fn fts_fragments_reference_search_vector() {
        assert!(FTS_AND_MATCH.contains("plainto_tsquery"));
        assert!(FTS_OR_MATCH.contains("replace"));
    }
}