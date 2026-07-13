//! Crawler pipeline orchestration (parallel sources + full runs).

use std::sync::Arc;

use super::deduper;
use super::models;
use super::normalizer;
use super::settings;
use super::sources::SourceCrawler;
use super::upsert::{
    count_raws_per_source, default_source_registry_url, persist_crawl_results, update_source_status,
    upsert_tools, UpsertTarget,
};

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

/// Per-source crawl result before merge/dedupe.
type SourceCrawlOutcome = (
    String,
    &'static str,
    Result<Vec<normalizer::RawTool>, String>,
);

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


pub async fn trigger_source(pool: &sqlx::PgPool, source: &str) {
    use crate::crawler::sources::{
        bazaar, clawhub, cryptoskill, github, mcp_registry, npm, pypi, vendor_orgs, web3mcp,
    };

    match source {
        "npm" => npm::run_once(pool).await,
        "clawhub" => clawhub::run_once(pool).await,
        "cryptoskill" => cryptoskill::run_once(pool).await,
        "web3-mcp-hub" => web3mcp::run_once(pool).await,
        "github" => github::run_once(pool).await,
        "mcp-registry" => mcp_registry::run_once(pool).await,
        "vendor_orgs" => vendor_orgs::run_once(pool).await,
        "bazaar" => bazaar::run_once(pool).await,
        "pypi" => pypi::run_once(pool).await,
        "sync_stars" => github::sync_stars(pool).await,
        other => tracing::warn!(source = other, "unknown crawler source trigger"),
    }
}

/// Resolve initial `approval_status` for a crawl source.
///
/// `vendor_orgs` and `bazaar` always return `"pending"`, ignoring
/// `require_tool_approval` (§4.2).
pub fn gated_approval_status(
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
pub fn prepare_merged_crawl_tools(
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
