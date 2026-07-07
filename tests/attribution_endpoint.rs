//! HTTP integration tests for `POST /api/v2/tools/{slug}/attribution`.

#[cfg(feature = "ssr")]
mod ssr {
    use axum::body::{to_bytes, Body};
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use onchainai::build_app;
    use onchainai::config::Config;
    use sqlx::postgres::PgPoolOptions;
    use std::fmt::Display;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tower::ServiceExt;
    use uuid::Uuid;

    fn db_tests_required() -> bool {
        std::env::var("ONCHAINAI_REQUIRE_DB_TESTS")
            .ok()
            .is_some_and(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
    }

    fn skip_or_panic(context: &str, err: impl Display) {
        if db_tests_required() {
            panic!("{context}: {err}");
        }
        eprintln!("SKIP: {context}: {err}");
    }

    async fn test_pool_and_config() -> Result<(sqlx::PgPool, Config), String> {
        let _ = dotenvy::dotenv();
        let database_url = std::env::var("SUPABASE_URL_TEST")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::env::var("DATABASE_URL")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .ok_or_else(|| "missing SUPABASE_URL_TEST or DATABASE_URL".to_string())?;
        let config = Config::from_env().map_err(|e| format!("failed to load config: {e}"))?;
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(15))
            .connect(&database_url)
            .await
            .map_err(|e| format!("failed to connect test database: {e}"))?;
        Ok((pool, config))
    }

    fn attribution_post(slug: &str, body: serde_json::Value, ip_octet: u8) -> Request<Body> {
        let mut req = Request::builder()
            .method("POST")
            .uri(format!("/api/v2/tools/{slug}/attribution"))
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .expect("build attribution request");
        req.extensions_mut().insert(ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(203, 0, 113, ip_octet)),
            0,
        )));
        req
    }

    async fn response_json(res: axum::http::Response<Body>) -> (StatusCode, serde_json::Value) {
        let status = res.status();
        let bytes = to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("read response body");
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or_else(|_| serde_json::json!({}));
        (status, json)
    }

    async fn cleanup_tool(pool: &sqlx::PgPool, tool_id: Uuid) {
        let _ = sqlx::query("DELETE FROM referral_events WHERE tool_id = $1")
            .bind(tool_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM tools WHERE id = $1")
            .bind(tool_id)
            .execute(pool)
            .await;
    }

    async fn insert_referral_test_tool(pool: &sqlx::PgPool) -> (Uuid, String) {
        let slug = format!("attr-http-test-{}", Uuid::new_v4());
        let tool_id: Uuid = sqlx::query_scalar(
            "INSERT INTO tools (slug, name, description, approval_status, claim_state, install_risk_level, referral_enabled, pricing) \
             VALUES ($1, 'Attribution HTTP Test', 'integration', 'approved', 'unclaimed', 'low', true, 'free') \
             RETURNING id",
        )
        .bind(&slug)
        .fetch_one(pool)
        .await
        .expect("insert referral test tool");
        (tool_id, slug)
    }

    async fn insert_non_referral_test_tool(pool: &sqlx::PgPool) -> (Uuid, String) {
        let slug = format!("attr-http-skip-{}", Uuid::new_v4());
        let tool_id: Uuid = sqlx::query_scalar(
            "INSERT INTO tools (slug, name, description, approval_status, claim_state, install_risk_level, referral_enabled, pricing) \
             VALUES ($1, 'Attribution Skip Test', 'integration', 'approved', 'unclaimed', 'low', false, 'free') \
             RETURNING id",
        )
        .bind(&slug)
        .fetch_one(pool)
        .await
        .expect("insert non-referral test tool");
        (tool_id, slug)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn attribution_returns_404_for_missing_slug() {
        let (pool, config) = match test_pool_and_config().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("attribution 404 DB setup", err);
                return;
            }
        };
        let app = build_app(pool, config);
        let missing = format!("missing-attr-{}", Uuid::new_v4());
        let req = attribution_post(&missing, serde_json::json!({ "platform": "cursor" }), 7);
        let res = app.oneshot(req).await.expect("route attribution");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn attribution_returns_400_for_invalid_platform() {
        let (pool, config) = match test_pool_and_config().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("attribution 400 DB setup", err);
                return;
            }
        };
        let (tool_id, slug) = insert_referral_test_tool(&pool).await;
        let app = build_app(pool.clone(), config);
        let req = attribution_post(&slug, serde_json::json!({ "platform": "bad platform" }), 8);
        let res = app.oneshot(req).await.expect("route attribution");
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        cleanup_tool(&pool, tool_id).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn attribution_skips_when_referral_disabled() {
        let (pool, config) = match test_pool_and_config().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("attribution skip DB setup", err);
                return;
            }
        };
        let (tool_id, slug) = insert_non_referral_test_tool(&pool).await;
        let app = build_app(pool.clone(), config);
        let req = attribution_post(
            &slug,
            serde_json::json!({ "platform": "cursor", "attribution_session": "sess-skip" }),
            9,
        );
        let (status, json) =
            response_json(app.oneshot(req).await.expect("route attribution")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json.get("ok"), Some(&serde_json::json!(true)));
        assert_eq!(json.get("recorded"), Some(&serde_json::json!(false)));
        cleanup_tool(&pool, tool_id).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn attribution_records_and_dedups_session() {
        let (pool, config) = match test_pool_and_config().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("attribution record DB setup", err);
                return;
            }
        };
        let (tool_id, slug) = insert_referral_test_tool(&pool).await;
        let app = build_app(pool.clone(), config);
        let session = format!("sess-dedup-{}", Uuid::new_v4());
        let body = serde_json::json!({
            "platform": "cursor",
            "attribution_session": session,
        });

        let (status1, json1) = response_json(
            app.clone()
                .oneshot(attribution_post(&slug, body.clone(), 10))
                .await
                .expect("first attribution"),
        )
        .await;
        assert_eq!(status1, StatusCode::OK);
        assert_eq!(json1.get("recorded"), Some(&serde_json::json!(true)));

        let (status2, json2) = response_json(
            app.oneshot(attribution_post(&slug, body, 10))
                .await
                .expect("second attribution"),
        )
        .await;
        assert_eq!(status2, StatusCode::OK);
        assert_eq!(json2.get("recorded"), Some(&serde_json::json!(false)));

        cleanup_tool(&pool, tool_id).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn attribution_rate_limits_per_ip() {
        std::env::remove_var("ONCHAINAI_RELAX_RATE_LIMIT");
        let (pool, config) = match test_pool_and_config().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("attribution rate limit DB setup", err);
                return;
            }
        };
        let (tool_id, slug) = insert_referral_test_tool(&pool).await;
        let app = build_app(pool.clone(), config);
        let mut got_429 = false;
        for i in 0..65 {
            let session = format!("rate-{i}-{}", Uuid::new_v4());
            let req = attribution_post(
                &slug,
                serde_json::json!({ "platform": "cursor", "attribution_session": session }),
                55,
            );
            let res = app.clone().oneshot(req).await.expect("attribution burst");
            if res.status() == StatusCode::TOO_MANY_REQUESTS {
                got_429 = true;
                break;
            }
        }
        assert!(
            got_429,
            "attribution endpoint should return 429 after per-IP quota"
        );
        cleanup_tool(&pool, tool_id).await;
    }
}
