//! Scheduler — tokio-cron-scheduler registration.
//!
//! Cron expressions (see MVP_DESIGN.md section 3):
//! - npm: every hour (`0 0 * * * *`)
//! - CryptoSkill: every 6h (`0 0 */6 * * *`)
//! - web3-mcp-hub: every 12h (`0 0 */12 * * *`)
//! - GitHub topics: every hour at 30min offset (`0 30 * * * *`)
//! - GitHub star sync: every 30min (`0 */30 * * * *`)
//!
//! The actual source crawl logic is added in a later milestone; this module
//! wires the scheduler so `main` can spawn it.

use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Number of cron jobs registered by the crawler scheduler.
pub const SCHEDULER_JOB_COUNT: usize = 5;

/// Start the scheduler with all cron jobs registered.
pub async fn start(pool: sqlx::PgPool) -> Result<()> {
    let scheduler = JobScheduler::new().await?;

    // Self-register OnchainAI once at scheduler startup before any scheduled
    // jobs run. This is idempotent (`ON CONFLICT (slug) DO NOTHING`).
    crate::crawler::sources::github::self_register(&pool).await;

    // npm: every hour.
    let npm_pool = pool.clone();
    let npm_job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let pool = npm_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: npm");
            crate::crawler::sources::npm::run_once(&pool).await;
        })
    })?;
    scheduler.add(npm_job).await?;

    // CryptoSkill: every 6h.
    let cs_pool = pool.clone();
    let cs_job = Job::new_async("0 0 */6 * * *", move |_uuid, _l| {
        let pool = cs_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: cryptoskill");
            crate::crawler::sources::cryptoskill::run_once(&pool).await;
        })
    })?;
    scheduler.add(cs_job).await?;

    // web3-mcp-hub: every 12h.
    let w3_pool = pool.clone();
    let w3_job = Job::new_async("0 0 */12 * * *", move |_uuid, _l| {
        let pool = w3_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: web3-mcp-hub");
            crate::crawler::sources::web3mcp::run_once(&pool).await;
        })
    })?;
    scheduler.add(w3_job).await?;

    // GitHub topics: every hour at 30min offset.
    let gh_pool = pool.clone();
    let gh_job = Job::new_async("0 30 * * * *", move |_uuid, _l| {
        let pool = gh_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: github topics");
            crate::crawler::sources::github::run_once(&pool).await;
        })
    })?;
    scheduler.add(gh_job).await?;

    // GitHub star sync: every 30min.
    let star_pool = pool.clone();
    let star_job = Job::new_async("0 */30 * * * *", move |_uuid, _l| {
        let pool = star_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled: star sync");
            crate::crawler::sources::github::sync_stars(&pool).await;
        })
    })?;
    scheduler.add(star_job).await?;

    scheduler.start().await?;
    tracing::info!(
        "crawler scheduler started with {} jobs",
        SCHEDULER_JOB_COUNT
    );

    // Keep the scheduler task alive indefinitely.
    // The scheduler runs jobs on its own runtime; we just don't return.
    std::future::pending::<()>().await;

    Ok(())
}
