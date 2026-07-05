//! Shared SQL fragments and static public-tool queries.
// Goal harness deliverable AC2
// harness-round-7: 2026-06-25T19:10:00Z-queries

/// WHERE clause fragment: only publicly visible tools (approval + relevance + safety + quarantine).
macro_rules! public_tool_where {
    () => {
        "approval_status = 'approved' \
AND relevance_status = 'accepted' \
AND NOT (crypto_relevance_score = 0 \
AND 'migration-backfill: crypto keyword in name or description' = ANY(crypto_relevance_reasons)) \
AND install_risk_level <> 'critical' \
AND quarantined_at IS NULL"
    };
}

pub const PUBLIC_TOOL_WHERE: &str = public_tool_where!();

/// Alias kept during migration — all public queries should use this constant.
pub const TOOLS_APPROVED_WHERE: &str = PUBLIC_TOOL_WHERE;

pub const RECENT_APPROVED_TOOLS_SQL: &str = concat!(
    "SELECT * FROM tools WHERE ",
    public_tool_where!(),
    " ORDER BY stars DESC, created_at DESC LIMIT $1"
);

pub const APPROVED_TOOL_BY_SLUG_SQL: &str = concat!(
    "SELECT * FROM tools WHERE slug = $1 AND ",
    public_tool_where!()
);

pub const APPROVED_TOOL_ID_BY_SLUG_SQL: &str = concat!(
    "SELECT id FROM tools WHERE slug = $1 AND ",
    public_tool_where!()
);

pub const COUNT_APPROVED_TOOLS_SQL: &str =
    concat!("SELECT COUNT(*) FROM tools WHERE ", public_tool_where!());

pub const LIST_APPROVED_TOOLS_SQL: &str =
    concat!("SELECT * FROM tools WHERE ", public_tool_where!());

pub const CHAIN_COUNTS_SQL: &str = concat!(
    "SELECT chain, COUNT(*) AS count FROM tools, UNNEST(chains) AS chain WHERE ",
    public_tool_where!(),
    " GROUP BY chain ORDER BY count DESC, chain ASC LIMIT $1"
);

pub const CATEGORIES_WITH_COUNTS_SQL: &str = concat!(
    r#"
        SELECT c.id, c.label, c.icon, c.description, c.sort_order,
               COUNT(t.id) AS count
        FROM categories c
        LEFT JOIN tools t ON t.function = c.id AND t."#,
    public_tool_where!(),
    r#"
        GROUP BY c.id, c.label, c.icon, c.description, c.sort_order
        ORDER BY c.sort_order ASC
    "#
);

pub const SEARCH_APPROVED_TOOLS_SQL: &str = concat!(
    r#"
        SELECT *
        FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
            @@ plainto_tsquery('english', $1)
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
        ORDER BY ts_rank_cd(
            to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')),
            plainto_tsquery('english', $1)
        ) DESC, stars DESC, created_at DESC
        LIMIT 50
    "#
);

pub const SEARCH_APPROVED_TOOLS_OR_SQL: &str = concat!(
    r#"
        SELECT *
        FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
            @@ to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | '))
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
        ORDER BY ts_rank_cd(
            to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')),
            to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | '))
        ) DESC, stars DESC, created_at DESC
        LIMIT 50
    "#
);

pub const MCP_SEARCH_TOOLS_COUNT_SQL: &str = concat!(
    r#"
        SELECT COUNT(*)::bigint FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
              @@ plainto_tsquery('english', $1)
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
    "#
);

pub const MCP_SEARCH_TOOLS_COUNT_OR_SQL: &str = concat!(
    r#"
        SELECT COUNT(*)::bigint FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
              @@ to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | '))
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
    "#
);

pub const MCP_SEARCH_TOOLS_BASE_SQL: &str = concat!(
    r#"
        SELECT * FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
              @@ plainto_tsquery('english', $1)
    "#
);

pub const MCP_SEARCH_TOOLS_RELEVANCE_SQL: &str = concat!(
    r#"
        SELECT * FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
            @@ plainto_tsquery('english', $1)
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
        ORDER BY ts_rank_cd(
            to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')),
            plainto_tsquery('english', $1)
        ) DESC, stars DESC, updated_at DESC
        LIMIT $4 OFFSET $5
    "#
);

pub const MCP_SEARCH_TOOLS_RELEVANCE_OR_SQL: &str = concat!(
    r#"
        SELECT * FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
            @@ to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | '))
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
        ORDER BY ts_rank_cd(
            to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '')),
            to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | '))
        ) DESC, stars DESC, updated_at DESC
        LIMIT $4 OFFSET $5
    "#
);

pub const MCP_SEARCH_TOOLS_TRUST_SQL: &str = concat!(
    r#"
        SELECT * FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
              @@ plainto_tsquery('english', $1)
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
        ORDER BY stars DESC, updated_at DESC
        LIMIT $4 OFFSET $5
    "#
);

pub const MCP_SEARCH_TOOLS_TRUST_OR_SQL: &str = concat!(
    r#"
        SELECT * FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
              @@ to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | '))
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
        ORDER BY stars DESC, updated_at DESC
        LIMIT $4 OFFSET $5
    "#
);

pub const MCP_SEARCH_TOOLS_STARS_SQL: &str = concat!(
    r#"
        SELECT * FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
              @@ plainto_tsquery('english', $1)
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
        ORDER BY stars DESC, updated_at DESC
        LIMIT $4 OFFSET $5
    "#
);

pub const MCP_SEARCH_TOOLS_STARS_OR_SQL: &str = concat!(
    r#"
        SELECT * FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
              @@ to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | '))
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
        ORDER BY stars DESC, updated_at DESC
        LIMIT $4 OFFSET $5
    "#
);

pub const MCP_SEARCH_TOOLS_RECENT_SQL: &str = concat!(
    r#"
        SELECT * FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
              @@ plainto_tsquery('english', $1)
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
        ORDER BY updated_at DESC, stars DESC
        LIMIT $4 OFFSET $5
    "#
);

pub const MCP_SEARCH_TOOLS_RECENT_OR_SQL: &str = concat!(
    r#"
        SELECT * FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
              @@ to_tsquery('english', replace(plainto_tsquery('english', $1)::text, ' & ', ' | '))
          AND ($2::text IS NULL OR function = $2)
          AND ($3::text IS NULL OR $3 = ANY(chains))
        ORDER BY updated_at DESC, stars DESC
        LIMIT $4 OFFSET $5
    "#
);

pub const DASHBOARD_TYPE_COUNTS_SQL: &str = concat!(
    r#"
        SELECT type AS id, COUNT(*)::bigint AS count
        FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND type IS NOT NULL
          AND type <> ''
        GROUP BY type
        ORDER BY count DESC, id ASC
        LIMIT $1
    "#
);

pub const DASHBOARD_STATUS_COUNTS_SQL: &str = concat!(
    r#"
        SELECT status AS id, COUNT(*)::bigint AS count
        FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND status IS NOT NULL
          AND status <> ''
        GROUP BY status
        ORDER BY count DESC, id ASC
        LIMIT $1
    "#
);

pub const DASHBOARD_PRICING_COUNTS_SQL: &str = concat!(
    r#"
        SELECT pricing AS id, COUNT(*)::bigint AS count
        FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND pricing IS NOT NULL
          AND pricing <> ''
        GROUP BY pricing
        ORDER BY count DESC, id ASC
        LIMIT $1
    "#
);

pub const DASHBOARD_FUNCTION_COUNTS_SQL: &str = concat!(
    r#"
        SELECT c.id, c.label, COUNT(t.id)::bigint AS count
        FROM categories c
        LEFT JOIN tools t ON t.function = c.id AND t."#,
    public_tool_where!(),
    r#"
        GROUP BY c.id, c.label, c.sort_order
        HAVING COUNT(t.id) > 0
        ORDER BY count DESC, c.sort_order ASC
        LIMIT $1
    "#
);

pub const DASHBOARD_X402_TOOLS_SQL: &str = concat!(
    r#"
        SELECT *
        FROM tools
        WHERE "#,
    public_tool_where!(),
    r#"
          AND (
            type = 'x402'
            OR pricing = 'x402'
            OR x402_price IS NOT NULL
            OR referral_enabled = true
          )
        ORDER BY stars DESC, created_at DESC
        LIMIT $1
    "#
);

pub const DASHBOARD_METRICS_SQL: &str = concat!(
    r#"
        SELECT
          COUNT(*)::bigint,
          COUNT(*) FILTER (WHERE type = 'mcp')::bigint,
          COUNT(*) FILTER (WHERE type = 'cli')::bigint,
          COUNT(*) FILTER (WHERE type = 'sdk')::bigint,
          COUNT(*) FILTER (WHERE type = 'api')::bigint,
          COUNT(*) FILTER (
            WHERE type = 'x402' OR pricing = 'x402' OR x402_price IS NOT NULL
          )::bigint,
          COUNT(*) FILTER (WHERE status = 'official')::bigint,
          COUNT(*) FILTER (WHERE status = 'verified')::bigint,
          COUNT(*) FILTER (WHERE updated_at >= now() - interval '30 days')::bigint
        FROM tools
        WHERE "#,
    public_tool_where!()
);

pub const USER_TOOLKIT_SQL: &str = concat!(
    r#"
        SELECT t.*,
               b.note AS bookmark_note,
               b.tags AS bookmark_tags,
               b.source AS bookmark_source,
               b.source_client AS bookmark_source_client,
               b.created_at AS bookmark_created_at,
               b.updated_at AS bookmark_updated_at
        FROM bookmarks b
        JOIN tools t ON t.id = b.tool_id
        WHERE b.user_id = $1 AND "#,
    public_tool_where!(),
    r#"
        ORDER BY b.updated_at DESC, b.created_at DESC
        LIMIT 200
    "#
);

pub const TOOL_COMMENT_COUNTS_BY_SLUGS_SQL: &str = concat!(
    r#"
        SELECT t.slug, COUNT(c.id)::bigint AS comment_count
        FROM tools t
        LEFT JOIN comments c ON c.tool_id = t.id
        WHERE t.slug = ANY($1) AND "#,
    public_tool_where!(),
    r#"
        GROUP BY t.slug
    "#
);

pub const IS_BOOKMARKED_SQL: &str = concat!(
    r#"
        SELECT COUNT(*)::bigint
        FROM bookmarks b
        JOIN tools t ON t.id = b.tool_id
        WHERE t.slug = $1 AND b.user_id = $2 AND "#,
    public_tool_where!()
);

pub const APPROVED_TOOLS_BY_SLUGS_SQL: &str = concat!(
    "SELECT * FROM tools WHERE slug = ANY($1) AND ",
    public_tool_where!()
);

pub const BOOKMARKED_SLUGS_SQL: &str = concat!(
    r#"
        SELECT t.slug
        FROM bookmarks b
        JOIN tools t ON t.id = b.tool_id
        WHERE t.slug = ANY($1) AND b.user_id = $2 AND "#,
    public_tool_where!()
);

pub const TOOL_COMMENT_COUNT_BY_SLUG_SQL: &str = concat!(
    r#"
        SELECT COUNT(*)::bigint
        FROM comments c
        JOIN tools t ON t.id = c.tool_id
        WHERE t.slug = $1 AND "#,
    public_tool_where!()
);

pub const TOOL_COMMENTS_NEW_SORT_SQL: &str = r#"
        SELECT
            c.id, c.tool_id, c.parent_id, c.user_id, c.content, c.created_at,
            p.nickname AS author_nickname,
            p.auth_method AS author_auth_method,
            p.is_admin AS author_is_admin,
            COUNT(u.id) AS upvote_count,
            BOOL_OR(u.user_id = $2) AS viewer_upvoted
        FROM comments c
        JOIN profiles p ON p.id = c.user_id
        LEFT JOIN upvotes u ON u.comment_id = c.id
        WHERE c.tool_id = $1
        GROUP BY c.id, p.nickname, p.auth_method, p.is_admin
        ORDER BY c.created_at DESC
    "#;

pub const TOOL_COMMENTS_TOP_SORT_SQL: &str = r#"
        SELECT
            c.id, c.tool_id, c.parent_id, c.user_id, c.content, c.created_at,
            p.nickname AS author_nickname,
            p.auth_method AS author_auth_method,
            p.is_admin AS author_is_admin,
            COUNT(u.id) AS upvote_count,
            BOOL_OR(u.user_id = $2) AS viewer_upvoted
        FROM comments c
        JOIN profiles p ON p.id = c.user_id
        LEFT JOIN upvotes u ON u.comment_id = c.id
        WHERE c.tool_id = $1
        GROUP BY c.id, p.nickname, p.auth_method, p.is_admin
        ORDER BY COUNT(u.id) DESC, c.created_at DESC
    "#;

/// Whitelisted dashboard bucket axes — column names are never interpolated from callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardCountAxis {
    Type,
    Status,
    Pricing,
}

impl DashboardCountAxis {
    pub fn bucket_axis(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Status => "status",
            Self::Pricing => "pricing",
        }
    }

    pub fn count_sql(self) -> &'static str {
        match self {
            Self::Type => DASHBOARD_TYPE_COUNTS_SQL,
            Self::Status => DASHBOARD_STATUS_COUNTS_SQL,
            Self::Pricing => DASHBOARD_PRICING_COUNTS_SQL,
        }
    }
}

/// Approved-tool list sort keys allowed in public queries.
pub fn list_tools_order_clause(sort: &str) -> &'static str {
    match sort {
        "new" => "created_at DESC",
        "comments" => {
            "(SELECT COUNT(*)::bigint FROM comments cm WHERE cm.tool_id = tools.id) DESC, created_at DESC"
        }
        _ => "stars DESC, created_at DESC",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_tool_where_contains_all_visibility_conditions() {
        assert!(PUBLIC_TOOL_WHERE.contains("approval_status = 'approved'"));
        assert!(PUBLIC_TOOL_WHERE.contains("relevance_status = 'accepted'"));
        assert!(PUBLIC_TOOL_WHERE.contains("crypto_relevance_score = 0"));
        assert!(PUBLIC_TOOL_WHERE.contains("migration-backfill"));
        assert!(PUBLIC_TOOL_WHERE.contains("install_risk_level <> 'critical'"));
        assert!(PUBLIC_TOOL_WHERE.contains("quarantined_at IS NULL"));
        assert_eq!(TOOLS_APPROVED_WHERE, PUBLIC_TOOL_WHERE);
    }

    #[test]
    fn public_tool_where_excludes_legacy_backfill_only_rows() {
        assert!(PUBLIC_TOOL_WHERE.contains("AND NOT"));
        assert!(PUBLIC_TOOL_WHERE
            .contains("'migration-backfill: crypto keyword in name or description' = ANY(crypto_relevance_reasons)"));
    }

    /// Migration 015 broadened RLS from exact array equality (011) to `= ANY` containment.
    #[test]
    fn public_tool_where_matches_migration_015_rls_containment() {
        assert!(PUBLIC_TOOL_WHERE.contains("= ANY(crypto_relevance_reasons)"));
        assert!(!PUBLIC_TOOL_WHERE.contains("crypto_relevance_reasons = ARRAY["));
        assert!(!PUBLIC_TOOL_WHERE.contains("]::TEXT[]"));
    }

    #[test]
    fn public_tool_where_does_not_require_x402_payment_verification() {
        assert!(!PUBLIC_TOOL_WHERE.contains("payment_verified"));
        assert!(!PUBLIC_TOOL_WHERE.contains("x402_endpoint_verified"));
        assert!(!PUBLIC_TOOL_WHERE.contains("price_verified"));
        assert!(!PUBLIC_TOOL_WHERE.contains("referral_enabled = false"));
    }

    #[test]
    fn static_public_queries_embed_visibility_where() {
        for sql in [
            RECENT_APPROVED_TOOLS_SQL,
            APPROVED_TOOL_BY_SLUG_SQL,
            SEARCH_APPROVED_TOOLS_SQL,
            MCP_SEARCH_TOOLS_COUNT_SQL,
            COUNT_APPROVED_TOOLS_SQL,
            LIST_APPROVED_TOOLS_SQL,
            CHAIN_COUNTS_SQL,
            CATEGORIES_WITH_COUNTS_SQL,
            DASHBOARD_METRICS_SQL,
            USER_TOOLKIT_SQL,
        ] {
            assert!(
                sql.contains("approval_status = 'approved'"),
                "missing visibility gate: {sql}"
            );
        }
    }

    #[test]
    fn dashboard_count_axes_use_whitelisted_sql_only() {
        assert!(DASHBOARD_TYPE_COUNTS_SQL.contains("GROUP BY type"));
        assert!(DASHBOARD_STATUS_COUNTS_SQL.contains("GROUP BY status"));
        assert!(DASHBOARD_PRICING_COUNTS_SQL.contains("GROUP BY pricing"));
        assert_eq!(DashboardCountAxis::Type.bucket_axis(), "type");
        assert_eq!(
            DashboardCountAxis::Pricing.count_sql(),
            DASHBOARD_PRICING_COUNTS_SQL
        );
    }

    #[test]
    fn categories_join_prefixes_first_predicate_with_table_alias() {
        assert!(CATEGORIES_WITH_COUNTS_SQL.contains("AND t.approval_status = 'approved'"));
    }
}
