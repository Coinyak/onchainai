//! OnchainAI — crypto tool directory.
//!
//! Single Rust binary: Leptos SSR + Axum + rmcp + sqlx + tokio-cron-scheduler.

mod config;
mod crawler;
#[allow(dead_code)]
mod models;
mod server;

pub use config::Config;

/// Build the Axum application router.
///
/// Holds a [`PgPool`] in app state for server functions and the MCP handler.
/// The crawler scheduler is spawned separately in [`main`].
fn build_app(pool: sqlx::PgPool) -> axum::Router {
    axum::Router::new()
        .route("/", axum::routing::get(|| async { "OnchainAI" }))
        .with_state(pool)
}

/// Apply embedded SQL migrations.
async fn run_migrations(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| anyhow::anyhow!("migration failed: {e}"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env (harmless if missing).
    let _ = dotenvy::dotenv();

    // Initialize tracing subscriber with env filter (RUST_LOG, default info).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("OnchainAI starting up");

    // Load configuration from environment.
    let cfg = Config::from_env()?;
    tracing::info!(
        "config loaded (port={}, siwx_domain={})",
        cfg.port,
        cfg.siwx_domain
    );

    // Initialize DB pool + run migrations.
    let pool = config::setup_db(&cfg.database_url).await?;
    tracing::info!("database pool initialized");
    run_migrations(&pool).await?;
    tracing::info!("migrations applied");

    // Crawler scheduler — background task.
    let crawler_pool = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = crawler::start_scheduler(crawler_pool).await {
            tracing::error!("crawler scheduler exited with error: {e}");
        }
    });
    tracing::info!("crawler scheduler spawned in background (tokio::spawn)");

    // Axum server (website + MCP endpoint on the same port).
    let app = build_app(pool);
    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!("binding Axum server on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
