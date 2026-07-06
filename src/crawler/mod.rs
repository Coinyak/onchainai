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

/// PostgreSQL execution target for crawler upsert/persist helpers.
pub enum UpsertTarget<'a> {
    Pool(&'a sqlx::PgPool),
    /// Open transaction connection (`&mut *tx` from [`sqlx::PgPool::begin`]).
    Connection(&'a mut sqlx::PgConnection),
}

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

/// Upsert one crawled tool row (used by [`upsert_tools`] and DB integration tests).
async fn upsert_one_tool<'e, E>(executor: E, tool: &models::Tool) -> anyhow::Result<()>
where
    E: sqlx::PgExecutor<'e>,
{
    use anyhow::Context;

    let logo_url = crate::models::tool::sanitize_logo_url(tool.logo_url.clone());
    sqlx::query(
            r#"
            INSERT INTO tools (
                name, slug, description, function, asset_class, actor, type,
                repo_url, homepage, npm_package, install_command, mcp_endpoint,
                chains, status, official_team, trust_score, approval_status,
                submitted_by, rejection_reason,
                crypto_relevance_score, crypto_relevance_reasons, relevance_status,
                install_risk_level, install_risk_reasons, requires_secret, safe_copy_command,
                review_policy_version, last_reviewed_at,
                license, pricing, x402_price, x402_endpoint,
                stars, last_commit_at, source, source_url, logo_url, logo_monogram,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
                    $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36,
                    $37, $38, $39, now())
            ON CONFLICT (slug) DO UPDATE SET
                name = CASE
                    WHEN tools.status IN ('official', 'verified')
                         OR tools.claim_state = 'claimed'
                    THEN tools.name
                    ELSE EXCLUDED.name
                END,
                description = CASE
                    WHEN tools.status IN ('official', 'verified')
                         OR tools.claim_state = 'claimed'
                    THEN COALESCE(NULLIF(tools.description, ''), EXCLUDED.description)
                    ELSE EXCLUDED.description
                END,
                function = EXCLUDED.function,
                asset_class = EXCLUDED.asset_class,
                actor = EXCLUDED.actor,
                type = EXCLUDED.type,
                repo_url = CASE
                    WHEN tools.status IN ('official', 'verified')
                         OR tools.claim_state = 'claimed'
                    THEN COALESCE(NULLIF(tools.repo_url, ''), EXCLUDED.repo_url)
                    ELSE EXCLUDED.repo_url
                END,
                homepage = CASE
                    WHEN tools.status IN ('official', 'verified')
                         OR tools.claim_state = 'claimed'
                    THEN COALESCE(NULLIF(tools.homepage, ''), EXCLUDED.homepage)
                    ELSE EXCLUDED.homepage
                END,
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
                relevance_status = CASE
                    WHEN tools.last_reviewed_at IS NOT NULL THEN tools.relevance_status
                    ELSE EXCLUDED.relevance_status
                END,
                install_risk_level = EXCLUDED.install_risk_level,
                install_risk_reasons = EXCLUDED.install_risk_reasons,
                requires_secret = EXCLUDED.requires_secret,
                safe_copy_command = EXCLUDED.safe_copy_command,
                review_policy_version = EXCLUDED.review_policy_version,
                license = EXCLUDED.license,
                pricing = CASE
                    WHEN tools.pricing IN ('x402', 'paid', 'freemium') THEN tools.pricing
                    ELSE EXCLUDED.pricing
                END,
                x402_price = COALESCE(tools.x402_price, EXCLUDED.x402_price),
                x402_endpoint = COALESCE(tools.x402_endpoint, EXCLUDED.x402_endpoint),
                stars = EXCLUDED.stars,
                last_commit_at = EXCLUDED.last_commit_at,
                source = EXCLUDED.source,
                source_url = EXCLUDED.source_url,
                logo_url = EXCLUDED.logo_url,
                logo_monogram = COALESCE(EXCLUDED.logo_monogram, tools.logo_monogram),
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
        .bind(tool.last_reviewed_at)
        .bind(&tool.license)
        .bind(&tool.pricing)
        .bind(&tool.x402_price)
        .bind(&tool.x402_endpoint)
        .bind(tool.stars)
        .bind(tool.last_commit_at)
        .bind(&tool.source)
        .bind(&tool.source_url)
        .bind(&logo_url)
        .bind(&tool.logo_monogram)
        .bind(tool.created_at)
        .execute(executor)
        .await
        .with_context(|| format!("upserting tool slug={}", tool.slug))?;
    Ok(())
}

/// Upsert a batch of crawled tools into the `tools` table.
///
/// Matching is by `slug` (unique). Existing rows keep their `status` and
/// `approval_status` when present (`official` / `verified` are preserved); all
/// other fields are overwritten with the freshly crawled values. This satisfies
/// VAL-CRAWL-014 (re-crawl preserves official/verified status).
pub async fn upsert_tools(target: UpsertTarget<'_>, tools: &[models::Tool]) -> anyhow::Result<()> {
    match target {
        UpsertTarget::Pool(pool) => {
            for tool in tools {
                upsert_one_tool(pool, tool).await?;
            }
        }
        UpsertTarget::Connection(conn) => {
            for tool in tools {
                upsert_one_tool(&mut *conn, tool).await?;
            }
        }
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
        "mcp-registry" => "https://registry.modelcontextprotocol.io/v0/servers",
        "vendor_orgs" => "https://api.github.com/orgs",
        "bazaar" => "https://api.cdp.coinbase.com/platform/v2/x402/discovery/resources",
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

/// Run the default production pipeline: all registered source crawlers,
/// normalize, dedupe, upsert to DB, and update `sources` status per source.
///
/// Errors from individual sources are logged and do not abort the run; the
/// `sources` table is updated for both successes and failures.
#[allow(dead_code)]
pub async fn run_all_sources(pool: &sqlx::PgPool) {
    let crawlers = crate::crawler::sources::default_crawlers();

    let crawler_settings = settings::load_crawler_settings(pool).await;

    let mut crawl_outcomes: Vec<SourceCrawlOutcome> = Vec::new();

    for crawler in &crawlers {
        let name = crawler.source_name().to_string();
        let url = default_source_registry_url(crawler.source_name());
        match crawler.crawl_with_pool(pool).await {
            Ok(raws) => {
                crawl_outcomes.push((name, url, Ok(raws)));
            }
            Err(e) => {
                crawl_outcomes.push((name, url, Err(e.to_string())));
            }
        }
    }

    let tools = prepare_merged_crawl_tools(&crawl_outcomes, crawler_settings.require_tool_approval);
    let all_raws: Vec<normalizer::RawTool> = crawl_outcomes
        .iter()
        .filter_map(|(_, _, outcome)| outcome.as_ref().ok())
        .flatten()
        .cloned()
        .collect();
    let raw_counts = count_raws_per_source(&all_raws);
    tracing::debug!(?raw_counts, "raw tool counts by source field");
    let upsert_err = upsert_tools(UpsertTarget::Pool(pool), &tools)
        .await
        .err()
        .map(|e| e.to_string());

    for (name, url, outcome) in crawl_outcomes {
        match outcome {
            Ok(raws) => {
                let count = raws.len() as i32;
                if let Some(ref err) = upsert_err {
                    update_source_status(
                        UpsertTarget::Pool(pool),
                        &name,
                        url,
                        "error",
                        0,
                        Some(err),
                    )
                    .await;
                } else {
                    update_source_status(
                        UpsertTarget::Pool(pool),
                        &name,
                        url,
                        "success",
                        count,
                        None,
                    )
                    .await;
                }
            }
            Err(msg) => {
                update_source_status(UpsertTarget::Pool(pool), &name, url, "error", 0, Some(&msg))
                    .await;
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
    target: UpsertTarget<'_>,
    name: &str,
    url: &str,
    status: &str,
    items_found: i32,
    error_message: Option<&str>,
) {
    let result = match target {
        UpsertTarget::Pool(pool) => {
            update_source_status_one(pool, name, url, status, items_found, error_message).await
        }
        UpsertTarget::Connection(conn) => {
            update_source_status_one(&mut *conn, name, url, status, items_found, error_message)
                .await
        }
    };
    if let Err(e) = result {
        tracing::error!(source = name, error = %e, "failed to update sources table");
    }
}

async fn update_source_status_one<'e, E>(
    executor: E,
    name: &str,
    url: &str,
    status: &str,
    items_found: i32,
    error_message: Option<&str>,
) -> sqlx::Result<sqlx::postgres::PgQueryResult>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query(
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
    .execute(executor)
    .await
}

/// Resolve initial `approval_status` for a crawl source.
///
/// `vendor_orgs` and `bazaar` always return `"pending"`, ignoring
/// `require_tool_approval` (§4.2).
pub(crate) fn gated_approval_status(
    source_name: &str,
    require_tool_approval: bool,
) -> &'static str {
    if source_name == "vendor_orgs" || source_name == "bazaar" {
        "pending"
    } else {
        settings::initial_approval_status(require_tool_approval)
    }
}

/// Merge per-source crawl outcomes with per-source approval gating (§4.2).
///
/// `vendor_orgs` and `bazaar` always get `pending`; other sources follow
/// `require_tool_approval`. Global `repo_url` dedupe runs after merge.
pub(crate) fn prepare_merged_crawl_tools(
    outcomes: &[SourceCrawlOutcome],
    require_tool_approval: bool,
) -> Vec<models::Tool> {
    let mut all_tools = Vec::new();
    for (name, _, outcome) in outcomes {
        if let Ok(raws) = outcome {
            let approval = gated_approval_status(name, require_tool_approval);
            all_tools.extend(normalizer::normalize_batch_with_status(raws, approval));
        }
    }
    deduper::dedupe(all_tools)
}

/// DB-free pipeline: normalize raw crawls with gated approval, then dedupe.
pub(crate) fn prepare_crawled_tools_gated(
    raws: &[normalizer::RawTool],
    source_name: &str,
    require_tool_approval: bool,
) -> Vec<models::Tool> {
    let approval = gated_approval_status(source_name, require_tool_approval);
    let tools = normalizer::normalize_batch_with_status(raws, approval);
    deduper::dedupe(tools)
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
    match upsert_tools(UpsertTarget::Pool(pool), &tools).await {
        Ok(()) => {
            tracing::info!(source = name, count, "crawled tools upserted");
            update_source_status(UpsertTarget::Pool(pool), name, url, "success", count, None).await;
        }
        Err(e) => {
            tracing::error!(source = name, error = %e, "failed to upsert crawled tools");
            update_source_status(
                UpsertTarget::Pool(pool),
                name,
                url,
                "error",
                0,
                Some(&e.to_string()),
            )
            .await;
        }
    }
}

/// Persist gated crawl results with an explicit `require_tool_approval` flag.
///
/// Used by integration tests to prove vendor_orgs/bazaar force `pending` without
/// mutating `site_settings`. Production callers use [`persist_crawl_results_gated`].
pub async fn persist_crawl_results_gated_with_require(
    target: UpsertTarget<'_>,
    name: &str,
    url: &str,
    raws: Vec<normalizer::RawTool>,
    require_tool_approval: bool,
) {
    let tools = prepare_crawled_tools_gated(&raws, name, require_tool_approval);
    let count = tools.len() as i32;

    match target {
        UpsertTarget::Pool(pool) => match upsert_tools(UpsertTarget::Pool(pool), &tools).await {
            Ok(()) => {
                tracing::info!(source = name, count, "crawled tools upserted (gated)");
                update_source_status(UpsertTarget::Pool(pool), name, url, "success", count, None)
                    .await;
            }
            Err(e) => {
                tracing::error!(source = name, error = %e, "failed to upsert gated crawled tools");
                update_source_status(
                    UpsertTarget::Pool(pool),
                    name,
                    url,
                    "error",
                    0,
                    Some(&e.to_string()),
                )
                .await;
            }
        },
        UpsertTarget::Connection(conn) => {
            match upsert_tools(UpsertTarget::Connection(conn), &tools).await {
                Ok(()) => {
                    tracing::info!(source = name, count, "crawled tools upserted (gated)");
                    update_source_status(
                        UpsertTarget::Connection(conn),
                        name,
                        url,
                        "success",
                        count,
                        None,
                    )
                    .await;
                }
                Err(e) => {
                    tracing::error!(source = name, error = %e, "failed to upsert gated crawled tools");
                    update_source_status(
                        UpsertTarget::Connection(conn),
                        name,
                        url,
                        "error",
                        0,
                        Some(&e.to_string()),
                    )
                    .await;
                }
            }
        }
    }
}

/// Persist crawl results with per-source approval gating (§4.2).
///
/// For `vendor_orgs` and `bazaar`, newly normalized tools always get
/// `approval_status = "pending"`, regardless of `site_settings.require_tool_approval`.
pub async fn persist_crawl_results_gated(
    pool: &sqlx::PgPool,
    name: &str,
    url: &str,
    raws: Vec<normalizer::RawTool>,
) {
    let crawler_settings = settings::load_crawler_settings(pool).await;
    persist_crawl_results_gated_with_require(
        UpsertTarget::Pool(pool),
        name,
        url,
        raws,
        crawler_settings.require_tool_approval,
    )
    .await;
}

/// Manually trigger a single crawler job (admin UI / server function).
///
/// Spawns work synchronously in the caller's task; long-running crawls should be
/// invoked from a background `tokio::spawn` at the call site.
pub async fn trigger_source(pool: &sqlx::PgPool, source: &str) {
    use crate::crawler::sources::{
        bazaar, cryptoskill, github, mcp_registry, npm, vendor_orgs, web3mcp,
    };

    match source {
        "npm" => npm::run_once(pool).await,
        "cryptoskill" => cryptoskill::run_once(pool).await,
        "web3-mcp-hub" => web3mcp::run_once(pool).await,
        "github" => github::run_once(pool).await,
        "mcp-registry" => mcp_registry::run_once(pool).await,
        "vendor_orgs" => vendor_orgs::run_once(pool).await,
        "bazaar" => bazaar::run_once(pool).await,
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
            stars,
            source: "stub".into(),
            ..Default::default()
        }
    }

    #[test]
    fn gated_approval_status_forces_pending_for_vendor_orgs_and_bazaar() {
        assert_eq!(gated_approval_status("vendor_orgs", false), "pending");
        assert_eq!(gated_approval_status("bazaar", false), "pending");
        assert_eq!(gated_approval_status("vendor_orgs", true), "pending");
        assert_eq!(gated_approval_status("npm", false), "approved");
        assert_eq!(gated_approval_status("npm", true), "pending");
    }

    #[test]
    fn prepare_merged_crawl_tools_forces_pending_for_bazaar_when_auto_publish() {
        let bazaar_raw = raw("Bazaar Item", None, 1, "x402 payment api");
        let mut bazaar_raw = bazaar_raw;
        bazaar_raw.source = "bazaar".into();
        bazaar_raw.tool_type = "x402".into();
        bazaar_raw.pricing = "x402".into();

        let npm_raw = raw(
            "Npm Tool",
            Some("https://github.com/acme/pkg"),
            5,
            "swap dex",
        );
        let mut npm_raw = npm_raw;
        npm_raw.source = "npm".into();

        let outcomes = vec![
            (
                "bazaar".to_string(),
                default_source_registry_url("bazaar"),
                Ok(vec![bazaar_raw]),
            ),
            (
                "npm".to_string(),
                default_source_registry_url("npm"),
                Ok(vec![npm_raw]),
            ),
        ];
        let tools = prepare_merged_crawl_tools(&outcomes, false);
        assert_eq!(tools.len(), 2);
        let bazaar_tool = tools
            .iter()
            .find(|t| t.source == "bazaar")
            .expect("bazaar tool");
        let npm_tool = tools.iter().find(|t| t.source == "npm").expect("npm tool");
        assert_eq!(bazaar_tool.approval_status, "pending");
        assert_eq!(npm_tool.approval_status, "approved");
    }

    #[test]
    fn prepare_crawled_tools_gated_forces_pending_for_bazaar_when_auto_publish() {
        let tools = prepare_crawled_tools_gated(
            &[raw("Bazaar Item", None, 1, "x402 payment api")],
            "bazaar",
            false,
        );
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].approval_status, "pending");
    }

    async fn test_pool() -> Option<sqlx::PgPool> {
        connect_test_pool(false).await
    }

    fn db_test_env_configured() -> bool {
        let _ = dotenvy::dotenv();
        ["SUPABASE_URL_TEST", "DATABASE_URL", "DATABASE_URL_TEST"]
            .iter()
            .any(|key| {
                std::env::var(key)
                    .ok()
                    .is_some_and(|url| !url.trim().is_empty())
            })
    }

    /// Skip when no DB env vars are set; fail when vars are set but connection fails.
    async fn test_pool_required_if_configured() -> Option<sqlx::PgPool> {
        if !db_test_env_configured() {
            eprintln!(
                "SKIP: SUPABASE_URL_TEST, DATABASE_URL, or DATABASE_URL_TEST must be set for crawler DB integration test"
            );
            return None;
        }
        connect_test_pool(true).await
    }

    async fn connect_test_pool(required: bool) -> Option<sqlx::PgPool> {
        let _ = dotenvy::dotenv();
        let mut candidates = Vec::new();
        for key in ["SUPABASE_URL_TEST", "DATABASE_URL", "DATABASE_URL_TEST"] {
            if let Ok(url) = std::env::var(key) {
                let url = url.trim().to_string();
                if !url.is_empty() && !candidates.iter().any(|(k, _)| k == &key) {
                    candidates.push((key, url));
                }
            }
        }

        if candidates.is_empty() {
            let msg =
                "SUPABASE_URL_TEST, DATABASE_URL, or DATABASE_URL_TEST must be set for crawler DB integration test";
            if required {
                panic!("{msg}");
            }
            eprintln!("SKIP: {msg}");
            return None;
        }

        let mut last_err = None;
        for (key, database_url) in candidates {
            match sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(std::time::Duration::from_secs(30))
                .connect(&database_url)
                .await
            {
                Ok(pool) => {
                    eprintln!("crawler DB integration test connected via {key}");
                    return Some(pool);
                }
                Err(e) => {
                    eprintln!("crawler DB integration test: {key} connect failed ({e})");
                    last_err = Some((key, e));
                }
            }
        }

        if required {
            let (key, err) = last_err.expect("candidate urls were empty");
            panic!("all database URLs failed — last attempt {key}: {err}");
        }
        eprintln!("SKIP: database connection failed — crawler DB integration test");
        None
    }

    #[tokio::test]
    async fn upsert_x402_clobber_guard_preserves_self_listing_metadata() {
        let Some(pool) = test_pool().await else {
            return;
        };

        let mut tx = pool
            .begin()
            .await
            .expect("begin upsert_x402_clobber_guard test tx");

        let slug = format!("x402-clobber-{}", uuid::Uuid::new_v4());
        let reviewed_at = chrono::Utc::now();

        let mut seed_raw = raw(
            "Self Listed",
            Some("https://github.com/self/listed"),
            0,
            "x402 usdc payment checkout",
        );
        seed_raw.pricing = "x402".into();
        seed_raw.x402_price = Some("$0.05".into());
        seed_raw.x402_endpoint = Some("https://api.listed.example/x402/resource".into());
        seed_raw.tool_type = "x402".into();
        let mut seed_tool =
            normalizer::normalize(&seed_raw, &std::collections::HashSet::new(), "approved");
        seed_tool.slug = slug.clone();
        seed_tool.pricing = "x402".into();
        seed_tool.x402_price = Some("$0.05".into());
        seed_tool.x402_endpoint = Some("https://api.listed.example/x402/resource".into());
        seed_tool.relevance_status = "accepted".into();
        seed_tool.last_reviewed_at = Some(reviewed_at);

        eprintln!("upsert_x402_clobber_guard: begin tx, seed slug={slug}");
        upsert_tools(
            UpsertTarget::Connection(&mut tx),
            std::slice::from_ref(&seed_tool),
        )
        .await
        .expect("seed self-listed row via upsert_tools INSERT path");

        let mut crawl_raw = raw("Crawl Overwrite", None, 1, "free dev tool");
        crawl_raw.pricing = "free".into();
        crawl_raw.x402_price = Some("$9.99".into());
        crawl_raw.x402_endpoint = Some("https://evil.example/x402".into());
        let mut crawl_tool =
            normalizer::normalize(&crawl_raw, &std::collections::HashSet::new(), "approved");
        crawl_tool.slug = slug.clone();
        crawl_tool.pricing = "free".into();
        crawl_tool.x402_price = Some("$9.99".into());
        crawl_tool.x402_endpoint = Some("https://evil.example/x402".into());
        crawl_tool.relevance_status = "rejected".into();

        upsert_tools(
            UpsertTarget::Connection(&mut tx),
            std::slice::from_ref(&crawl_tool),
        )
        .await
        .expect("upsert conflicting crawl row on slug conflict");
        eprintln!("upsert_x402_clobber_guard: conflict upsert done, selecting row");

        let row: (String, Option<String>, Option<String>, String) = sqlx::query_as(
            r#"
            SELECT pricing, x402_price, x402_endpoint, relevance_status
            FROM tools WHERE slug = $1
            "#,
        )
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await
        .expect("load post-upsert row");

        assert_eq!(row.0, "x402");
        assert_eq!(row.1.as_deref(), Some("$0.05"));
        assert_eq!(
            row.2.as_deref(),
            Some("https://api.listed.example/x402/resource")
        );
        assert_eq!(row.3, "accepted");
        eprintln!(
            "upsert_x402_clobber_guard: preserved pricing={} x402_price={:?} relevance={}",
            row.0, row.1, row.3
        );

        tx.rollback()
            .await
            .expect("rollback upsert_x402_clobber_guard test");
        eprintln!("upsert_x402_clobber_guard: rollback ok");
    }

    #[tokio::test]
    async fn vendor_orgs_slug_rename_policy() {
        use crate::crawler::sources::vendor_orgs::{effective_tool_name, should_rename_repo_slug};

        assert!(should_rename_repo_slug("skills"));
        assert_eq!(
            effective_tool_name("circlefin", "skills"),
            "circlefin-skills"
        );

        let Some(pool) = test_pool_required_if_configured().await else {
            return;
        };

        let mut tx = pool
            .begin()
            .await
            .expect("begin vendor_orgs_slug_rename_policy test tx");

        let slug = format!("circlefin-skills-{}", uuid::Uuid::new_v4());
        let trusted_repo = "https://github.com/circlefin/skills";
        let trusted_homepage = "https://agents.circle.com/skills";

        let mut seed_tool = normalizer::normalize(
            &RawTool {
                name: "circlefin-skills".into(),
                description: Some("Official Circle skills".into()),
                tool_type: "sdk".into(),
                repo_url: Some(trusted_repo.into()),
                homepage: Some(trusted_homepage.into()),
                source: "manual".into(),
                ..Default::default()
            },
            &std::collections::HashSet::new(),
            "approved",
        );
        seed_tool.slug = slug.clone();
        seed_tool.status = "official".into();
        seed_tool.official_team = Some("Circle".into());

        upsert_tools(
            UpsertTarget::Connection(&mut tx),
            std::slice::from_ref(&seed_tool),
        )
        .await
        .expect("seed trusted official row");

        let mut crawl_tool = normalizer::normalize(
            &RawTool {
                name: "circlefin-skills".into(),
                description: Some("Crawler overwrite attempt".into()),
                tool_type: "mcp".into(),
                repo_url: Some("https://github.com/circlefin/skills-fork".into()),
                homepage: Some("https://evil.example/skills".into()),
                source: "vendor_orgs".into(),
                ..Default::default()
            },
            &std::collections::HashSet::new(),
            "pending",
        );
        crawl_tool.slug = slug.clone();

        upsert_tools(
            UpsertTarget::Connection(&mut tx),
            std::slice::from_ref(&crawl_tool),
        )
        .await
        .expect("upsert vendor_orgs crawl row on slug conflict");

        let row: (String, Option<String>, Option<String>, String) = sqlx::query_as(
            r#"
            SELECT name, repo_url, homepage, status
            FROM tools WHERE slug = $1
            "#,
        )
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await
        .expect("load post-upsert trusted row");

        assert_eq!(row.0, "circlefin-skills");
        assert_eq!(row.1.as_deref(), Some(trusted_repo));
        assert_eq!(row.2.as_deref(), Some(trusted_homepage));
        assert_eq!(row.3, "official");
        eprintln!(
            "vendor_orgs_slug_rename_policy: trusted-row guard preserved name={} repo_url={:?} homepage={:?} status={}",
            row.0, row.1, row.2, row.3
        );

        tx.rollback()
            .await
            .expect("rollback vendor_orgs_slug_rename_policy test");
        eprintln!("vendor_orgs_slug_rename_policy: rollback ok");
    }

    #[tokio::test]
    async fn persist_crawl_results_gated_respects_force_pending() {
        let Some(pool) = test_pool().await else {
            return;
        };

        let mut tx = pool
            .begin()
            .await
            .expect("begin persist_crawl_results_gated test tx");

        let suffix = uuid::Uuid::new_v4();
        let test_url = format!("https://test.example/vendor-orgs-{suffix}");
        let raws = vec![RawTool {
            name: format!("vendor-org-gated-{suffix}"),
            description: Some("x402 vendor org repo".into()),
            tool_type: "mcp".into(),
            source: "vendor_orgs".into(),
            ..Default::default()
        }];
        let expected_slug = prepare_crawled_tools_gated(&raws, "vendor_orgs", false)[0]
            .slug
            .clone();

        eprintln!(
            "persist_crawl_results_gated: begin tx, require_tool_approval=false injected, slug={expected_slug}"
        );
        persist_crawl_results_gated_with_require(
            UpsertTarget::Connection(&mut tx),
            "vendor_orgs",
            &test_url,
            raws,
            false,
        )
        .await;

        let approval_status: String =
            sqlx::query_scalar("SELECT approval_status FROM tools WHERE slug = $1")
                .bind(&expected_slug)
                .fetch_one(&mut *tx)
                .await
                .expect("gated upsert row exists");

        assert_eq!(approval_status, "pending");
        eprintln!("persist_crawl_results_gated: approval_status={approval_status}");

        tx.rollback()
            .await
            .expect("rollback persist_crawl_results_gated test");
        eprintln!("persist_crawl_results_gated: rollback ok");
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
        assert_eq!(
            default_source_registry_url("mcp-registry"),
            "https://registry.modelcontextprotocol.io/v0/servers"
        );
        assert_eq!(
            default_source_registry_url("vendor_orgs"),
            "https://api.github.com/orgs"
        );
        assert_eq!(
            default_source_registry_url("bazaar"),
            "https://api.cdp.coinbase.com/platform/v2/x402/discovery/resources"
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
