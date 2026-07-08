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
/// - Among tools sharing the same `repo_url` **and** the same
///   `install_command`, keep the one with the highest `stars`. Ties keep
///   the first-seen entry. This prevents the same installable package from
///   appearing twice (e.g. crawled by both npm and vendor_orgs).
/// - Tools that share a `repo_url` but have **different** install commands
///   are treated as distinct packages (e.g. `pkg` + `pkg-core` from the
///   same monorepo) and are both preserved.
/// - If a `repo_url` group has both install-identified entries (from npm/PyPI)
///   and a placeholder with no `install_command` (e.g. from `vendor_orgs`),
///   the placeholder is **dropped** to avoid cross-source duplicates.
/// - Order of `None`-repo_url tools is preserved.
/// - Order of the kept `Some`-repo_url tools follows first occurrence of
///   their (winning) entry.
#[allow(dead_code)]
pub fn dedupe(tools: Vec<Tool>) -> Vec<Tool> {
    use std::collections::HashMap;

    // Phase 1: Group by repo_url, dedupe within each (repo_url, install) key.
    //
    // Map (repo_url, install_command) → best tool index in the output so far.
    let mut best_by_key: HashMap<(String, String), usize> = HashMap::new();
    // Track which output indices are "no-install" placeholders per repo_url,
    // so we can drop them if a real install-identified entry exists for the
    // same repo.
    let mut no_install_indices: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<Tool> = Vec::with_capacity(tools.len());

    for tool in tools {
        match &tool.repo_url {
            None => out.push(tool),
            Some(url) => {
                let install = tool
                    .install_command
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let has_install = !install.is_empty();
                let key = (url.clone(), install);

                if let Some(&idx) = best_by_key.get(&key) {
                    // Collision: keep higher stars; tie → first-seen stays.
                    if tool.stars > out[idx].stars {
                        out[idx] = tool;
                    }
                } else {
                    best_by_key.insert(key, out.len());
                    if !has_install {
                        no_install_indices.insert(url.clone(), out.len());
                    }
                    out.push(tool);
                }
            }
        }
    }

    // Phase 2: Drop no-install placeholders when install-identified entries
    // exist for the same repo_url. This prevents vendor_orgs entries (which
    // have install_command=None) from surviving as duplicates when npm/PyPI
    // already produced a concrete installable tool for the same repo.
    let mut dropped_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for (url, &placeholder_idx) in &no_install_indices {
        // Check if any other entry for this repo_url has a real install_command.
        let has_install_entry = out.iter().enumerate().any(|(i, t)| {
            i != placeholder_idx
                && t.repo_url.as_deref() == Some(url.as_str())
                && t.install_command
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty())
        });
        if has_install_entry {
            dropped_indices.insert(placeholder_idx);
        }
    }

    if dropped_indices.is_empty() {
        return out;
    }

    out.into_iter()
        .enumerate()
        .filter_map(|(i, t)| {
            if dropped_indices.contains(&i) {
                None
            } else {
                Some(t)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_tool(name: &str, repo_url: Option<&str>, stars: i32) -> Tool {
        make_tool_with_install(name, repo_url, stars, None)
    }

    fn make_tool_with_install(
        name: &str,
        repo_url: Option<&str>,
        stars: i32,
        install: Option<&str>,
    ) -> Tool {
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
            install_command: install.map(|s| s.to_string()),
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
            referral_enabled: false,
            referral_bps: None,
            referral_payout_address: None,
            referral_model: None,
            x402_pay_to_address: None,
            x402_builder_code: None,
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
            x402_endpoint: None,
            x402_last_checked_at: None,
            x402_check_failures: 0,
            stars,
            last_commit_at: None,
            source: "github".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
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

    #[test]
    fn same_repo_different_install_both_preserved() {
        // Simulates bnbagent-studio + bnbagent-studio-core: same GitHub repo
        // URL, different pip install commands → both should be kept.
        let tools = vec![
            make_tool_with_install(
                "bnbagent-studio",
                Some("https://github.com/bnb-chain/bnbagent-studio"),
                0,
                Some("pip install bnbagent-studio"),
            ),
            make_tool_with_install(
                "bnbagent-studio-core",
                Some("https://github.com/bnb-chain/bnbagent-studio"),
                0,
                Some("pip install bnbagent-studio-core"),
            ),
        ];
        let out = dedupe(tools);
        assert_eq!(
            out.len(),
            2,
            "same repo + different install should both survive"
        );
        assert!(out.iter().any(|t| t.name == "bnbagent-studio"));
        assert!(out.iter().any(|t| t.name == "bnbagent-studio-core"));
    }

    #[test]
    fn same_repo_same_install_dedupes() {
        // Same repo + same install (e.g. crawled by two sources) → dedupe.
        let tools = vec![
            make_tool_with_install("via-npm", Some("https://github.com/x/x"), 10, Some("npx x")),
            make_tool_with_install(
                "via-vendor",
                Some("https://github.com/x/x"),
                500,
                Some("npx x"),
            ),
        ];
        let out = dedupe(tools);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "via-vendor");
        assert_eq!(out[0].stars, 500);
    }

    #[test]
    fn cross_source_vendor_placeholder_dropped_when_install_exists() {
        // vendor_orgs produces install_command=None; npm produces
        // install_command=Some("npx ...") for the same repo_url.
        // The no-install placeholder should be dropped to avoid duplicates.
        let tools = vec![
            make_tool_with_install(
                "vendor-placeholder",
                Some("https://github.com/x/x"),
                0,
                None,
            ),
            make_tool_with_install(
                "npm-real",
                Some("https://github.com/x/x"),
                42,
                Some("npx x"),
            ),
        ];
        let out = dedupe(tools);
        assert_eq!(out.len(), 1, "placeholder should be dropped");
        assert_eq!(out[0].name, "npm-real");
    }

    #[test]
    fn whitespace_only_install_treated_as_no_install() {
        let tools = vec![
            make_tool_with_install(
                "vendor-placeholder",
                Some("https://github.com/x/x"),
                0,
                None,
            ),
            make_tool_with_install(
                "whitespace-only",
                Some("https://github.com/x/x"),
                5,
                Some("   "),
            ),
            make_tool_with_install(
                "npm-real",
                Some("https://github.com/x/x"),
                42,
                Some("npx x"),
            ),
        ];
        let out = dedupe(tools);
        assert_eq!(
            out.len(),
            1,
            "whitespace-only install must not survive as a distinct key"
        );
        assert_eq!(out[0].name, "npm-real");
    }

    #[test]
    fn cross_source_placeholder_kept_when_no_install_entry() {
        // vendor_orgs placeholder with no install entry from any source
        // should be preserved (it's the only signal for that repo).
        let tools = vec![make_tool_with_install(
            "vendor-only",
            Some("https://github.com/y/y"),
            5,
            None,
        )];
        let out = dedupe(tools);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "vendor-only");
    }
}
