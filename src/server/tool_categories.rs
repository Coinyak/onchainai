//! Canonical OnchainAI public tool category ids.
//!
//! Shared by MCP `search_tools` validation, inputSchema enums, and web browse
//! filters so category whitelists cannot drift from each other.

/// Public tool function/category ids accepted by MCP `search_tools.category`.
pub const PUBLIC_TOOL_CATEGORY_IDS: &[&str] = &[
    "bridge",
    "swap",
    "wallet",
    "payments",
    "lending",
    "staking",
    "trading",
    "nft",
    "data",
    "dev-tool",
    "identity",
    "governance",
    "social",
    "ai-agent",
];

/// Returns true when `category` is a known public tool function id.
pub fn is_public_tool_category(category: &str) -> bool {
    PUBLIC_TOOL_CATEGORY_IDS.contains(&category)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_tool_categories_has_fourteen_entries() {
        assert_eq!(PUBLIC_TOOL_CATEGORY_IDS.len(), 14);
        assert!(is_public_tool_category("dev-tool"));
        assert!(!is_public_tool_category("unknown"));
    }
}
