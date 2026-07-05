//! Shared full-text search semantics for public tool discovery (web + MCP).

use crate::discovery::parse_search_intent;
use sqlx::PgPool;

/// Document vector used for public tool text search (name weighted above description).
pub const TOOL_SEARCH_VECTOR: &str =
    "setweight(to_tsvector('english', coalesce(name, '')), 'A') || setweight(to_tsvector('english', coalesce(description, '')), 'B')";

/// AND match — every non-stop-word token must appear (Postgres `plainto_tsquery`).
pub const FTS_AND_MATCH: &str =
    "setweight(to_tsvector('english', coalesce(name, '')), 'A') || setweight(to_tsvector('english', coalesce(description, '')), 'B') @@ plainto_tsquery('english', $1)";

/// Prefix match — sanitized tokens with `:*` suffix (used when AND returns zero rows).
pub const FTS_PREFIX_MATCH: &str =
    "setweight(to_tsvector('english', coalesce(name, '')), 'A') || setweight(to_tsvector('english', coalesce(description, '')), 'B') @@ to_tsquery('english', $1)";

/// OR fallback — any token may match (used only when AND and Prefix return zero rows).
pub const FTS_OR_MATCH: &str =
    "setweight(to_tsvector('english', coalesce(name, '')), 'A') || setweight(to_tsvector('english', coalesce(description, '')), 'B') @@ to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | '))";

pub const TS_RANK_AND: &str =
    "ts_rank_cd(setweight(to_tsvector('english', coalesce(name, '')), 'A') || setweight(to_tsvector('english', coalesce(description, '')), 'B'), plainto_tsquery('english', $1))";

pub const TS_RANK_PREFIX: &str =
    "ts_rank_cd(setweight(to_tsvector('english', coalesce(name, '')), 'A') || setweight(to_tsvector('english', coalesce(description, '')), 'B'), to_tsquery('english', $1))";

pub const TS_RANK_OR: &str =
    "ts_rank_cd(setweight(to_tsvector('english', coalesce(name, '')), 'A') || setweight(to_tsvector('english', coalesce(description, '')), 'B'), to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | ')))";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedSearchIntent {
    pub query: String,
    pub function: Option<String>,
    pub chain: Option<String>,
    pub tool_type: Option<String>,
    pub install_risk: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSearchMatch {
    And,
    Prefix,
    Or,
}

impl ToolSearchMatch {
    pub fn fts_match(self) -> &'static str {
        match self {
            Self::And => FTS_AND_MATCH,
            Self::Prefix => FTS_PREFIX_MATCH,
            Self::Or => FTS_OR_MATCH,
        }
    }

    pub fn ts_rank(self) -> &'static str {
        match self {
            Self::And => TS_RANK_AND,
            Self::Prefix => TS_RANK_PREFIX,
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

/// Sanitize a single whitespace token for safe inclusion in a prefix `to_tsquery`.
fn sanitize_prefix_token(token: &str) -> Option<String> {
    let cleaned: String = token
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(format!("{cleaned}:*"))
    }
}

/// Build a prefix tsquery string (`token:* & token2:*`) from user input.
pub fn build_prefix_tsquery(query: &str) -> Option<String> {
    let tokens: Vec<String> = query
        .split_whitespace()
        .filter_map(sanitize_prefix_token)
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" & "))
    }
}

/// Value bound as `$1` for the active FTS match mode.
pub fn fts_query_bind(match_mode: ToolSearchMatch, query: &str) -> Option<String> {
    match match_mode {
        ToolSearchMatch::And | ToolSearchMatch::Or => Some(query.to_string()),
        ToolSearchMatch::Prefix => build_prefix_tsquery(query),
    }
}

/// Count approved tools matching the FTS predicate (AND, Prefix, or OR).
pub async fn count_fts_matches(
    pool: &PgPool,
    count_sql: &str,
    match_mode: ToolSearchMatch,
    query: &str,
    category: Option<&str>,
    chain: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let Some(bind_value) = fts_query_bind(match_mode, query) else {
        return Ok(0);
    };
    sqlx::query_scalar::<_, i64>(count_sql)
        .bind(bind_value)
        .bind(category)
        .bind(chain)
        .fetch_one(pool)
        .await
}

/// Parse natural-language query tokens into FTS text plus optional axis filters.
/// Explicit `function` / `chain` parameters take priority over parsed intent values.
pub fn resolve_search_intent(
    query: &str,
    function: Option<String>,
    chain: Option<String>,
) -> ResolvedSearchIntent {
    let intent = parse_search_intent(query);
    ResolvedSearchIntent {
        query: intent.query_terms.trim().to_string(),
        function: function.or(intent.function),
        chain: chain.or(intent.chain),
        tool_type: intent.tool_type,
        install_risk: intent.install_risk,
    }
}

/// Resolve AND vs Prefix vs OR: try AND first; fall back to Prefix when zero; then OR for multi-token.
/// Probes downstream count SQL before selecting a mode so invalid `to_tsquery` inputs stay on AND
/// (empty results) instead of surfacing as 500s.
pub async fn resolve_search_match(
    pool: &PgPool,
    count_sql_and: &str,
    count_sql_prefix: &str,
    count_sql_or: &str,
    query: &str,
    category: Option<&str>,
    chain: Option<&str>,
) -> Result<ToolSearchMatch, sqlx::Error> {
    let and_count = count_fts_matches(
        pool,
        count_sql_and,
        ToolSearchMatch::And,
        query,
        category,
        chain,
    )
    .await?;
    if and_count == 0 {
        if build_prefix_tsquery(query).is_some() {
            match count_fts_matches(
                pool,
                count_sql_prefix,
                ToolSearchMatch::Prefix,
                query,
                category,
                chain,
            )
            .await
            {
                Ok(prefix_count) if prefix_count > 0 => return Ok(ToolSearchMatch::Prefix),
                Ok(_) => {}
                Err(_) => {}
            }
        }
        if should_try_or_fallback(query) {
            match count_fts_matches(
                pool,
                count_sql_or,
                ToolSearchMatch::Or,
                query,
                category,
                chain,
            )
            .await
            {
                Ok(or_count) if or_count > 0 => return Ok(ToolSearchMatch::Or),
                Ok(_) | Err(_) => {}
            }
        }
    }
    Ok(ToolSearchMatch::And)
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
        assert!(FTS_PREFIX_MATCH.contains("to_tsquery"));
        assert!(TOOL_SEARCH_VECTOR.contains("setweight"));
    }

    #[test]
    fn build_prefix_tsquery_sanitizes_and_suffixes_tokens() {
        assert_eq!(build_prefix_tsquery("unisw"), Some("unisw:*".into()));
        assert_eq!(
            build_prefix_tsquery("uni swap"),
            Some("uni:* & swap:*".into())
        );
        assert_eq!(build_prefix_tsquery("unisw!@#"), Some("unisw:*".into()));
        assert_eq!(build_prefix_tsquery("!!!"), None);
        assert_eq!(build_prefix_tsquery("   "), None);
    }

    #[test]
    fn build_prefix_tsquery_strips_non_alnum_from_each_token() {
        assert_eq!(
            build_prefix_tsquery("uni-swap_v2"),
            Some("uniswapv2:*".into())
        );
    }

    #[test]
    fn fts_query_bind_returns_prefix_query_for_prefix_mode() {
        assert_eq!(
            fts_query_bind(ToolSearchMatch::Prefix, "unisw"),
            Some("unisw:*".into())
        );
        assert_eq!(
            fts_query_bind(ToolSearchMatch::And, "uniswap"),
            Some("uniswap".into())
        );
    }

    #[test]
    fn resolve_search_intent_extracts_bridge_and_base() {
        let intent = resolve_search_intent("bridge USDC to Base", None, None);
        assert_eq!(intent.function.as_deref(), Some("bridge"));
        assert_eq!(intent.chain.as_deref(), Some("base"));
        assert!(!intent.query.is_empty());
    }

    #[test]
    fn resolve_search_intent_honors_explicit_params_and_strips_query_tokens() {
        let intent =
            resolve_search_intent("mcp wallet", Some("swap".into()), Some("solana".into()));
        assert_eq!(intent.query, "wallet");
        assert_eq!(intent.function.as_deref(), Some("swap"));
        assert_eq!(intent.chain.as_deref(), Some("solana"));
        assert_eq!(intent.tool_type.as_deref(), Some("mcp"));
    }

    #[test]
    fn resolve_search_intent_empty_fts_when_intent_consumes_all_tokens() {
        let intent = resolve_search_intent("mcp base", None, None);
        assert_eq!(intent.query, "");
        assert!(intent.function.is_none());
        assert_eq!(intent.chain.as_deref(), Some("base"));
    }

    #[test]
    fn resolve_search_intent_maps_type_only_query() {
        let intent = resolve_search_intent("mcp", None, None);
        assert_eq!(intent.query, "");
        assert_eq!(intent.tool_type.as_deref(), Some("mcp"));
    }
}
