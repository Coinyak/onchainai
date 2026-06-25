//! Crawler — auto-discovery orchestrator and scheduler.
//!
//! See `docs/MVP_DESIGN.md` section 3 for the full design. The MVP foundation
//! only wires the scheduler spawn point; source implementations are added in
//! later milestones.

pub mod deduper;
pub mod normalizer;
pub mod scheduler;
pub mod sources;

/// Start the crawler scheduler.
///
/// Registers all cron jobs and blocks until the scheduler stops. Spawned as a
/// background task in `main`. Errors are logged, not propagated to callers
/// (the caller is a `tokio::spawn` task).
pub async fn start_scheduler(pool: sqlx::PgPool) -> anyhow::Result<()> {
    scheduler::start(pool).await
}
