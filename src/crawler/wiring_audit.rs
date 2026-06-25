//! Static wiring audits — prove crawler sources persist via `persist_crawl_results`.

#[cfg(test)]
mod tests {
    #[test]
    fn all_four_sources_call_persist_crawl_results_in_run_once() {
        let sources = [
            ("github", include_str!("sources/github.rs")),
            ("npm", include_str!("sources/npm.rs")),
            ("cryptoskill", include_str!("sources/cryptoskill.rs")),
            ("web3mcp", include_str!("sources/web3mcp.rs")),
        ];
        for (name, src) in sources {
            assert!(
                src.contains("persist_crawl_results"),
                "{name} run_once must call persist_crawl_results to upsert tools"
            );
        }
    }

    #[test]
    fn github_topics_use_resolve_keywords_not_hardcoded_const() {
        let src = include_str!("sources/github.rs");
        assert!(
            src.contains("resolve_keywords"),
            "github crawler must read live search_keywords via resolve_keywords"
        );
        assert!(
            !src.contains("const TOPICS"),
            "compile-time TOPICS const must not remain in github.rs"
        );
    }

    #[test]
    fn persist_crawl_results_wires_approval_status_decision() {
        let src = include_str!("mod.rs");
        assert!(src.contains("initial_approval_status"));
        assert!(src.contains("normalize_batch_with_status"));
        assert!(src.contains("upsert_tools"));
    }
}