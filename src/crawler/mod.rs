//! Crawler — auto-discovery orchestrator and scheduler.
//!
//! See `docs/MVP_DESIGN.md` section 3 for the full design. The orchestrator
//! runs all registered sources in parallel (`tokio::spawn`), collects their
//! results, normalizes raw tools into [`Tool`]s, deduplicates by `repo_url`,
//! and upserts into the database. Per-source errors are logged and do not
//! abort the run.
//!
//! This module (crawler-core) implements the trait, normalizer, deduper, and
//! orchestrator. Source implementations land in the `crawler-sources`
//! milestone; the orchestrator is written against the [`SourceCrawler`] trait
//! so it works with any set of sources.

pub mod deduper;
pub mod normalizer;
pub mod scheduler;
pub mod sources;

use std::sync::Arc;

use sources::SourceCrawler;

/// Run a set of source crawlers in parallel, normalize + dedupe the results.
///
/// Each source runs in its own `tokio::spawn` task. Errors from individual
/// sources are logged via `tracing::error!` and do not abort the other
/// sources. Returns the combined, normalized, deduplicated tool list.
///
/// The DB upsert step (writing tools into the `tools` table) is added in the
/// `crawler-scheduler-star-sync` milestone; this function performs the
/// in-memory pipeline only, which is what unit tests exercise.
pub async fn run_pipeline(crawlers: Vec<Arc<dyn SourceCrawler>>) -> Vec<models::Tool> {
    use std::collections::HashSet;

    // Spawn one task per source so they run concurrently.
    let mut handles = Vec::with_capacity(crawlers.len());
    for crawler in crawlers {
        let handle = tokio::spawn(async move {
            let name = crawler.source_name().to_string();
            match crawler.crawl().await {
                Ok(raws) => {
                    tracing::info!(
                        source = %name,
                        count = raws.len(),
                        "source crawl completed"
                    );
                    (name, Ok(raws))
                }
                Err(e) => {
                    tracing::error!(source = %name, error = %e, "source crawl failed");
                    (name, Err(e))
                }
            }
        });
        handles.push(handle);
    }

    // Collect raw tools from all successful sources.
    let mut all_raws: Vec<normalizer::RawTool> = Vec::new();
    for handle in handles {
        match handle.await {
            Ok((_name, Ok(raws))) => all_raws.extend(raws),
            Ok((_name, Err(_e))) => continue, // already logged
            Err(join_err) => tracing::error!(error = %join_err, "crawler task panicked"),
        }
    }

    // Normalize → dedupe.
    let tools = normalizer::normalize_batch(&all_raws);
    let tools = deduper::dedupe(tools);

    // The set of taken slugs is only used within normalize_batch; this
    // assertion keeps the variable alive for documentation and future use.
    let _taken: HashSet<String> = tools.iter().map(|t| t.slug.clone()).collect();

    tools
}

/// Start the crawler scheduler.
///
/// Registers all cron jobs and blocks until the scheduler stops. Spawned as a
/// background task in `main`. Errors are logged, not propagated to callers
/// (the caller is a `tokio::spawn` task).
pub async fn start_scheduler(pool: sqlx::PgPool) -> anyhow::Result<()> {
    scheduler::start(pool).await
}

/// Convenience alias so internal code can refer to `models::Tool` without
/// re-importing at each use site.
mod models {
    pub use crate::models::Tool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crawler::normalizer::RawTool;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A stub source that returns a fixed list of raw tools.
    struct StubSource {
        name: &'static str,
        interval: &'static str,
        raws: Vec<RawTool>,
    }

    #[async_trait]
    impl SourceCrawler for StubSource {
        async fn crawl(&self) -> anyhow::Result<Vec<RawTool>> {
            Ok(self.raws.clone())
        }
        fn source_name(&self) -> &str {
            self.name
        }
        fn interval(&self) -> &'static str {
            self.interval
        }
    }

    /// A stub source that always errors.
    struct FailingSource {
        name: &'static str,
    }

    #[async_trait]
    impl SourceCrawler for FailingSource {
        async fn crawl(&self) -> anyhow::Result<Vec<RawTool>> {
            Err(anyhow::anyhow!("simulated crawl failure"))
        }
        fn source_name(&self) -> &str {
            self.name
        }
        fn interval(&self) -> &'static str {
            "0 0 * * * *"
        }
    }

    fn raw(name: &str, repo: Option<&str>, stars: i32, desc: &str) -> RawTool {
        RawTool {
            name: name.into(),
            description: Some(desc.into()),
            tool_type: "mcp".into(),
            repo_url: repo.map(|s| s.to_string()),
            homepage: None,
            npm_package: None,
            install_command: None,
            mcp_endpoint: None,
            chains: vec![],
            stars,
            last_commit_at: None,
            source: "stub".into(),
            source_url: None,
            license: None,
        }
    }

    #[tokio::test]
    async fn pipeline_runs_sources_in_parallel_and_merges() {
        let crawlers: Vec<Arc<dyn SourceCrawler>> = vec![
            Arc::new(StubSource {
                name: "alpha",
                interval: "0 0 * * * *",
                raws: vec![
                    raw(
                        "Alpha Bridge",
                        Some("https://github.com/a/a"),
                        10,
                        "bridge cross-chain",
                    ),
                    raw(
                        "Beta Swap",
                        Some("https://github.com/b/b"),
                        20,
                        "uniswap dex swap",
                    ),
                ],
            }),
            Arc::new(StubSource {
                name: "beta",
                interval: "0 0 * * * *",
                raws: vec![raw("Gamma Agent", None, 5, "autonomous ai agent eliza")],
            }),
        ];
        let tools = run_pipeline(crawlers).await;
        assert_eq!(tools.len(), 3);
        assert!(tools.iter().any(|t| t.function == "bridge"));
        assert!(tools.iter().any(|t| t.function == "swap"));
        assert!(tools
            .iter()
            .any(|t| t.actor == "ai-agent" && t.name == "Gamma Agent"));
    }

    #[tokio::test]
    async fn pipeline_dedupes_across_sources() {
        // Two sources return the same repo_url; deduper keeps higher stars.
        let crawlers: Vec<Arc<dyn SourceCrawler>> = vec![
            Arc::new(StubSource {
                name: "alpha",
                interval: "0 0 * * * *",
                raws: vec![raw(
                    "Low",
                    Some("https://github.com/dup/dup"),
                    1,
                    "swap dex",
                )],
            }),
            Arc::new(StubSource {
                name: "beta",
                interval: "0 0 * * * *",
                raws: vec![raw(
                    "High",
                    Some("https://github.com/dup/dup"),
                    999,
                    "swap dex",
                )],
            }),
        ];
        let tools = run_pipeline(crawlers).await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].stars, 999);
    }

    #[tokio::test]
    async fn pipeline_continues_when_source_fails() {
        let call_count = Arc::new(AtomicUsize::new(0));
        struct CountingStub {
            count: Arc<AtomicUsize>,
        }
        #[async_trait]
        impl SourceCrawler for CountingStub {
            async fn crawl(&self) -> anyhow::Result<Vec<RawTool>> {
                self.count.fetch_add(1, Ordering::SeqCst);
                Ok(vec![raw("Survivor", None, 1, "staking yield")])
            }
            fn source_name(&self) -> &str {
                "survivor"
            }
            fn interval(&self) -> &'static str {
                "0 0 * * * *"
            }
        }
        let crawlers: Vec<Arc<dyn SourceCrawler>> = vec![
            Arc::new(FailingSource { name: "failer" }),
            Arc::new(CountingStub {
                count: call_count.clone(),
            }),
        ];
        let tools = run_pipeline(crawlers).await;
        // Failing source contributed nothing; survivor source still ran.
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "Survivor");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn pipeline_empty_sources_returns_empty() {
        let crawlers: Vec<Arc<dyn SourceCrawler>> = vec![];
        let tools = run_pipeline(crawlers).await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn pipeline_unique_slugs_across_sources() {
        // Two distinct tools with the same name from different sources.
        let crawlers: Vec<Arc<dyn SourceCrawler>> = vec![
            Arc::new(StubSource {
                name: "alpha",
                interval: "0 0 * * * *",
                raws: vec![raw("Same Name", None, 1, "bridge")],
            }),
            Arc::new(StubSource {
                name: "beta",
                interval: "0 0 * * * *",
                raws: vec![raw("Same Name", None, 2, "swap dex")],
            }),
        ];
        let tools = run_pipeline(crawlers).await;
        assert_eq!(tools.len(), 2);
        let slugs: Vec<_> = tools.iter().map(|t| t.slug.as_str()).collect();
        assert!(slugs.contains(&"same-name"));
        assert!(slugs.contains(&"same-name-2"));
    }
}
