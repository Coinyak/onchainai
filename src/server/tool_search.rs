//! Shared full-text search semantics for public tool discovery (web + MCP).

use crate::discovery::parse_search_intent;
use sqlx::PgPool;

/// Expands to the FTS document vector SQL expression (for use inside `concat!`).
macro_rules! tool_search_vector {
    () => {
        "(setweight(to_tsvector('english', coalesce(name, '')), 'A') \
 || setweight(to_tsvector('english', coalesce(description, '')), 'B') \
 || setweight(to_tsvector('english', coalesce(slug, '')), 'A') \
 || setweight(to_tsvector('english', coalesce(repo_url, '')), 'C'))"
    };
}

/// Document vector used for public tool text search (name/slug weighted above description/repo).
/// All vector arms use the `english` config so document lexemes match `plainto_tsquery('english', ...)`.
/// GIN index `idx_tools_search` must mirror this expression — see `migrations/037_tools_search_vector_index.sql`.
/// Parentheses are required before `@@` in match predicates (`@@` binds tighter than `||`).
pub const TOOL_SEARCH_VECTOR: &str = tool_search_vector!();

/// AND match — every non-stop-word token must appear (Postgres `plainto_tsquery`).
pub const FTS_AND_MATCH: &str =
    concat!(tool_search_vector!(), " @@ plainto_tsquery('english', $1)");

/// Prefix match — sanitized tokens with `:*` suffix (used when AND returns zero rows).
pub const FTS_PREFIX_MATCH: &str = concat!(tool_search_vector!(), " @@ to_tsquery('english', $1)");

/// OR fallback — any token may match (used only when AND and Prefix return zero rows).
pub const FTS_OR_MATCH: &str = concat!(
    tool_search_vector!(),
    " @@ to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | '))"
);

pub const TS_RANK_AND: &str = concat!(
    "ts_rank_cd(",
    tool_search_vector!(),
    ", plainto_tsquery('english', $1))"
);

pub const TS_RANK_PREFIX: &str = concat!(
    "ts_rank_cd(",
    tool_search_vector!(),
    ", to_tsquery('english', $1))"
);

pub const TS_RANK_OR: &str = concat!(
    "ts_rank_cd(",
    tool_search_vector!(),
    ", to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | ')))"
);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedSearchIntent {
    pub query: String,
    pub function: Option<String>,
    pub chain: Option<String>,
    pub tool_type: Option<String>,
    pub install_risk: Option<String>,
    /// Untouched input text, kept even when `query` is emptied by axis-token
    /// extraction (e.g. "base mcp" -> chain=base, tool_type=mcp, query="").
    /// Used to rank literal name matches (like a tool named "Base MCP")
    /// instead of losing all relevance signal to a stars/date fallback.
    pub raw_query: String,
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
        raw_query: query.trim().to_string(),
    }
}

/// Select AND vs Prefix vs OR from precomputed counts (`None` = not tried or query error).
pub fn select_search_match(
    query: &str,
    and_count: i64,
    prefix_count: Option<i64>,
    or_count: Option<i64>,
) -> ToolSearchMatch {
    if and_count > 0 {
        return ToolSearchMatch::And;
    }
    if prefix_count.is_some_and(|count| count > 0) {
        return ToolSearchMatch::Prefix;
    }
    if should_try_or_fallback(query) && or_count.is_some_and(|count| count > 0) {
        return ToolSearchMatch::Or;
    }
    ToolSearchMatch::And
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
    let prefix_count = if and_count == 0 && build_prefix_tsquery(query).is_some() {
        count_fts_matches(
            pool,
            count_sql_prefix,
            ToolSearchMatch::Prefix,
            query,
            category,
            chain,
        )
        .await
        .ok()
    } else {
        None
    };
    let or_count = if and_count == 0 && should_try_or_fallback(query) {
        count_fts_matches(
            pool,
            count_sql_or,
            ToolSearchMatch::Or,
            query,
            category,
            chain,
        )
        .await
        .ok()
    } else {
        None
    };
    Ok(select_search_match(
        query,
        and_count,
        prefix_count,
        or_count,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_search_match_prefers_and_when_count_positive() {
        assert_eq!(
            select_search_match("uniswap", 3, Some(0), Some(0)),
            ToolSearchMatch::And
        );
    }

    #[test]
    fn select_search_match_falls_back_to_prefix_then_or() {
        assert_eq!(
            select_search_match("unisw", 0, Some(2), None),
            ToolSearchMatch::Prefix
        );
        assert_eq!(
            select_search_match("bridge USDC", 0, Some(0), Some(1)),
            ToolSearchMatch::Or
        );
        assert_eq!(
            select_search_match("bridge", 0, Some(0), Some(1)),
            ToolSearchMatch::And
        );
    }

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
        assert!(TOOL_SEARCH_VECTOR.starts_with('('));
        assert!(FTS_AND_MATCH.starts_with('('));
        assert!(FTS_AND_MATCH.contains("slug"));
        assert!(FTS_AND_MATCH.contains("repo_url"));
        assert!(
            !FTS_AND_MATCH.contains("'simple'"),
            "vector arms must use english to match tsquery config"
        );
        assert_eq!(
            FTS_AND_MATCH.matches("'english'").count(),
            5,
            "name, description, slug, repo_url vectors plus plainto_tsquery"
        );
        assert!(
            FTS_AND_MATCH.find("@@").unwrap() > FTS_AND_MATCH.find("||").unwrap(),
            "@@ must apply to the full vector, not only the description arm"
        );
    }

    #[test]
    fn fts_vector_includes_slug_and_repo_fields() {
        assert!(TOOL_SEARCH_VECTOR.contains("slug"));
        assert!(TOOL_SEARCH_VECTOR.contains("repo_url"));
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
    fn resolve_search_intent_single_token_mcp_stays_fts() {
        let intent = resolve_search_intent("mcp", None, None);
        assert_eq!(intent.query, "mcp");
        assert_eq!(intent.tool_type, None);
    }

    #[test]
    fn resolve_search_intent_single_token_x402_stays_fts() {
        let intent = resolve_search_intent("x402", None, None);
        assert_eq!(intent.query, "x402");
        assert_eq!(intent.tool_type, None);
    }
}
