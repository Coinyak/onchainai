//! Shared SQL fragments for public tool queries.
// Goal harness deliverable AC2
// harness-round-7: 2026-06-25T19:10:00Z-queries

/// WHERE clause fragment: only publicly visible tools (approval + relevance + safety + quarantine).
pub const PUBLIC_TOOL_WHERE: &str = "\
approval_status = 'approved' \
AND relevance_status = 'accepted' \
AND NOT (crypto_relevance_score = 0 \
AND crypto_relevance_reasons = ARRAY['migration-backfill: crypto keyword in name or description']::TEXT[]) \
AND install_risk_level <> 'critical' \
AND quarantined_at IS NULL";

/// Alias kept during migration — all public queries should use this constant.
pub const TOOLS_APPROVED_WHERE: &str = PUBLIC_TOOL_WHERE;

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
            .contains("crypto_relevance_reasons = ARRAY['migration-backfill: crypto keyword in name or description']::TEXT[]"));
    }
}
