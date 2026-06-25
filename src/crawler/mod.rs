//! Crawler — auto-discovery orchestrator and scheduler.
//!
//! See `docs/MVP_DESIGN.md` section 3 for the full design. The orchestrator
//! runs all registered sources in parallel (`tokio::spawn`), collects their
//! results, normalizes raw tools into [`Tool`]s, deduplicates by `repo_url`,
//! and upserts into the database. Per-source errors are logged and do not
//! abort the run.
//!
//! This module (crawler-core) implements the trait, normalizer, deduper,
//! orchestrator, source-status helpers, and the DB upsert used by the
//! `crawler-scheduler-star-sync` milestone.

pub mod deduper;
pub mod normalizer;
pub mod scheduler;
pub mod sources;

// Re-export normalizer helpers so the public module API is easy to consume.
use std::sync::Arc;

use sources::SourceCrawler;

/// Run a set of source crawlers in parallel, normalize + dedupe the results.
///
/// Each source runs in its own `tokio::spawn` task. Errors from individual
/// sources are logged via `tracing::error!` and do not abort the other
/// sources. Returns the combined, normalized, deduplicated tool list.
///
/// Callers that also want to persist results should pass the returned tools to
/// [`upsert_tools`] and update the `sources` table with [`update_source_status`].
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

/// Upsert a batch of crawled tools into the `tools` table.
///
/// Matching is by `slug` (unique). Existing rows keep their `status` and
/// `approval_status` when present (`official` / `verified` are preserved); all
/// other fields are overwritten with the freshly crawled values. This satisfies
/// VAL-CRAWL-014 (re-crawl preserves official/verified status).
pub async fn upsert_tools(pool: &sqlx::PgPool, tools: &[models::Tool]) -> anyhow::Result<()> {
    use anyhow::Context;

    for tool in tools {
        sqlx::query(
            r#"
            INSERT INTO tools (
                name, slug, description, function, asset_class, actor, type,
                repo_url, homepage, npm_package, install_command, mcp_endpoint,
                chains, status, official_team, trust_score, approval_status,
                submitted_by, rejection_reason, license, pricing, x402_price,
                stars, last_commit_at, source, source_url, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, now())
            ON CONFLICT (slug) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                function = EXCLUDED.function,
                asset_class = EXCLUDED.asset_class,
                actor = EXCLUDED.actor,
                type = EXCLUDED.type,
                repo_url = EXCLUDED.repo_url,
                homepage = EXCLUDED.homepage,
                npm_package = EXCLUDED.npm_package,
                install_command = EXCLUDED.install_command,
                mcp_endpoint = EXCLUDED.mcp_endpoint,
                chains = EXCLUDED.chains,
                status = CASE
                    WHEN tools.status IN ('official', 'verified') THEN tools.status
                    ELSE EXCLUDED.status
                END,
                official_team = COALESCE(tools.official_team, EXCLUDED.official_team),
                trust_score = EXCLUDED.trust_score,
                approval_status = COALESCE(NULLIF(tools.approval_status, ''), EXCLUDED.approval_status),
                submitted_by = tools.submitted_by,
                rejection_reason = tools.rejection_reason,
                license = EXCLUDED.license,
                pricing = EXCLUDED.pricing,
                x402_price = EXCLUDED.x402_price,
                stars = EXCLUDED.stars,
                last_commit_at = EXCLUDED.last_commit_at,
                source = EXCLUDED.source,
                source_url = EXCLUDED.source_url,
                updated_at = now()
            "#,
        )
        .bind(&tool.name)
        .bind(&tool.slug)
        .bind(&tool.description)
        .bind(&tool.function)
        .bind(&tool.asset_class)
        .bind(&tool.actor)
        .bind(&tool.tool_type)
        .bind(&tool.repo_url)
        .bind(&tool.homepage)
        .bind(&tool.npm_package)
        .bind(&tool.install_command)
        .bind(&tool.mcp_endpoint)
        .bind(&tool.chains)
        .bind(&tool.status)
        .bind(&tool.official_team)
        .bind(tool.trust_score)
        .bind(&tool.approval_status)
        .bind(tool.submitted_by)
        .bind(&tool.rejection_reason)
        .bind(&tool.license)
        .bind(&tool.pricing)
        .bind(&tool.x402_price)
        .bind(tool.stars)
        .bind(tool.last_commit_at)
        .bind(&tool.source)
        .bind(&tool.source_url)
        .bind(tool.created_at)
        .execute(pool)
        .await
        .with_context(|| format!("upserting tool slug={}", tool.slug))?;
    }

    Ok(())
}

/// Run the default production pipeline: all four source crawlers in parallel,
/// normalize, dedupe, upsert to DB, and update `sources` status.
///
/// Errors from individual sources are logged and do not abort the run; the
/// `sources` table is updated for both successes and failures.
#[allow(dead_code)]
pub async fn run_all_sources(pool: &sqlx::PgPool) {
    use crate::crawler::sources::{
        cryptoskill::CryptoSkillCrawler, github::GitHubTopicsCrawler, npm::NpmCrawler,
        web3mcp::Web3McpHubCrawler,
    };
    use std::sync::Arc;

    let crawlers: Vec<Arc<dyn SourceCrawler>> = vec![
        Arc::new(NpmCrawler),
        Arc::new(CryptoSkillCrawler),
        Arc::new(Web3McpHubCrawler),
        Arc::new(GitHubTopicsCrawler),
    ];

    // Run pipeline in memory first.
    let tools = run_pipeline(crawlers).await;

    // Upsert normalized tools. The total count is logged regardless of source origin.
    if let Err(e) = upsert_tools(pool, &tools).await {
        tracing::error!(error = %e, "failed to upsert crawled tools");
    } else {
        tracing::info!(count = tools.len(), "crawled tools upserted");
    }

    // Per-source status updates are written by the individual `run_once` helpers
    // (github/npm/cryptoskill/web3mcp) so exact source-level counts are preserved.
}

/// Update the `sources` table after a crawl finishes.
///
/// Inserts the source row if missing, then sets `last_crawled_at = now()`,
/// `crawl_status`, `items_found`, and an optional `error_message`.
pub async fn update_source_status(
    pool: &sqlx::PgPool,
    name: &str,
    url: &str,
    status: &str,
    items_found: i32,
    error_message: Option<&str>,
) {
    let result = sqlx::query(
        r#"
        INSERT INTO sources (name, url, last_crawled_at, crawl_status, items_found, error_message)
        VALUES ($1, $2, now(), $3, $4, $5)
        ON CONFLICT (name) DO UPDATE SET
            url = EXCLUDED.url,
            last_crawled_at = EXCLUDED.last_crawled_at,
            crawl_status = EXCLUDED.crawl_status,
            items_found = EXCLUDED.items_found,
            error_message = EXCLUDED.error_message,
            updated_at = now()
        "#,
    )
    .bind(name)
    .bind(url)
    .bind(status)
    .bind(items_found)
    .bind(error_message)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::error!(source = name, error = %e, "failed to update sources table");
    }
}

/// Convenience helper: update source status from a successful raw result set.
pub async fn upsert_source_results(
    pool: &sqlx::PgPool,
    name: &str,
    url: &str,
    raws: Vec<normalizer::RawTool>,
) {
    update_source_status(pool, name, url, "success", raws.len() as i32, None).await;
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
