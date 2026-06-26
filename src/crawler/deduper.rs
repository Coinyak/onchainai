//! Deduper — removes duplicates by `repo_url`, keeping the highest-star entry.
//!
//! Tools with `None` repo_url are preserved (they may be distinct tools that
//! simply don't have a repository, e.g. hosted APIs). When two tools share the
//! same `repo_url`, the one with the higher `stars` count is kept; ties break
//! toward the first-seen entry (stable).

use crate::models::Tool;

/// Remove duplicate tools by `repo_url`.
///
/// Rules (per `docs/MVP_DESIGN.md` section 3 + VAL-CRAWL-012):
/// - Tools with `None` repo_url are **always preserved**.
/// - Among tools sharing the same `repo_url`, keep the one with the highest
///   `stars`. Ties keep the first-seen entry.
/// - Order of `None`-repo_url tools is preserved.
/// - Order of the kept `Some`-repo_url tools follows first occurrence of
///   their (winning) entry.
#[allow(dead_code)]
pub fn dedupe(tools: Vec<Tool>) -> Vec<Tool> {
    use std::collections::HashMap;

    // Map repo_url → best tool index in the output so far.
    let mut best_by_url: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<Tool> = Vec::with_capacity(tools.len());

    for tool in tools {
        match &tool.repo_url {
            None => out.push(tool),
            Some(url) => {
                if let Some(&idx) = best_by_url.get(url) {
                    // Collision: keep higher stars; tie → first-seen stays.
                    if tool.stars > out[idx].stars {
                        out[idx] = tool;
                    }
                } else {
                    best_by_url.insert(url.clone(), out.len());
                    out.push(tool);
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_tool(name: &str, repo_url: Option<&str>, stars: i32) -> Tool {
        let review = crate::models::tool::default_review_fields();
        Tool {
            id: Uuid::new_v4(),
            name: name.into(),
            slug: slug::slugify(name),
            description: None,
            function: "dev-tool".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "mcp".into(),
            repo_url: repo_url.map(|s| s.to_string()),
            homepage: None,
            npm_package: None,
            install_command: None,
            mcp_endpoint: None,
            chains: vec![],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "approved".into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: review.crypto_relevance_score,
            crypto_relevance_reasons: review.crypto_relevance_reasons,
            relevance_status: review.relevance_status,
            install_risk_level: review.install_risk_level,
            install_risk_reasons: review.install_risk_reasons,
            requires_secret: review.requires_secret,
            safe_copy_command: review.safe_copy_command,
            quarantined_at: review.quarantined_at,
            last_reviewed_at: review.last_reviewed_at,
            review_policy_version: review.review_policy_version,
            claim_state: "unclaimed".into(),
            license: None,
            pricing: "free".into(),
            x402_price: None,
            stars,
            last_commit_at: None,
            source: "github".into(),
            source_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(dedupe(Vec::new()).is_empty());
    }

    #[test]
    fn no_duplicates_all_preserved() {
        let tools = vec![
            make_tool("A", Some("https://github.com/a/a"), 1),
            make_tool("B", Some("https://github.com/b/b"), 2),
        ];
        let out = dedupe(tools);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn duplicate_repo_url_keeps_higher_stars() {
        let tools = vec![
            make_tool("A-low", Some("https://github.com/a/a"), 10),
            make_tool("A-high", Some("https://github.com/a/a"), 500),
        ];
        let out = dedupe(tools);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "A-high");
        assert_eq!(out[0].stars, 500);
    }

    #[test]
    fn duplicate_repo_url_tie_keeps_first() {
        let tools = vec![
            make_tool("First", Some("https://github.com/a/a"), 100),
            make_tool("Second", Some("https://github.com/a/a"), 100),
        ];
        let out = dedupe(tools);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "First");
    }

    #[test]
    fn none_repo_url_all_preserved() {
        let tools = vec![
            make_tool("A", None, 1),
            make_tool("B", None, 2),
            make_tool("C", None, 3),
        ];
        let out = dedupe(tools);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn mixed_none_and_duplicates() {
        let tools = vec![
            make_tool("None-A", None, 5),
            make_tool("Dup-low", Some("https://github.com/x/x"), 1),
            make_tool("None-B", None, 7),
            make_tool("Dup-high", Some("https://github.com/x/x"), 999),
        ];
        let out = dedupe(tools);
        // 2 None + 1 deduped repo = 3 total.
        assert_eq!(out.len(), 3);
        assert!(out.iter().any(|t| t.name == "None-A"));
        assert!(out.iter().any(|t| t.name == "None-B"));
        assert!(out.iter().any(|t| t.name == "Dup-high" && t.stars == 999));
    }

    #[test]
    fn three_way_duplicate_keeps_max() {
        let tools = vec![
            make_tool("D1", Some("https://github.com/d/d"), 50),
            make_tool("D2", Some("https://github.com/d/d"), 300),
            make_tool("D3", Some("https://github.com/d/d"), 100),
        ];
        let out = dedupe(tools);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].stars, 300);
        assert_eq!(out[0].name, "D2");
    }

    #[test]
    fn distinct_urls_preserved_in_order() {
        let tools = vec![
            make_tool("A", Some("https://github.com/a/a"), 1),
            make_tool("B", Some("https://github.com/b/b"), 2),
            make_tool("A2", Some("https://github.com/a/a"), 3),
            make_tool("C", Some("https://github.com/c/c"), 4),
        ];
        let out = dedupe(tools);
        // a (stars=3 wins), b, c → 3 entries.
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].name, "A2");
        assert_eq!(out[0].stars, 3);
        assert_eq!(out[1].name, "B");
        assert_eq!(out[2].name, "C");
    }
}
