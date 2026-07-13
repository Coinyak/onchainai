//! Crawler — auto-discovery orchestrator and scheduler.
// Goal harness deliverable AC1
// harness-round-7: 2026-06-25T19:10:00Z-mod
//!
//! See `docs/MVP_DESIGN.md` section 3 for the full design.

pub mod deduper;
pub mod normalizer;
pub mod pipeline;
pub mod relevance;
pub mod scheduler;
pub mod settings;
pub mod sources;
pub mod upsert;

// Re-export normalizer helpers so the public module API is easy to consume.
pub use pipeline::{run_all_sources, run_pipeline, trigger_source};
pub use pipeline::prepare_merged_crawl_tools;
pub use upsert::{
    count_raws_per_source, default_source_registry_url, gated_approval_status,
    persist_crawl_results, persist_crawl_results_gated, persist_crawl_results_gated_with_require,
    prepare_crawled_tools, prepare_crawled_tools_gated, update_source_status, upsert_tools,
    UpsertTarget,
};

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

/// Start the crawler scheduler.
///
/// Registers all cron jobs and blocks until the scheduler stops. Spawned as a
/// background task in `main`. Errors are logged, not propagated to callers
/// (the caller is a `tokio::spawn` task).
#[cfg(test)]
mod tests;
