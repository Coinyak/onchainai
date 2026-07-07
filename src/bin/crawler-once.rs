//! One-shot crawler trigger for operator use (prod DB via DATABASE_URL).
//!
//! Usage: `PG_INSECURE_SSL=1 cargo run --features ssr --bin crawler-once -- vendor_orgs`

use onchainai::{config, crawler};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let source = std::env::args()
        .nth(1)
        .unwrap_or_else(|| {
            eprintln!("usage: crawler-once <source>");
            eprintln!("sources: npm, clawhub, cryptoskill, web3-mcp-hub, github, mcp-registry, vendor_orgs, bazaar, pypi, sync_stars");
            std::process::exit(1);
        });

    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;
    let pool = config::setup_db(&database_url).await?;
    tracing::info!(source = %source, "crawler-once: starting");
    crawler::trigger_source(&pool, &source).await;
    tracing::info!(source = %source, "crawler-once: finished");
    Ok(())
}
