//! OnchainAI library — Axum API server (Next.js frontend on Vercel).

#![recursion_limit = "256"]

pub mod auth;
pub mod chains;
pub mod config;
#[cfg(feature = "ssr")]
pub mod crawler;
pub mod discovery;
pub mod filter_query;
pub mod install_safety;
pub mod models;
pub mod public_install_guide;
pub mod server;
pub mod trust_verification;
pub mod vendor_orgs;
pub mod workbench;

pub use config::{Config, CANONICAL_DOMAIN, MCP_ENDPOINT_CMD, SITE_ORIGIN};

/// Shared application state for Axum handlers.
#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Config,
    /// True only when OKX x402 middleware is applied with non-empty route config.
    pub okx_premium_gate_active: bool,
}

#[cfg(feature = "ssr")]
fn canonical_www_location(host: &str, uri: &axum::http::Uri) -> Option<String> {
    let host_without_port = host.split(':').next().unwrap_or(host);
    if host_without_port != "onchain-ai.xyz" {
        return None;
    }
    let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
    Some(format!("https://www.onchain-ai.xyz{path_and_query}"))
}

#[cfg(feature = "ssr")]
async fn canonical_host_redirect(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::{header, HeaderValue, StatusCode};
    use axum::response::IntoResponse;

    let location = req
        .headers()
        .get(header::HOST)
        .and_then(|host| host.to_str().ok())
        .and_then(|host| canonical_www_location(host, req.uri()));

    if let Some(location) = location {
        if let Ok(value) = HeaderValue::from_str(&location) {
            let mut response = StatusCode::MOVED_PERMANENTLY.into_response();
            response.headers_mut().insert(header::LOCATION, value);
            return response;
        }
    }

    next.run(req).await
}

#[cfg(feature = "ssr")]
fn cache_control_for_response(path: &str) -> Option<axum::http::HeaderValue> {
    if path == "/mcp"
        || path.starts_with("/api/")
        || path.starts_with("/auth/")
        || path.starts_with("/onboarding/")
    {
        return Some(axum::http::HeaderValue::from_static("no-store"));
    }
    None
}

#[cfg(feature = "ssr")]
async fn cache_control_headers(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = req.uri().path().to_string();
    let mut response = next.run(req).await;

    if response
        .headers()
        .contains_key(axum::http::header::CACHE_CONTROL)
    {
        return response;
    }

    if let Some(value) = cache_control_for_response(&path) {
        response
            .headers_mut()
            .insert(axum::http::header::CACHE_CONTROL, value);
    }

    response
}

/// Origins allowed for browser cross-origin API calls (Vercel, local Next.js, SIWX).
#[cfg(feature = "ssr")]
fn cors_allowed_origins(siwx_domain: &str) -> Vec<String> {
    let mut allowed = vec![
        SITE_ORIGIN.to_string(),
        format!("https://{siwx_domain}"),
        "http://localhost:3000".to_string(),
        "http://localhost:3001".to_string(),
    ];
    if let Ok(extra) = std::env::var("FRONTEND_ORIGIN") {
        for origin in extra.split(',') {
            let trimmed = origin.trim();
            if !trimmed.is_empty() && !allowed.iter().any(|a| a == trimmed) {
                allowed.push(trimmed.to_string());
            }
        }
    }
    allowed
}

/// Build the Axum application router (API-only; no Leptos SSR).
#[cfg(feature = "ssr")]
pub async fn build_app(pool: sqlx::PgPool, config: Config) -> axum::Router {
    use axum::Router;
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

    let siwx_domain = config.siwx_domain.clone();

    // OKX init before AppState — handlers must know whether middleware is truly active.
    let okx_server = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        crate::server::okx_payment::init_okx_server(),
    )
    .await
    {
        Ok(Some(server)) => Some(server),
        Ok(None) => None,
        Err(_) => {
            tracing::warn!(
                "OKX facilitator init timed out (5s) — A2MCP disabled, CDP routes remain active"
            );
            None
        }
    };
    let okx_routes = crate::server::okx_payment::build_okx_routes();
    let okx_premium_gate_active = okx_server.is_some() && !okx_routes.is_empty();
    if okx_server.is_some() && okx_routes.is_empty() {
        tracing::warn!(
            "OKX credentials set but pay-to routes are empty — OKX A2MCP middleware skipped"
        );
    }

    let state = AppState {
        pool,
        config,
        okx_premium_gate_active,
    };

    let allowed_origins = cors_allowed_origins(&siwx_domain);
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin, _request_head| {
            if let Ok(origin_str) = origin.to_str() {
                allowed_origins.iter().any(|a| a == origin_str)
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
            axum::http::HeaderName::from_static("payment-signature"),
            axum::http::HeaderName::from_static("payment-required"),
            axum::http::HeaderName::from_static("payment-response"),
        ])
        .expose_headers([
            axum::http::HeaderName::from_static("payment-required"),
            axum::http::HeaderName::from_static("payment-response"),
        ])
        .allow_credentials(true);

    use crate::server::rate_limit::{AUTH_PER_MINUTE, GENERAL_PER_MINUTE, MCP_PER_MINUTE};

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
                "default-src 'none'; frame-ancestors 'none'; base-uri 'none';",
            ),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("x-xss-protection"),
            axum::http::HeaderValue::from_static("0"),
        ));

    let auth_routes = Router::new()
        .route(
            "/auth/github",
            axum::routing::get(auth::routes::github_login),
        )
        .route(
            "/auth/github/switch",
            axum::routing::post(auth::routes::github_switch),
        )
        .route(
            "/auth/google",
            axum::routing::get(auth::google::google_login),
        )
        .route(
            "/auth/google/callback",
            axum::routing::get(auth::google::google_callback),
        )
        .route(
            "/auth/email",
            axum::routing::post(auth::email::send_magic_link),
        )
        .route(
            "/auth/callback",
            axum::routing::get(auth::routes::oauth_callback),
        )
        .route(
            "/auth/logout",
            axum::routing::get(auth::routes::logout_get).post(auth::routes::logout),
        )
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
        .with_state(state.clone());

    let auth_routes = if relax_rate_limit {
        auth_routes
    } else {
        auth_routes.layer(auth_rate_limit)
    };

    let mcp_routes = Router::new()
        .route(
            "/mcp",
            axum::routing::post(server::mcp::handle_mcp).get(server::mcp::handle_mcp_info),
        )
        .with_state(state.clone())
        .layer(mcp_rate_limit);

    // Public static assets (brand logos, chain icons) — immutable cached.
    let static_routes = Router::new()
        .route_service("/favicon.ico", ServeFile::new("public/favicon.ico"))
        .route_service(
            "/apple-touch-icon.png",
            ServeFile::new("public/brand/onchainai-icon-180.png"),
        )
        .route_service(
            "/site.webmanifest",
            ServeFile::new("public/site.webmanifest"),
        )
        .nest_service(
            "/brand",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::if_not_present(
                    axum::http::header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("public/brand").append_index_html_on_directories(false)),
        )
        .nest_service(
            "/chains",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::if_not_present(
                    axum::http::header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("public/chains").append_index_html_on_directories(false)),
        );

    let operator_routes = Router::new()
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
        .with_state(state.clone());

    let operator_routes = if relax_rate_limit {
        operator_routes
    } else {
        operator_routes.layer(general_rate_limit)
    };

    let api_v2_routes = crate::server::api_v2::router(state.clone());

    let app = Router::new()
        .merge(auth_routes)
        .merge(mcp_routes)
        .merge(static_routes)
        .merge(api_v2_routes)
        .merge(operator_routes);

    let app = if okx_premium_gate_active {
        let server = okx_server.expect("okx_premium_gate_active implies okx_server");
        tracing::info!("Applying OKX x402 payment middleware to premium A2MCP routes");
        app.layer(x402_axum::payment_middleware(okx_routes, server))
    } else {
        app
    };

    app.layer(axum::middleware::from_fn(cache_control_headers))
        .layer(axum::middleware::from_fn(canonical_host_redirect))
        .layer(security_headers)
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(RequestBodyLimitLayer::new(8 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    fn cache_header_str(path: &str) -> Option<String> {
        cache_control_for_response(path)
            .map(|value| value.to_str().expect("valid cache-control").to_string())
    }

    #[test]
    fn canonical_www_location_redirects_apex_with_path_and_query() {
        let uri = "/tools?chain=base".parse().expect("valid uri");
        assert_eq!(
            canonical_www_location("onchain-ai.xyz", &uri),
            Some("https://www.onchain-ai.xyz/tools?chain=base".to_string())
        );
    }

    #[test]
    fn canonical_www_location_ignores_www_and_local_hosts() {
        let uri = "/".parse().expect("valid uri");
        assert_eq!(canonical_www_location("www.onchain-ai.xyz", &uri), None);
        assert_eq!(canonical_www_location("localhost:3000", &uri), None);
    }

    #[test]
    fn dynamic_mutation_and_auth_paths_are_not_stored() {
        for path in [
            "/api/v2/tools",
            "/auth/logout",
            "/onboarding/complete",
            "/mcp",
        ] {
            assert_eq!(
                cache_header_str(path).as_deref(),
                Some("no-store"),
                "{path}"
            );
        }
    }

    #[test]
    fn static_assets_do_not_get_dynamic_cache_policy() {
        assert!(cache_control_for_response("/brand/onchainai-logo.svg").is_none());
        assert!(cache_control_for_response("/chains/base.svg").is_none());
    }

    fn test_config() -> Config {
        Config {
            database_url: String::new(),
            supabase_url: "https://proj.supabase.co".into(),
            supabase_anon_key: String::new(),
            supabase_service_key: String::new(),
            github_client_id: String::new(),
            github_client_secret: String::new(),
            github_redirect_uri: None,
            google_client_id: None,
            google_client_secret: None,
            google_redirect_uri: None,
            siwx_domain: "localhost:3000".into(),
            siwx_session_ttl: 86_400,
            jwt_secret: "test-secret-at-least-32-bytes-long-aaaa".into(),
            github_api_token: None,
            admin_github_logins: Vec::new(),
            port: 3000,
        }
    }

    fn test_pool() -> sqlx::PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/onchainai_test")
            .expect("lazy pool")
    }

    fn request_with_ip(uri: &str) -> axum::http::Request<axum::body::Body> {
        use axum::extract::ConnectInfo;
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};
        let mut req: axum::http::Request<axum::body::Body> = axum::http::Request::builder()
            .uri(uri)
            .body(axum::body::Body::empty())
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
            0,
        )));
        req
    }

    #[tokio::test]
    async fn static_assets_bypass_general_rate_limiter() {
        std::env::remove_var("ONCHAINAI_RELAX_RATE_LIMIT");
        // Prevent live OKX network calls during tests.
        std::env::remove_var("OKX_API_KEY");

        let app = build_app(test_pool(), test_config()).await;
        for path in [
            "/favicon.ico",
            "/apple-touch-icon.png",
            "/site.webmanifest",
            "/brand/onchainai-logo.svg",
            "/chains/base.svg",
        ] {
            for attempt in 0..150 {
                let req = request_with_ip(path);
                let res = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
                assert_ne!(
                    res.status(),
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    "{path} hit 429 on request {}",
                    attempt + 1
                );
            }
        }
    }

    #[tokio::test]
    async fn dynamic_routes_stay_rate_limited() {
        std::env::remove_var("ONCHAINAI_RELAX_RATE_LIMIT");
        // Prevent live OKX network calls during tests.
        std::env::remove_var("OKX_API_KEY");

        let app = build_app(test_pool(), test_config()).await;
        let mut got_429 = false;
        for _ in 0..150 {
            let req = request_with_ip("/api/admin/operator/snapshot");
            let res = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
            if res.status() == axum::http::StatusCode::TOO_MANY_REQUESTS {
                got_429 = true;
                break;
            }
        }
        assert!(got_429, "dynamic route should hit 429 after burst");
    }
}

#[cfg(feature = "ssr")]
fn migrations_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("MIGRATIONS_DIR") {
        return std::path::PathBuf::from(dir);
    }
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

#[cfg(feature = "ssr")]
async fn run_migrations(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    let migrator = sqlx::migrate::Migrator::new(migrations_dir().as_path())
        .await
        .map_err(|e| anyhow::anyhow!("failed to load migrations: {e}"))?;
    migrator
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

    tracing::info!("OnchainAI API server starting up");

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
    let x402_pool = pool.clone();
    let skip_crawler = std::env::var("SKIP_CRAWLER").is_ok();
    let skip_x402_verify = std::env::var("X402_VERIFY_DISABLED").is_ok();
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

    if skip_x402_verify {
        tracing::info!("x402 verify scheduler skipped (X402_VERIFY_DISABLED set)");
    } else {
        tokio::spawn(async move {
            if let Err(e) = server::x402_verify::start_scheduler(x402_pool).await {
                tracing::error!("x402 verify scheduler exited with error: {e}");
            }
        });
        tracing::info!("x402 verify scheduler spawned in background (tokio::spawn)");
    }

    let port = cfg.port;
    let app = build_app(pool, cfg).await;
    let addr = format!("0.0.0.0:{port}");
    tracing::info!("binding Axum API server on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
