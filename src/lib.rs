//! OnchainAI library — shared between SSR server binary and WASM hydration bundle.

#![recursion_limit = "256"]

pub mod app;
pub mod auth;
pub mod chains;
pub mod client_storage;
pub mod components;
pub mod config;
#[cfg(feature = "ssr")]
pub mod crawler;
pub mod filter_query;
pub mod install_safety;
pub mod models;
pub mod pages;
pub mod server;
pub mod trust_verification;
pub mod workbench;

pub use config::{Config, CANONICAL_DOMAIN, MCP_ENDPOINT_CMD, SITE_ORIGIN};

/// Shared application state for Axum + Leptos SSR.
#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Config,
    pub leptos_options: leptos::config::LeptosOptions,
}

#[cfg(feature = "ssr")]
impl axum::extract::FromRef<AppState> for leptos::config::LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos_options.clone()
    }
}

/// Build the Axum application router.
#[cfg(feature = "ssr")]
pub fn build_app(pool: sqlx::PgPool, config: Config) -> axum::Router {
    use axum::Router;
    use leptos::config::get_configuration;
    use leptos_axum::{file_and_error_handler_with_context, generate_route_list, LeptosRoutes};
    use tower::ServiceBuilder;
    use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
    use tower_http::{
        compression::CompressionLayer,
        cors::{AllowOrigin, CorsLayer},
        limit::RequestBodyLimitLayer,
        services::{ServeDir, ServeFile},
        set_header::SetResponseHeaderLayer,
        trace::TraceLayer,
    };

    let conf = get_configuration(Some("Cargo.toml")).expect("leptos configuration");
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(app::App);

    let siwx_domain = config.siwx_domain.clone();

    let state = AppState {
        pool,
        config,
        leptos_options: leptos_options.clone(),
    };

    let leptos_options_for_handler = leptos_options.clone();
    let state_for_context = state.clone();
    let state_for_fallback = state.clone();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin, _request_head| {
            let allowed = [
                SITE_ORIGIN.to_string(),
                format!("https://{siwx_domain}"),
                "http://localhost:3000".to_string(),
            ];
            if let Ok(origin_str) = origin.to_str() {
                allowed.iter().any(|a| a == origin_str)
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

    use crate::server::rate_limit::{AUTH_PER_MINUTE, GENERAL_PER_MINUTE, MCP_PER_MINUTE};

    // Catalog pages fan out ~6 read-only server-fn calls on hydrate; smoke tests
    // navigate quickly — allow short bursts above 60/min without blocking SSR reads.
    let relax_rate_limit = std::env::var("ONCHAINAI_RELAX_RATE_LIMIT")
        .map(|v| v == "1")
        .unwrap_or(false);
    let general_rate_limit = GovernorLayer::new(
        GovernorConfigBuilder::default()
            .per_second(5)
            .burst_size(GENERAL_PER_MINUTE.saturating_mul(2))
            .finish()
            .expect("general governor config"),
    );
    let auth_rate_limit = GovernorLayer::new(
        GovernorConfigBuilder::default()
            .per_second(12)
            .burst_size(AUTH_PER_MINUTE)
            .finish()
            .expect("auth governor config"),
    );
    let mcp_rate_limit = GovernorLayer::new(
        GovernorConfigBuilder::default()
            .per_millisecond(600)
            .burst_size(MCP_PER_MINUTE)
            .finish()
            .expect("mcp governor config"),
    );

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
                "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';",
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
            leptos::prelude::provide_context(state_for_context.pool.clone());
            leptos::prelude::provide_context(state_for_context.config.clone());
        }
    };
    let provide_fallback_context = {
        let state_for_fallback = state_for_fallback.clone();
        move || {
            leptos::prelude::provide_context(state_for_fallback.pool.clone());
            leptos::prelude::provide_context(state_for_fallback.config.clone());
        }
    };

    let auth_routes = Router::new()
        .route(
            "/auth/github",
            axum::routing::get(auth::routes::github_login),
        )
        .route(
            "/auth/email",
            axum::routing::post(auth::email::send_magic_link),
        )
        .route(
            "/auth/callback",
            axum::routing::get(auth::routes::oauth_callback),
        )
        .route("/auth/logout", axum::routing::post(auth::routes::logout))
        .route(
            "/onboarding/complete",
            axum::routing::post(auth::onboarding::complete),
        )
        .route(
            "/onboarding/skip",
            axum::routing::post(auth::onboarding::skip),
        )
        .route(
            "/auth/siwx/challenge",
            axum::routing::post(auth::siwx::challenge),
        )
        .route("/auth/siwx/verify", axum::routing::post(auth::siwx::verify))
        .with_state(state.clone())
        .layer(auth_rate_limit);

    let mcp_routes = Router::new()
        .route("/mcp", axum::routing::post(server::mcp::handle_mcp))
        .with_state(state.clone())
        .layer(mcp_rate_limit);

    let pkg_no_cache = std::env::var("ONCHAINAI_PKG_NO_CACHE")
        .map(|v| v == "1")
        .unwrap_or(false);
    let pkg_cache_control = if pkg_no_cache {
        axum::http::HeaderValue::from_static("no-store")
    } else {
        axum::http::HeaderValue::from_static("public, max-age=31536000, immutable")
    };
    let css_service = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CACHE_CONTROL,
            pkg_cache_control.clone(),
        ))
        .service(ServeFile::new("style/output.css"));
    let pkg_service = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CACHE_CONTROL,
            pkg_cache_control,
        ))
        .service(leptos_axum::site_pkg_dir_service(&leptos_options));

    let app_routes = Router::new()
        .route_service("/pkg/onchainai.css", css_service)
        .nest_service(
            "/chains",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::if_not_present(
                    axum::http::header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("public/chains").append_index_html_on_directories(false)),
        )
        .route_service(&static_route, pkg_service)
        .route(
            "/api/admin/operator/snapshot",
            axum::routing::get(server::operator_harness::get_operator_snapshot),
        )
        .route(
            "/api/admin/operator/run",
            axum::routing::post(server::operator_harness::post_operator_run),
        )
        .route(
            "/api/admin/operator/review-run",
            axum::routing::post(server::operator_harness::post_create_review_run),
        )
        .route(
            "/api/admin/operator/review-entry",
            axum::routing::post(server::operator_harness::post_append_review_entry),
        )
        .route(
            "/api/admin/operator/review-timeline",
            axum::routing::get(server::operator_harness::get_review_timeline),
        )
        .leptos_routes_with_context(&state, routes, provide_leptos_context, move || {
            app::shell(leptos_options_for_handler.clone())
        })
        .fallback(file_and_error_handler_with_context::<AppState, _>(
            provide_fallback_context,
            app::shell,
        ))
        .with_state(state.clone());

    let app_routes = if relax_rate_limit {
        app_routes
    } else {
        app_routes.layer(general_rate_limit)
    };

    Router::new()
        .merge(auth_routes)
        .merge(mcp_routes)
        .merge(app_routes)
        .layer(security_headers)
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(RequestBodyLimitLayer::new(8 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
}

#[cfg(feature = "ssr")]
async fn run_migrations(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| anyhow::anyhow!("migration failed: {e}"))
}

/// Start the Axum server (SSR binary entry).
#[cfg(feature = "ssr")]
pub async fn run_server() -> anyhow::Result<()> {
    use std::net::SocketAddr;

    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("OnchainAI starting up");

    let cfg = Config::from_env()?;
    tracing::info!(
        "config loaded (port={}, siwx_domain={})",
        cfg.port,
        cfg.siwx_domain
    );

    let pool = config::setup_db(&cfg.database_url).await?;
    tracing::info!("database pool initialized");
    run_migrations(&pool).await?;
    tracing::info!("migrations applied");

    let crawler_pool = pool.clone();
    let skip_crawler = std::env::var("SKIP_CRAWLER").is_ok();
    if skip_crawler {
        tracing::info!("crawler scheduler skipped (SKIP_CRAWLER set)");
    } else {
        tokio::spawn(async move {
            if let Err(e) = crawler::start_scheduler(crawler_pool).await {
                tracing::error!("crawler scheduler exited with error: {e}");
            }
        });
        tracing::info!("crawler scheduler spawned in background (tokio::spawn)");
    }

    let port = cfg.port;
    let app = build_app(pool, cfg);
    let addr = format!("0.0.0.0:{port}");
    tracing::info!("binding Axum server on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

/// WASM hydration entry — mounts interactive Leptos components in the browser.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
