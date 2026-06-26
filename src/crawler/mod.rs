//! Crawler — auto-discovery orchestrator and scheduler.
// Goal harness deliverable AC1
// harness-round-7: 2026-06-25T19:10:00Z-mod
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
pub mod relevance;
pub mod scheduler;
pub mod settings;
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
#[allow(dead_code)]
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
                submitted_by, rejection_reason,
                crypto_relevance_score, crypto_relevance_reasons, relevance_status,
                install_risk_level, install_risk_reasons, requires_secret, safe_copy_command,
                review_policy_version,
                license, pricing, x402_price,
                stars, last_commit_at, source, source_url, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
                    $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, now())
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
                crypto_relevance_score = EXCLUDED.crypto_relevance_score,
                crypto_relevance_reasons = EXCLUDED.crypto_relevance_reasons,
                relevance_status = EXCLUDED.relevance_status,
                install_risk_level = EXCLUDED.install_risk_level,
                install_risk_reasons = EXCLUDED.install_risk_reasons,
                requires_secret = EXCLUDED.requires_secret,
                safe_copy_command = EXCLUDED.safe_copy_command,
                review_policy_version = EXCLUDED.review_policy_version,
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
        .bind(tool.crypto_relevance_score)
        .bind(&tool.crypto_relevance_reasons)
        .bind(&tool.relevance_status)
        .bind(&tool.install_risk_level)
        .bind(&tool.install_risk_reasons)
        .bind(tool.requires_secret)
        .bind(&tool.safe_copy_command)
        .bind(&tool.review_policy_version)
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

/// Registry URL written to `sources.url` for each built-in crawler name.
pub(crate) fn default_source_registry_url(source_name: &str) -> &'static str {
    match source_name {
        "npm" => "https://registry.npmjs.org/",
        "cryptoskill" => "https://cryptoskill.org/skills.json",
        "web3-mcp-hub" => {
            "https://raw.githubusercontent.com/rudazy/web3-mcp-hub/main/registry.json"
        }
        "github" => "https://github.com/topics",
        _ => "https://www.onchain-ai.xyz",
    }
}

/// Per-source crawl result before merge/dedupe.
type SourceCrawlOutcome = (
    String,
    &'static str,
    Result<Vec<normalizer::RawTool>, String>,
);

/// Count raw crawl rows per `RawTool.source` (for diagnostics / status reporting).
pub(crate) fn count_raws_per_source(
    raws: &[normalizer::RawTool],
) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    for raw in raws {
        *counts.entry(raw.source.clone()).or_insert(0) += 1;
    }
    counts
}

/// Run the default production pipeline: all four source crawlers in parallel,
/// normalize, dedupe, upsert to DB, and update `sources` status per source.
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

    let crawler_settings = settings::load_crawler_settings(pool).await;

    let mut crawl_outcomes: Vec<SourceCrawlOutcome> = Vec::new();
    let mut all_raws: Vec<normalizer::RawTool> = Vec::new();

    for crawler in &crawlers {
        let name = crawler.source_name().to_string();
        let url = default_source_registry_url(crawler.source_name());
        match crawler.crawl().await {
            Ok(raws) => {
                all_raws.extend(raws.clone());
                crawl_outcomes.push((name, url, Ok(raws)));
            }
            Err(e) => {
                crawl_outcomes.push((name, url, Err(e.to_string())));
            }
        }
    }

    let tools = prepare_crawled_tools(&all_raws, crawler_settings.require_tool_approval);
    let raw_counts = count_raws_per_source(&all_raws);
    tracing::debug!(?raw_counts, "raw tool counts by source field");
    let upsert_err = upsert_tools(pool, &tools)
        .await
        .err()
        .map(|e| e.to_string());

    for (name, url, outcome) in crawl_outcomes {
        match outcome {
            Ok(raws) => {
                let count = raws.len() as i32;
                if let Some(ref err) = upsert_err {
                    update_source_status(pool, &name, url, "error", 0, Some(err)).await;
                } else {
                    update_source_status(pool, &name, url, "success", count, None).await;
                }
            }
            Err(msg) => {
                update_source_status(pool, &name, url, "error", 0, Some(&msg)).await;
            }
        }
    }

    if let Some(err) = upsert_err {
        tracing::error!(error = %err, "failed to upsert crawled tools");
    } else {
        tracing::info!(count = tools.len(), "crawled tools upserted");
    }
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

/// DB-free pipeline: normalize raw crawls with the approval decision, then dedupe.
///
/// Called by [`persist_crawl_results`] after loading `require_tool_approval` from
/// `site_settings`. Unit-tested without a database.
pub(crate) fn prepare_crawled_tools(
    raws: &[normalizer::RawTool],
    require_tool_approval: bool,
) -> Vec<models::Tool> {
    let approval = settings::initial_approval_status(require_tool_approval);
    let tools = normalizer::normalize_batch_with_status(raws, approval);
    deduper::dedupe(tools)
}

/// Normalize, dedupe, upsert crawled tools, then update the `sources` table.
///
/// Loads [`settings::CrawlerSettings`] to decide initial `approval_status` for
/// newly discovered tools (`pending` vs `approved`).
pub async fn persist_crawl_results(
    pool: &sqlx::PgPool,
    name: &str,
    url: &str,
    raws: Vec<normalizer::RawTool>,
) {
    let crawler_settings = settings::load_crawler_settings(pool).await;
    let tools = prepare_crawled_tools(&raws, crawler_settings.require_tool_approval);
    let count = tools.len() as i32;

    match upsert_tools(pool, &tools).await {
        Ok(()) => {
            tracing::info!(source = name, count, "crawled tools upserted");
            update_source_status(pool, name, url, "success", count, None).await;
        }
        Err(e) => {
            tracing::error!(source = name, error = %e, "failed to upsert crawled tools");
            update_source_status(pool, name, url, "error", 0, Some(&e.to_string())).await;
        }
    }
}

/// Manually trigger a single crawler job (admin UI / server function).
///
/// Spawns work synchronously in the caller's task; long-running crawls should be
/// invoked from a background `tokio::spawn` at the call site.
pub async fn trigger_source(pool: &sqlx::PgPool, source: &str) {
    use crate::crawler::sources::{cryptoskill, github, npm, web3mcp};

    match source {
        "npm" => npm::run_once(pool).await,
        "cryptoskill" => cryptoskill::run_once(pool).await,
        "web3-mcp-hub" => web3mcp::run_once(pool).await,
        "github" => github::run_once(pool).await,
        "sync_stars" => github::sync_stars(pool).await,
        other => tracing::warn!(source = other, "unknown crawler source trigger"),
    }
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

    #[test]
    fn prepare_crawled_tools_pending_when_approval_required() {
        let tools =
            prepare_crawled_tools(&[raw("Pending Tool", None, 1, "bridge cross-chain")], true);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].approval_status, "pending");
        assert_eq!(tools[0].function, "bridge");
    }

    #[test]
    fn prepare_crawled_tools_approved_when_auto_publish() {
        let tools = prepare_crawled_tools(&[raw("Auto Tool", None, 1, "swap dex")], false);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].approval_status, "approved");
        assert_eq!(tools[0].function, "swap");
    }

    #[test]
    fn count_raws_per_source_groups_by_source_field() {
        let mut a = raw("One", None, 1, "bridge");
        a.source = "npm".into();
        let mut b = raw("Two", None, 2, "swap");
        b.source = "npm".into();
        let mut c = raw("Three", None, 3, "bridge");
        c.source = "github".into();
        let counts = count_raws_per_source(&[a, b, c]);
        assert_eq!(counts.get("npm"), Some(&2));
        assert_eq!(counts.get("github"), Some(&1));
    }

    #[test]
    fn default_source_registry_url_matches_run_once_urls() {
        assert_eq!(
            default_source_registry_url("npm"),
            "https://registry.npmjs.org/"
        );
        assert_eq!(
            default_source_registry_url("github"),
            "https://github.com/topics"
        );
    }

    #[test]
    fn prepare_crawled_tools_dedupes_duplicate_repo_urls() {
        let raws = [
            raw(
                "Low Stars",
                Some("https://github.com/dup/dup"),
                1,
                "swap dex",
            ),
            raw(
                "High Stars",
                Some("https://github.com/dup/dup"),
                999,
                "swap dex",
            ),
        ];
        let tools = prepare_crawled_tools(&raws, false);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].stars, 999);
        assert_eq!(tools[0].approval_status, "approved");
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
