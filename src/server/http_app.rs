//! Axum application construction (routes, CORS, rate limits, OKX gate wiring).

use crate::config::{Config, SITE_ORIGIN};
use crate::AppState;

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
    tracing::info!("Initializing OKX A2MCP payment server (15s timeout)...");
    let okx_server = match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        crate::server::okx_payment::init_okx_server(),
    )
    .await
    {
        Ok(Some(server)) => {
            tracing::info!("OKX A2MCP payment server initialized successfully");
            Some(server)
        }
        Ok(None) => {
            tracing::warn!(
                "OKX A2MCP init returned None — credentials missing or facilitator error"
            );
            None
        }
        Err(_) => {
            tracing::warn!(
                "OKX facilitator init timed out (15s) — A2MCP disabled, CDP routes remain active"
            );
            None
        }
    };
    let okx_routes = crate::server::okx_payment::build_okx_routes();
    let okx_client = if okx_server.is_some() && !okx_routes.is_empty() {
        crate::server::okx_payment::create_okx_facilitator_client()
    } else {
        None
    };
    // Gate only when facilitator client is ready — avoids skipping CDP with no OKX gate.
    let okx_premium_gate_active = okx_client.is_some();
    if okx_server.is_some() && okx_routes.is_empty() {
        tracing::warn!(
            "OKX credentials set but pay-to routes are empty — OKX A2MCP middleware skipped"
        );
    }

    let state = AppState {
        pool,
        config,
        okx_premium_gate_active,
        okx_client,
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
            axum::routing::get(crate::auth::routes::github_login),
        )
        .route(
            "/auth/github/switch",
            axum::routing::post(crate::auth::routes::github_switch),
        )
        .route(
            "/auth/google",
            axum::routing::get(crate::auth::google::google_login),
        )
        .route(
            "/auth/google/callback",
            axum::routing::get(crate::auth::google::google_callback),
        )
        .route(
            "/auth/email",
            axum::routing::post(crate::auth::email::send_magic_link),
        )
        .route(
            "/auth/callback",
            axum::routing::get(crate::auth::routes::oauth_callback),
        )
        .route(
            "/auth/logout",
            axum::routing::get(crate::auth::routes::logout_get).post(crate::auth::routes::logout),
        )
        .route(
            "/onboarding/complete",
            axum::routing::post(crate::auth::onboarding::complete),
        )
        .route(
            "/onboarding/skip",
            axum::routing::post(crate::auth::onboarding::skip),
        )
        .route(
            "/auth/siwx/challenge",
            axum::routing::post(crate::auth::siwx::challenge),
        )
        .route("/auth/siwx/verify", axum::routing::post(crate::auth::siwx::verify))
        .with_state(state.clone());

    let auth_routes = if relax_rate_limit {
        auth_routes
    } else {
        auth_routes.layer(auth_rate_limit)
    };

    let mcp_routes = Router::new()
        .route(
            "/mcp",
            axum::routing::post(crate::server::mcp::handle_mcp).get(crate::server::mcp::handle_mcp_info),
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
            axum::routing::get(crate::server::operator_harness::get_operator_snapshot),
        )
        .route(
            "/api/admin/operator/run",
            axum::routing::post(crate::server::operator_harness::post_operator_run),
        )
        .route(
            "/api/admin/operator/review-run",
            axum::routing::post(crate::server::operator_harness::post_create_review_run),
        )
        .route(
            "/api/admin/operator/review-entry",
            axum::routing::post(crate::server::operator_harness::post_append_review_entry),
        )
        .route(
            "/api/admin/operator/review-timeline",
            axum::routing::get(crate::server::operator_harness::get_review_timeline),
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

    app.layer(axum::middleware::from_fn(crate::cache_control_headers))
        .layer(axum::middleware::from_fn(crate::canonical_host_redirect))
        .layer(security_headers)
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(RequestBodyLimitLayer::new(8 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
}
