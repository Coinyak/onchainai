//! Shared SQL fragments for public tool queries.
// Goal harness deliverable AC2
// harness-round-7: 2026-06-25T19:10:00Z-queries

/// WHERE clause fragment: only publicly visible tools.
pub const TOOLS_APPROVED_WHERE: &str = "approval_status = 'approved'";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_where_is_stable_literal() {
        assert_eq!(TOOLS_APPROVED_WHERE, "approval_status = 'approved'");
    }
}
