//! Crawler runtime settings loaded from `site_settings`.
// Goal harness deliverable AC3/AC4
// harness-round-7: 2026-06-25T19:10:00Z-settings

use crate::models::SiteSettings;

/// Default GitHub topic keywords when `site_settings.search_keywords` is empty.
pub const DEFAULT_SEARCH_KEYWORDS: &[&str] = &[
    "mcp-server",
    "crypto-mcp",
    "web3-mcp",
    "blockchain-mcp",
    "ai-agent",
    "agent-sdk",
    "bnb",
    "onchain-agent",
];

/// Runtime crawler configuration from the `site_settings` singleton.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlerSettings {
    pub require_tool_approval: bool,
    pub search_keywords: Vec<String>,
}

impl Default for CrawlerSettings {
    fn default() -> Self {
        Self {
            require_tool_approval: true,
            search_keywords: DEFAULT_SEARCH_KEYWORDS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        }
    }
}

/// Map `require_tool_approval` to the initial `approval_status` for newly crawled tools.
pub fn initial_approval_status(require_tool_approval: bool) -> &'static str {
    if require_tool_approval {
        "pending"
    } else {
        "approved"
    }
}

/// Resolve GitHub topic keywords, falling back to [`DEFAULT_SEARCH_KEYWORDS`] when empty.
pub fn resolve_keywords(keywords: &[String]) -> Vec<String> {
    if keywords.is_empty() {
        DEFAULT_SEARCH_KEYWORDS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    } else {
        keywords.to_vec()
    }
}

/// Load crawler settings from `site_settings` (id = 1).
///
/// Tolerant of a missing row — returns schema-aligned defaults.
pub async fn load_crawler_settings(pool: &sqlx::PgPool) -> CrawlerSettings {
    match sqlx::query_as::<_, SiteSettings>("SELECT * FROM site_settings WHERE id = 1")
        .fetch_optional(pool)
        .await
    {
        Ok(Some(row)) => CrawlerSettings {
            require_tool_approval: row.require_tool_approval,
            search_keywords: row.search_keywords,
        },
        Ok(None) | Err(_) => CrawlerSettings::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_approval_status_pending_when_required() {
        assert_eq!(initial_approval_status(true), "pending");
    }

    #[test]
    fn initial_approval_status_approved_when_not_required() {
        assert_eq!(initial_approval_status(false), "approved");
    }

    #[test]
    fn resolve_keywords_falls_back_to_defaults() {
        let kw = resolve_keywords(&[]);
        assert_eq!(kw.len(), DEFAULT_SEARCH_KEYWORDS.len());
        assert!(kw.contains(&"mcp-server".to_string()));
    }

    #[test]
    fn resolve_keywords_uses_live_settings() {
        let live = vec!["defi-mcp".into(), "agent-wallet".into()];
        assert_eq!(resolve_keywords(&live), live);
    }
}
