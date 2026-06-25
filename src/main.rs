//! OnchainAI — crypto tool directory.
//!
//! Single Rust binary: Leptos SSR + Axum + rmcp + sqlx + tokio-cron-scheduler.

mod app;
mod auth;
mod components;
mod config;
mod crawler;
#[allow(dead_code)]
mod models;
mod pages;
mod server;

use axum::Router;
use leptos::config::get_configuration;
use leptos_axum::{file_and_error_handler_with_context, generate_route_list, LeptosRoutes};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

pub use config::Config;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Postgres connection pool.
    pub pool: sqlx::PgPool,
    /// Environment configuration (auth, ports, secrets).
    pub config: Config,
    /// Leptos SSR options.
    pub leptos_options: leptos::config::LeptosOptions,
}

impl axum::extract::FromRef<AppState> for leptos::config::LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos_options.clone()
    }
}

/// Build the Axum application router.
///
/// Wires Leptos SSR routes, static file serving, MCP endpoint, security
/// headers, CORS, rate limiting, and compression. The router is returned
/// as a `Router` so it can be served with `axum::serve`.
fn build_app(pool: sqlx::PgPool, config: Config) -> axum::Router {
    let conf = get_configuration(None).expect("leptos configuration");
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(app::App);

    let state = AppState {
        pool,
        config,
        leptos_options: leptos_options.clone(),
    };

    let leptos_options_for_handler = leptos_options.clone();
    let state_for_context = state.clone();
    let state_for_fallback = state.clone();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _request_head| {
            let allowed = ["https://onchainai.xyz", "http://localhost:3000"];
            if let Ok(origin_str) = origin.to_str() {
                allowed.contains(&origin_str)
            } else {
                false
            }
        }))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true);

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(30)
        .finish()
        .expect("governor config");
    let rate_limit = GovernorLayer::new(governor_conf);

    let security_headers = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::X_FRAME_OPTIONS,
            axum::http::HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            axum::http::HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("strict-transport-security"),
            axum::http::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::REFERRER_POLICY,
            axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("permissions-policy"),
            axum::http::HeaderValue::from_static(
                "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()",
            ),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::CONTENT_SECURITY_POLICY,
            axum::http::HeaderValue::from_static(
                "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';",
            ),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("x-xss-protection"),
            axum::http::HeaderValue::from_static("0"),
        ));

    let static_route = leptos_axum::site_pkg_dir_service_route_path(&leptos_options);

    let provide_leptos_context = {
        let state_for_context = state_for_context.clone();
        move || {
            provide_context(state_for_context.pool.clone());
            provide_context(state_for_context.config.clone());
        }
    };
    let provide_fallback_context = {
        let state_for_fallback = state_for_fallback.clone();
        move || {
            provide_context(state_for_fallback.pool.clone());
            provide_context(state_for_fallback.config.clone());
        }
    };

    Router::new()
        // Static assets generated by the build (CSS, JS, WASM).
        .route_service(
            &static_route,
            leptos_axum::site_pkg_dir_service(&leptos_options),
        )
        // MCP JSON-RPC endpoint (4 tools, approved-only data).
        .route("/mcp", axum::routing::post(server::mcp::handle_mcp))
        // Supabase OAuth (GitHub) + logout.
        .route("/auth/github", axum::routing::get(auth::routes::github_login))
        .route("/auth/email", axum::routing::post(auth::email::send_magic_link))
        .route("/auth/callback", axum::routing::get(auth::routes::oauth_callback))
        .route("/auth/logout", axum::routing::post(auth::routes::logout))
        .route(
            "/auth/siwx/challenge",
            axum::routing::post(auth::siwx::challenge),
        )
        .route(
            "/auth/siwx/verify",
            axum::routing::post(auth::siwx::verify),
        )
        // Leptos SSR routes.
        .leptos_routes_with_context(
            &state,
            routes,
            provide_leptos_context,
            move || app::shell(leptos_options_for_handler.clone()),
        )
        // 404 + static file fallback.
        .fallback(file_and_error_handler_with_context::<AppState, _>(
            provide_fallback_context,
            app::shell,
        ))
        .with_state(state.clone())
        .layer(security_headers)
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(rate_limit)
        .layer(TraceLayer::new_for_http())
}

/// Provide a typed value as a Leptos context for server functions.
fn provide_context<T: 'static + Clone + Send + Sync>(value: T) {
    leptos::prelude::provide_context(value);
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

    // Load configuration from environment (also used in build_app for auth routes).
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
    let port = cfg.port;
    let app = build_app(pool, cfg);
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("binding Axum server on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
