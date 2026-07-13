//! Public tool listing, dashboard, and toolkit helpers.
use super::super::*;
/// Returns all function categories with live **approved** tool counts.
#[cfg(feature = "ssr")]
pub async fn fetch_categories(pool: &sqlx::PgPool) -> Result<Vec<(Category, i64)>, FnError> {
    fetch_filtered_category_counts(pool, &ToolFilters::default()).await
}

/// Per-function counts for the sidebar, respecting active filters (function axis excluded).
#[cfg(feature = "ssr")]
pub async fn fetch_filtered_category_counts(
    pool: &sqlx::PgPool,
    filters: &ToolFilters,
) -> Result<Vec<(Category, i64)>, FnError> {
    let mut facet_filters = filters.clone();
    facet_filters.function.clear();

    let mut q = sqlx::QueryBuilder::new(
        "SELECT c.id, c.label, c.icon, c.description, c.sort_order, COUNT(t.id)::bigint AS count \
         FROM categories c \
         LEFT JOIN tools t ON t.function = c.id AND ",
    );
    q.push(PUBLIC_TOOL_WHERE);
    append_tool_filters(&mut q, &facet_filters);
    q.push(
        " GROUP BY c.id, c.label, c.icon, c.description, c.sort_order \
          ORDER BY c.sort_order ASC",
    );

    let rows = q
        .build_query_as::<CategoryWithCount>()
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("failed to load filtered category counts: {e}")))?;

    Ok(rows.into_iter().map(CategoryWithCount::into_pair).collect())
}

/// Fetch a single **approved** tool by slug, if present.
#[cfg(feature = "ssr")]
pub async fn fetch_tool_by_slug(
    pool: &sqlx::PgPool,
    slug: &str,
) -> Result<Option<Tool>, FnError> {
    let tool = sqlx::query_as::<_, Tool>(APPROVED_TOOL_BY_SLUG_SQL)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(|e| FnError::new(format!("failed to load tool: {e}")))?;

    Ok(tool.map(sanitize_tool_for_public_response))
}

/// Maximum length for a tool slug accepted at server boundaries.
const MAX_SLUG_LEN: usize = 128;

/// True when `s` is a single token that looks like a tool slug (`^[a-z0-9][a-z0-9-]*$`).
pub fn looks_like_tool_slug(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().expect("non-empty");
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Validate an externally-supplied slug at the server boundary: non-empty,
/// bounded length, allowed charset (URL-safe slug characters only).
fn validate_slug(slug: &str) -> Result<(), FnError> {
    let slug = slug.trim();
    if slug.is_empty() {
        return Err(FnError::new("slug must not be empty"));
    }
    if slug.len() > MAX_SLUG_LEN {
        return Err(FnError::new(format!(
            "slug must be at most {MAX_SLUG_LEN} characters"
        )));
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(FnError::new(
            "slug contains invalid characters; only a-z, 0-9, '-', '_', '.' allowed",
        ));
    }
    Ok(())
}

/// Fetch an approved tool and build its public install guide (shared server/MCP path).
#[cfg(feature = "ssr")]
pub async fn fetch_public_install_guide(
    pool: &sqlx::PgPool,
    slug: &str,
    platform: &str,
) -> Result<crate::public_install_guide::PublicInstallGuide, FnError> {
    validate_slug(slug)?;
    let tool = fetch_tool_by_slug(pool, slug)
        .await?
        .ok_or_else(|| FnError::new(format!("tool not found: {slug}")))?;
    crate::public_install_guide::build_install_guide_for_platform(&tool, slug, platform)
        .map_err(FnError::new)
}

/// Maximum tools returned by `list_tools` / browser "load more" (matches UI cap).
pub const MAX_LIST_TOOLS_LIMIT: i64 = 500;

/// Clamp list-tools `limit` to the browser cap (500), never the legacy 100 ceiling.
pub fn clamp_list_tools_limit(limit: i64) -> i64 {
    limit.clamp(1, MAX_LIST_TOOLS_LIMIT)
}

const MAX_TOOL_FILTER_VALUES: usize = 20;
const MAX_TOOL_FILTER_VALUE_LEN: usize = 64;
const MAX_TOOL_LIST_QUERY_LEN: usize = 200;

pub fn validate_search_tools_input(
    query: &str,
    function: &Option<String>,
    chain: &Option<String>,
) -> Result<(), FnError> {
    if query.trim().is_empty() {
        return Err(FnError::new("query must not be empty"));
    }
    if query.len() > MAX_TOOL_LIST_QUERY_LEN {
        return Err(FnError::new(format!(
            "query must be at most {MAX_TOOL_LIST_QUERY_LEN} characters"
        )));
    }
    if let Some(filter) = function {
        validate_tool_filter_values("function", std::slice::from_ref(filter))?;
    }
    if let Some(filter) = chain {
        validate_tool_filter_values("chain", std::slice::from_ref(filter))?;
    }
    Ok(())
}

fn validate_tool_filter_values(axis: &str, values: &[String]) -> Result<(), FnError> {
    if values.len() > MAX_TOOL_FILTER_VALUES {
        return Err(FnError::new(format!(
            "filter `{axis}` accepts at most {MAX_TOOL_FILTER_VALUES} values"
        )));
    }
    for value in values {
        if value.len() > MAX_TOOL_FILTER_VALUE_LEN {
            return Err(FnError::new(format!(
                "filter `{axis}` values must be at most {MAX_TOOL_FILTER_VALUE_LEN} characters"
            )));
        }
    }
    Ok(())
}

/// Validates multi-axis tool filters for list/count queries.
pub fn validate_tool_filters(filters: &ToolFilters) -> Result<(), FnError> {
    validate_tool_filter_values("function", &filters.function)?;
    validate_tool_filter_values("asset_class", &filters.asset_class)?;
    validate_tool_filter_values("actor", &filters.actor)?;
    validate_tool_filter_values("tool_type", &filters.tool_type)?;
    validate_tool_filter_values("status", &filters.status)?;
    validate_tool_filter_values("pricing", &filters.pricing)?;
    validate_tool_filter_values("install_risk", &filters.install_risk)?;
    validate_tool_filter_values("chain", &filters.chain)?;
    Ok(())
}

fn validate_tool_sort(sort: &str) -> Result<(), FnError> {
    matches!(sort, "hot" | "new" | "comments")
        .then_some(())
        .ok_or_else(|| FnError::new("sort must be one of: hot, new, comments"))
}

/// Validates browser tool-list request bounds (rejects out-of-range instead of clamping).
pub fn validate_tool_list_request(req: &ToolListRequest) -> Result<(), FnError> {
    validate_tool_sort(&req.sort)?;
    validate_tool_filters(&req.filters)?;
    if req.offset < 0 {
        return Err(FnError::new("offset must be >= 0"));
    }
    if !(1..=MAX_LIST_TOOLS_LIMIT).contains(&req.limit) {
        return Err(FnError::new(format!(
            "limit must be between 1 and {MAX_LIST_TOOLS_LIMIT}"
        )));
    }
    if let Some(query) = req.query.as_ref() {
        if query.len() > MAX_TOOL_LIST_QUERY_LEN {
            return Err(FnError::new(format!(
                "query must be at most {MAX_TOOL_LIST_QUERY_LEN} characters"
            )));
        }
    }
    Ok(())
}

/// Stable request payload for browser tool-list queries (avoids positional arg drift).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolListRequest {
    pub sort: String,
    pub offset: i64,
    pub limit: i64,
    pub filters: ToolFilters,
    pub query: Option<String>,
}

/// Optional axis filters for tool list/count queries (AND across axes and within each axis).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolFilters {
    #[serde(default)]
    pub function: Vec<String>,
    #[serde(default)]
    pub asset_class: Vec<String>,
    #[serde(default)]
    pub actor: Vec<String>,
    #[serde(default)]
    pub tool_type: Vec<String>,
    #[serde(default)]
    pub status: Vec<String>,
    #[serde(default)]
    pub pricing: Vec<String>,
    #[serde(default)]
    pub install_risk: Vec<String>,
    #[serde(default)]
    pub chain: Vec<String>,
    /// Soft chain filters from parsed query text (not explicit API params).
    /// Matches tools where `chains` contains the token OR `chains` is empty,
    /// since many crawled tools have `chains: []` but still support the chain.
    #[serde(default, skip)]
    pub chain_soft: Vec<String>,
}

fn merge_optional_filter(values: &mut Vec<String>, value: Option<&str>) {
    if let Some(value) = value.filter(|part| !part.trim().is_empty()) {
        if !values.iter().any(|existing| existing == value) {
            values.push(value.to_string());
        }
    }
}

/// Build axis filters from parsed search intent (single value per axis).
pub fn intent_to_tool_filters(
    intent: &crate::server::tool_search::ResolvedSearchIntent,
) -> ToolFilters {
    let mut filters = ToolFilters::default();
    merge_optional_filter(&mut filters.function, intent.function.as_deref());
    merge_optional_filter(&mut filters.chain, intent.chain.as_deref());
    merge_optional_filter(&mut filters.tool_type, intent.tool_type.as_deref());
    merge_optional_filter(&mut filters.install_risk, intent.install_risk.as_deref());
    filters.chain_soft = intent.chain_soft.clone();
    filters
}

/// Merge parsed intent into existing browser/API filters without dropping URL params.
pub fn merge_search_intent_into_filters(
    base: &ToolFilters,
    intent: &crate::server::tool_search::ResolvedSearchIntent,
) -> ToolFilters {
    let mut merged = base.clone();
    merge_optional_filter(&mut merged.function, intent.function.as_deref());
    merge_optional_filter(&mut merged.chain, intent.chain.as_deref());
    merge_optional_filter(&mut merged.tool_type, intent.tool_type.as_deref());
    merge_optional_filter(&mut merged.install_risk, intent.install_risk.as_deref());
    // Only populate chain_soft from intent when no explicit chain filter was
    // set via URL params (explicit chain wins as hard filter).
    if merged.chain.is_empty() {
        merged.chain_soft = intent.chain_soft.clone();
    } else {
        merged.chain_soft.clear();
    }
    merged
}

#[cfg(feature = "ssr")]
fn append_scalar_union<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    column: &str,
    values: &'qb [String],
) {
    if values.is_empty() {
        return;
    }
    query.push(" AND ");
    if values.len() == 1 {
        query.push(column).push(" = ").push_bind(&values[0]);
        return;
    }
    query.push("(");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            query.push(" OR ");
        }
        query.push(column).push(" = ").push_bind(value);
    }
    query.push(")");
}

/// x402 catalog slice: matches dashboard snapshot semantics (not only `type`/`pricing` = x402).
#[cfg(feature = "ssr")]
fn append_x402_catalog_predicate(query: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>) {
    query.push(
        " AND (type = 'x402' OR pricing = 'x402' OR x402_price IS NOT NULL OR referral_enabled = true)",
    );
}

#[cfg(feature = "ssr")]
fn append_scalar_union_except<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    column: &str,
    values: &'qb [String],
    skip: &str,
) {
    let kept: Vec<&'qb String> = values
        .iter()
        .filter(|value| value.as_str() != skip)
        .collect();
    if kept.is_empty() {
        return;
    }
    query.push(" AND ");
    if kept.len() == 1 {
        query.push(column).push(" = ").push_bind(kept[0]);
        return;
    }
    query.push("(");
    for (index, value) in kept.iter().enumerate() {
        if index > 0 {
            query.push(" OR ");
        }
        query.push(column).push(" = ").push_bind(*value);
    }
    query.push(")");
}

#[cfg(feature = "ssr")]
fn filters_include_x402(filters: &ToolFilters) -> bool {
    filters.tool_type.iter().any(|value| value == "x402")
        || filters.pricing.iter().any(|value| value == "x402")
}

/// Canonicalize chain filter values so URL params using aliases (e.g.
/// `?chain=bnb`, `?chain=fantom`) match DB rows stored with canonical ids
/// (`bsc`, `sonic`). Deduplicates after normalization.
#[cfg(feature = "ssr")]
fn canonicalize_chain_filters(values: &[String]) -> Vec<String> {
    crate::chains::canonicalize_chain_values(values)
}

#[cfg(feature = "ssr")]
pub fn append_tool_filters<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    filters: &'qb ToolFilters,
) {
    append_scalar_union(query, "function", &filters.function);
    append_scalar_union(query, "asset_class", &filters.asset_class);
    append_scalar_union(query, "actor", &filters.actor);
    append_scalar_union_except(query, "type", &filters.tool_type, "x402");
    append_scalar_union(query, "status", &filters.status);
    append_scalar_union_except(query, "pricing", &filters.pricing, "x402");
    append_scalar_union(query, "install_risk_level", &filters.install_risk);
    if filters_include_x402(filters) {
        append_x402_catalog_predicate(query);
    }
    let canonical_chain = canonicalize_chain_filters(&filters.chain);
    if !canonical_chain.is_empty() {
        // Hard intersection: tool must support every selected chain.
        // Bind canonical ids as a single text[] array.
        query.push(" AND chains @> ").push_bind(canonical_chain);
    }
    let canonical_chain_soft = canonicalize_chain_filters(&filters.chain_soft);
    if !canonical_chain_soft.is_empty() {
        // Soft chain from NL query: single token may match empty `chains` too;
        // multi-token queries require an explicit chain hit (no empty escape).
        let allow_empty_chains = canonical_chain_soft.len() == 1;
        query.push(" AND (");
        for (index, chain) in canonical_chain_soft.iter().enumerate() {
            if index > 0 {
                query.push(" OR ");
            }
            // Bind each chain as a 1-element text[] so push_bind owns the data.
            query.push("chains @> ARRAY[").push_bind(chain.clone());
            query.push("]::text[]");
            if allow_empty_chains {
                query.push(" OR coalesce(array_length(chains, 1), 0) = 0");
            }
        }
        query.push(")");
    }
}

#[cfg(feature = "ssr")]
pub async fn count_list_fts_matches(
    pool: &sqlx::PgPool,
    search: &str,
    filters: &ToolFilters,
    match_mode: crate::server::tool_search::ToolSearchMatch,
) -> Result<i64, FnError> {
    let mut q = sqlx::QueryBuilder::new(COUNT_APPROVED_TOOLS_SQL);
    push_list_query_filter(&mut q, Some(search), match_mode);
    append_tool_filters(&mut q, filters);
    let count = q
        .build_query_as::<(i64,)>()
        .fetch_one(pool)
        .await
        .map_err(|e| FnError::new(format!("search count failed: {e}")))?;
    Ok(count.0)
}

#[cfg(feature = "ssr")]
pub async fn resolve_list_search_match(
    pool: &sqlx::PgPool,
    search: &str,
    filters: &ToolFilters,
) -> Result<crate::server::tool_search::ToolSearchMatch, FnError> {
    use crate::server::tool_search::{
        build_prefix_tsquery, select_search_match, should_try_or_fallback, ToolSearchMatch,
    };

    let and_count = count_list_fts_matches(pool, search, filters, ToolSearchMatch::And).await?;
    let prefix_count = if and_count == 0 && build_prefix_tsquery(search).is_some() {
        count_list_fts_matches(pool, search, filters, ToolSearchMatch::Prefix)
            .await
            .ok()
    } else {
        None
    };
    let or_count = if and_count == 0 && should_try_or_fallback(search) {
        count_list_fts_matches(pool, search, filters, ToolSearchMatch::Or)
            .await
            .ok()
    } else {
        None
    };
    Ok(select_search_match(
        search,
        and_count,
        prefix_count,
        or_count,
    ))
}

#[cfg(feature = "ssr")]
pub fn push_list_query_filter<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    search: Option<&'qb str>,
    match_mode: crate::server::tool_search::ToolSearchMatch,
) {
    let Some(text) = search.filter(|q| !q.trim().is_empty()) else {
        return;
    };
    use crate::server::tool_search::{fts_query_bind, ToolSearchMatch, TOOL_SEARCH_VECTOR};
    let Some(bind_value) = fts_query_bind(match_mode, text) else {
        return;
    };
    query.push(" AND ").push(TOOL_SEARCH_VECTOR).push(" @@ ");
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
}

#[cfg(feature = "ssr")]
fn push_list_order_offset_limit<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    sort: &str,
    offset: i64,
    limit: i64,
    search: Option<(&'qb str, crate::server::tool_search::ToolSearchMatch)>,
) {
    match sort {
        "new" => {
            query.push(" ORDER BY created_at DESC, metadata_quality DESC, stars DESC");
        }
        "comments" => {
            query.push(
                " ORDER BY \
                 (SELECT COUNT(*)::bigint FROM comments cm WHERE cm.tool_id = tools.id) DESC, \
                 metadata_quality DESC, \
                 created_at DESC",
            );
        }
        _ => {
            // Default ("hot") sort: when a search query is active, relevance
            // (ts_rank_cd) must outrank stars, or an exact name match like
            // "Base MCP" can lose to unrelated tools with more stars.
            if let Some((text, match_mode)) = search {
                push_fts_rank_order_clause(query, text, match_mode);
            } else {
                query.push(" ORDER BY metadata_quality DESC, stars DESC, created_at DESC");
            }
        }
    }
    query.push(" OFFSET ").push_bind(offset);
    query.push(" LIMIT ").push_bind(limit);
}

/// Shared `ORDER BY ts_rank_cd(...) DESC, metadata_quality DESC, stars DESC, created_at DESC` clause,
/// falling back to `metadata_quality DESC, stars DESC, created_at DESC` if the query has no bindable terms.
#[cfg(feature = "ssr")]
fn push_fts_rank_order_clause<'qb>(
    query: &mut sqlx::QueryBuilder<'qb, sqlx::Postgres>,
    search_text: &'qb str,
    match_mode: crate::server::tool_search::ToolSearchMatch,
) {
    use crate::server::tool_search::{fts_query_bind, ToolSearchMatch, TOOL_SEARCH_VECTOR};

    let Some(bind_value) = fts_query_bind(match_mode, search_text) else {
        query.push(" ORDER BY metadata_quality DESC, stars DESC, created_at DESC");
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
    query.push(") DESC, metadata_quality DESC, stars DESC, created_at DESC");
}

/// Count approved tools with optional multi-axis filters and optional FTS `search_q`.
#[cfg(feature = "ssr")]
pub async fn fetch_count_tools(
    pool: &sqlx::PgPool,
    filters: &ToolFilters,
    search: Option<&str>,
) -> Result<i64, FnError> {
    use crate::server::tool_search::ToolSearchMatch;

    let search_text = search.filter(|q| !q.trim().is_empty());
    let match_mode = if let Some(text) = search_text {
        resolve_list_search_match(pool, text, filters).await?
    } else {
        ToolSearchMatch::And
    };

    let mut q = sqlx::QueryBuilder::new(COUNT_APPROVED_TOOLS_SQL);
    push_list_query_filter(&mut q, search, match_mode);
    append_tool_filters(&mut q, filters);

    let count = q
        .build_query_as::<(i64,)>()
        .fetch_one(pool)
        .await
        .map_err(|e| FnError::new(format!("count failed: {e}")))?;

    Ok(count.0)
}

/// Top chains by approved-tool count for sidebar filters.
#[cfg(feature = "ssr")]
pub async fn fetch_chain_counts(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<Vec<(String, i64)>, FnError> {
    let limit = limit.clamp(1, 100);
    let rows = sqlx::query_as::<_, (String, i64)>(CHAIN_COUNTS_SQL)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("chain counts failed: {e}")))?;

    Ok(rows)
}

/// List approved tools with sort, pagination, FTS query, and optional filters.
#[cfg(feature = "ssr")]
pub async fn fetch_list_tools(
    pool: &sqlx::PgPool,
    sort: &str,
    offset: i64,
    limit: i64,
    filters: &ToolFilters,
    query: Option<&str>,
) -> Result<Vec<Tool>, FnError> {
    use crate::server::tool_search::ToolSearchMatch;

    let offset = offset.max(0);
    let limit = clamp_list_tools_limit(limit);
    let search_text = query.filter(|q| !q.trim().is_empty());
    let match_mode = if let Some(text) = search_text {
        resolve_list_search_match(pool, text, filters).await?
    } else {
        ToolSearchMatch::And
    };
    let mut q = sqlx::QueryBuilder::new(
        "SELECT *, \
         ((CASE WHEN repo_url IS NOT NULL THEN 1 ELSE 0 END) \
         + (CASE WHEN stars > 0 THEN 1 ELSE 0 END) \
         + (CASE WHEN coalesce(array_length(chains, 1), 0) > 0 THEN 1 ELSE 0 END) \
         + (CASE WHEN install_command IS NOT NULL THEN 1 ELSE 0 END) \
         + (CASE WHEN last_commit_at IS NOT NULL THEN 1 ELSE 0 END))::int AS metadata_quality \
         FROM tools WHERE ",
    );
    q.push(PUBLIC_TOOL_WHERE);
    push_list_query_filter(&mut q, query, match_mode);
    append_tool_filters(&mut q, filters);
    let search_for_order = search_text.map(|text| (text, match_mode));
    push_list_order_offset_limit(&mut q, sort, offset, limit, search_for_order);

    let tools = q
        .build_query_as::<Tool>()
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("list tools failed: {e}")))?;

    Ok(sanitize_tools_for_public_response(tools))
}

/// REST/MCP search: apply full intent (FTS + axis filters) with relevance ranking.
#[cfg(feature = "ssr")]
pub async fn fetch_search_by_intent(
    pool: &sqlx::PgPool,
    intent: &crate::server::tool_search::ResolvedSearchIntent,
) -> Result<Vec<Tool>, FnError> {
    use crate::server::tool_search::{meaningful_token_count, ToolSearchMatch};

    let filters = intent_to_tool_filters(intent);
    validate_tool_filters(&filters)?;
    let search_text = intent.query.trim();
    let has_fts = !search_text.is_empty();
    let match_mode = if has_fts {
        resolve_list_search_match(pool, search_text, &filters).await?
    } else {
        ToolSearchMatch::And
    };

    let mut q = sqlx::QueryBuilder::new(
        "SELECT *, \
         ((CASE WHEN repo_url IS NOT NULL THEN 1 ELSE 0 END) \
         + (CASE WHEN stars > 0 THEN 1 ELSE 0 END) \
         + (CASE WHEN coalesce(array_length(chains, 1), 0) > 0 THEN 1 ELSE 0 END) \
         + (CASE WHEN install_command IS NOT NULL THEN 1 ELSE 0 END) \
         + (CASE WHEN last_commit_at IS NOT NULL THEN 1 ELSE 0 END))::int AS metadata_quality \
         FROM tools WHERE ",
    );
    q.push(PUBLIC_TOOL_WHERE);
    if has_fts {
        push_list_query_filter(&mut q, Some(search_text), match_mode);
    }
    append_tool_filters(&mut q, &filters);
    if has_fts {
        push_fts_rank_order_clause(&mut q, search_text, match_mode);
    } else {
        // Axis-token extraction (e.g. "base mcp" -> chain=base, tool_type=mcp)
        // can empty `search_text` entirely. Rank (not filter) by the untouched
        // raw query so a literal name match like "Base MCP" still surfaces
        // first instead of falling back to a pure stars/date sort.
        let raw_query = intent.raw_query.trim();
        if raw_query.is_empty() {
            q.push(" ORDER BY metadata_quality DESC, stars DESC, created_at DESC");
        } else {
            push_fts_rank_order_clause(&mut q, raw_query, ToolSearchMatch::And);
        }
    }
    q.push(" LIMIT 50");

    let mut tools = q
        .build_query_as::<Tool>()
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("search failed: {e}")))?;

    tools = sanitize_tools_for_public_response(tools);

    // Exact slug match: prepend when query is a single slug-like token (e.g. "x402-foundation").
    if has_fts && meaningful_token_count(search_text) == 1 && looks_like_tool_slug(search_text) {
        if let Some(slug_tool) = fetch_tool_by_slug(pool, search_text).await? {
            if !tools.iter().any(|t| t.id == slug_tool.id) {
                tools.insert(0, slug_tool);
            }
        }
    }

    Ok(tools)
}

