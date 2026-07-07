//! Scheduler — tokio-cron-scheduler registration.
//!
//! Cron expressions (see MVP_DESIGN.md section 3):
//! - npm: every hour (`0 0 * * * *`)
//! - CryptoSkill: every 6h (`0 0 */6 * * *`)
//! - ClawHub: every 6h at :10 (`0 10 */6 * * *`)
//! - web3-mcp-hub: every 12h (`0 0 */12 * * *`)
//! - GitHub topics: every hour at 30min offset (`0 30 * * * *`)
//! - official MCP Registry: every 12h, offset 15min (`0 15 */12 * * *`)
//! - vendor orgs: daily at 03:45 UTC (`0 45 3 * * *`)
//! - Bazaar x402 discovery: every 6h at :20 (`0 20 */6 * * *`)
//! - PyPI: every 6h at :25 (`0 25 */6 * * *`)
//! - GitHub star sync: every 30min (`0 */30 * * * *`)
//!
//! The actual source crawl logic is added in a later milestone; this module
//! wires the scheduler so `main` can spawn it.

use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CrawlerJobSpec {
    pub source: &'static str,
    pub cron: &'static str,
}

const NPM_CRON: &str = "0 0 * * * *";
const CRYPTOSKILL_CRON: &str = "0 0 */6 * * *";
const CLAWHUB_CRON: &str = "0 10 */6 * * *";
const WEB3MCP_CRON: &str = "0 0 */12 * * *";
const GITHUB_CRON: &str = "0 30 * * * *";
const MCP_REGISTRY_CRON: &str = "0 15 */12 * * *";
const VENDOR_ORGS_CRON: &str = "0 45 3 * * *";
const BAZAAR_CRON: &str = "0 20 */6 * * *";
const PYPI_CRON: &str = "0 25 */6 * * *";
const STAR_SYNC_CRON: &str = "0 */30 * * * *";

pub(crate) const CRAWLER_JOB_SPECS: &[CrawlerJobSpec] = &[
    CrawlerJobSpec {
        source: "npm",
        cron: NPM_CRON,
    },
    CrawlerJobSpec {
        source: "cryptoskill",
        cron: CRYPTOSKILL_CRON,
    },
    CrawlerJobSpec {
        source: "clawhub",
        cron: CLAWHUB_CRON,
    },
    CrawlerJobSpec {
        source: "web3-mcp-hub",
        cron: WEB3MCP_CRON,
    },
    CrawlerJobSpec {
        source: "github",
        cron: GITHUB_CRON,
    },
    CrawlerJobSpec {
        source: "mcp-registry",
        cron: MCP_REGISTRY_CRON,
    },
    CrawlerJobSpec {
        source: "vendor_orgs",
        cron: VENDOR_ORGS_CRON,
    },
    CrawlerJobSpec {
        source: "bazaar",
        cron: BAZAAR_CRON,
    },
    CrawlerJobSpec {
        source: "pypi",
        cron: PYPI_CRON,
    },
    CrawlerJobSpec {
        source: "sync_stars",
        cron: STAR_SYNC_CRON,
    },
];

/// Number of cron jobs registered by the crawler scheduler.
pub const SCHEDULER_JOB_COUNT: usize = CRAWLER_JOB_SPECS.len();

/// Start the scheduler with all cron jobs registered.
pub async fn start(pool: sqlx::PgPool) -> Result<()> {
    let scheduler = JobScheduler::new().await?;

    // Self-register OnchainAI once at scheduler startup before any scheduled
    // jobs run. This is idempotent (`ON CONFLICT (slug) DO NOTHING`).
    crate::crawler::sources::github::self_register(&pool).await;

    // npm: every hour.
    let npm_pool = pool.clone();
    let npm_job = Job::new_async(NPM_CRON, move |_uuid, _l| {
        let pool = npm_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: npm");
            crate::crawler::sources::npm::run_once(&pool).await;
        })
    })?;
    scheduler.add(npm_job).await?;

    // CryptoSkill: every 6h.
    let cs_pool = pool.clone();
    let cs_job = Job::new_async(CRYPTOSKILL_CRON, move |_uuid, _l| {
        let pool = cs_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: cryptoskill");
            crate::crawler::sources::cryptoskill::run_once(&pool).await;
        })
    })?;
    scheduler.add(cs_job).await?;

    // ClawHub: every 6h at :10 UTC (offset from CryptoSkill).
    let clawhub_pool = pool.clone();
    let clawhub_job = Job::new_async(CLAWHUB_CRON, move |_uuid, _l| {
        let pool = clawhub_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: clawhub");
            crate::crawler::sources::clawhub::run_once(&pool).await;
        })
    })?;
    scheduler.add(clawhub_job).await?;

    // web3-mcp-hub: every 12h.
    let w3_pool = pool.clone();
    let w3_job = Job::new_async(WEB3MCP_CRON, move |_uuid, _l| {
        let pool = w3_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: web3-mcp-hub");
            crate::crawler::sources::web3mcp::run_once(&pool).await;
        })
    })?;
    scheduler.add(w3_job).await?;

    // GitHub topics: every hour at 30min offset.
    let gh_pool = pool.clone();
    let gh_job = Job::new_async(GITHUB_CRON, move |_uuid, _l| {
        let pool = gh_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: github topics");
            crate::crawler::sources::github::run_once(&pool).await;
        })
    })?;
    scheduler.add(gh_job).await?;

    // Official MCP Registry: every 12h, offset so it does not collide with web3-mcp-hub.
    let mcp_registry_pool = pool.clone();
    let mcp_registry_job = Job::new_async(MCP_REGISTRY_CRON, move |_uuid, _l| {
        let pool = mcp_registry_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: official MCP Registry");
            crate::crawler::sources::mcp_registry::run_once(&pool).await;
        })
    })?;
    scheduler.add(mcp_registry_job).await?;

    // Vendor org GitHub sweep: daily at 03:45 UTC.
    let vendor_orgs_pool = pool.clone();
    let vendor_orgs_job = Job::new_async(VENDOR_ORGS_CRON, move |_uuid, _l| {
        let pool = vendor_orgs_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: vendor orgs");
            crate::crawler::sources::vendor_orgs::run_once(&pool).await;
        })
    })?;
    scheduler.add(vendor_orgs_job).await?;

    // CDP Bazaar x402 discovery: every 6h at :20 UTC.
    let bazaar_pool = pool.clone();
    let bazaar_job = Job::new_async(BAZAAR_CRON, move |_uuid, _l| {
        let pool = bazaar_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: bazaar");
            crate::crawler::sources::bazaar::run_once(&pool).await;
        })
    })?;
    scheduler.add(bazaar_job).await?;

    // PyPI: every 6h at :25 UTC (offset from Bazaar).
    let pypi_pool = pool.clone();
    let pypi_job = Job::new_async(PYPI_CRON, move |_uuid, _l| {
        let pool = pypi_pool.clone();
        Box::pin(async move {
            tracing::info!("scheduled crawl: pypi");
            crate::crawler::sources::pypi::run_once(&pool).await;
        })
    })?;
    scheduler.add(pypi_job).await?;

    // GitHub star sync: every 30min.
    let star_pool = pool.clone();
    let star_job = Job::new_async(STAR_SYNC_CRON, move |_uuid, _l| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crawler_job_specs_cover_registered_scheduler_jobs() {
        assert_eq!(CRAWLER_JOB_SPECS.len(), SCHEDULER_JOB_COUNT);
        assert!(CRAWLER_JOB_SPECS
            .iter()
            .any(|spec| { spec.source == "mcp-registry" && spec.cron == MCP_REGISTRY_CRON }));
        assert!(CRAWLER_JOB_SPECS
            .iter()
            .any(|spec| { spec.source == "vendor_orgs" && spec.cron == VENDOR_ORGS_CRON }));
        assert!(CRAWLER_JOB_SPECS
            .iter()
            .any(|spec| { spec.source == "bazaar" && spec.cron == BAZAAR_CRON }));
        assert!(CRAWLER_JOB_SPECS
            .iter()
            .any(|spec| { spec.source == "pypi" && spec.cron == PYPI_CRON }));
    }
}
