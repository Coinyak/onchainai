//! Integration tests calling `run_review_tool` — the same business logic
//! the admin workbench uses via `/api/v2/admin/review`.

#[cfg(feature = "ssr")]
mod ssr {
    use axum::http::Request;
    use onchainai::auth::session::{issue_access_token, ACCESS_TOKEN_COOKIE};
    use onchainai::config::Config;
    use onchainai::server::functions::{run_review_tool, ReviewToolPayload};
    use sqlx::postgres::PgPoolOptions;
    use std::fmt::Display;

    #[test]
    fn review_tool_db_test_required_flag_accepts_truthy_values() {
        assert!(review_tool_db_test_required_value(Some("1")));
        assert!(review_tool_db_test_required_value(Some("true")));
        assert!(review_tool_db_test_required_value(Some("YES")));
        assert!(!review_tool_db_test_required_value(Some("0")));
        assert!(!review_tool_db_test_required_value(None));
    }

    fn review_tool_db_test_required_value(value: Option<&str>) -> bool {
        value
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
    }

    fn review_tool_db_tests_required() -> bool {
        let value = std::env::var("ONCHAINAI_REQUIRE_DB_TESTS").ok();
        review_tool_db_test_required_value(value.as_deref())
    }

    fn skip_or_panic(context: &str, err: impl Display) {
        if review_tool_db_tests_required() {
            panic!("{context}: {err}");
        }
        eprintln!("SKIP: {context}: {err}");
    }

    fn configured_database_url() -> Result<String, String> {
        std::env::var("SUPABASE_URL_TEST")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::env::var("DATABASE_URL")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .ok_or_else(|| "missing SUPABASE_URL_TEST or DATABASE_URL".to_string())
    }

    async fn test_pool_and_config() -> Result<(sqlx::PgPool, Config), String> {
        let _ = dotenvy::dotenv();
        let database_url = configured_database_url()?;
        let config =
            Config::from_env().map_err(|e| format!("failed to load JWT/session config: {e}"))?;
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(15))
            .connect(&database_url)
            .await
            .map_err(|e| format!("failed to connect test database: {e}"))?;
        Ok((pool, config))
    }

    fn admin_request_parts(admin_id: uuid::Uuid, config: &Config) -> axum::http::request::Parts {
        let token = issue_access_token(
            admin_id,
            &config.jwt_secret,
            config.siwx_session_ttl,
            &config.jwt_issuer(),
        )
        .expect("mint admin access token for review_tool test");
        let cookie = format!("{ACCESS_TOKEN_COOKIE}={token}");
        Request::builder()
            .header(axum::http::header::COOKIE, cookie)
            .body(())
            .expect("build admin request")
            .into_parts()
            .0
    }

    async fn call_run_review_tool(
        pool: &sqlx::PgPool,
        admin_id: uuid::Uuid,
        payload: ReviewToolPayload,
    ) -> Result<(), onchainai::server::fn_error::FnError> {
        run_review_tool(pool, admin_id, &payload).await
    }

    async fn admin_operator_id(pool: &sqlx::PgPool) -> Option<uuid::Uuid> {
        sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT id FROM profiles WHERE is_admin = true ORDER BY created_at LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .ok()?
    }

    async fn cleanup_tool(pool: &sqlx::PgPool, tool_id: uuid::Uuid) {
        let _ = sqlx::query("DELETE FROM operator_verdicts WHERE tool_id = $1")
            .bind(tool_id)
            .execute(pool)
            .await;
        let _ = sqlx::query(
            "DELETE FROM review_entries WHERE review_run_id IN (SELECT id FROM review_runs WHERE tool_id = $1)",
        )
        .bind(tool_id)
        .execute(pool)
        .await;
        let _ = sqlx::query("DELETE FROM review_runs WHERE tool_id = $1")
            .bind(tool_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM tool_official_links WHERE tool_id = $1")
            .bind(tool_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM tools WHERE id = $1")
            .bind(tool_id)
            .execute(pool)
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn review_tool_approves_claim_pending_listing() {
        let (pool, config) = match test_pool_and_config().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("review_tool approve claim_pending DB setup", err);
                return;
            }
        };

        let Some(admin_id) = admin_operator_id(&pool).await else {
            skip_or_panic(
                "review_tool approve claim_pending DB setup",
                "no admin profile",
            );
            return;
        };

        let _parts = admin_request_parts(admin_id, &config);

        let slug = format!("review-tool-test-{}", uuid::Uuid::new_v4());
        let tool_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO tools (slug, name, description, approval_status, claim_state, install_risk_level) \
             VALUES ($1, 'Review Tool Test', 'integration test', 'pending', 'claim_pending', 'low') \
             RETURNING id",
        )
        .bind(&slug)
        .fetch_one(&pool)
        .await
        .expect("insert test tool");

        let payload = ReviewToolPayload {
            slug: slug.clone(),
            action: "approve".into(),
            reason: "integration test approve".into(),
            override_reason: None,
            expected_updated_at: None,
            snapshot_id: None,
            recommendation_id: None,
        };

        call_run_review_tool(&pool, admin_id, payload)
            .await
            .expect("run_review_tool() must approve claim_pending listing");

        let claim_state: String = sqlx::query_scalar("SELECT claim_state FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&pool)
            .await
            .expect("read claim_state after review_tool()");

        assert_eq!(claim_state, "unclaimed");

        let verdict_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM operator_verdicts WHERE tool_id = $1 AND action = 'approve'",
        )
        .bind(tool_id)
        .fetch_one(&pool)
        .await
        .expect("count operator verdicts after review_tool()");

        assert_eq!(verdict_count, 1);

        cleanup_tool(&pool, tool_id).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn review_tool_transitions_claim_pending_to_claimed() {
        let (pool, _config) = match test_pool_and_config().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("review_tool claim transition DB setup", err);
                return;
            }
        };

        let Some(admin_id) = admin_operator_id(&pool).await else {
            skip_or_panic("review_tool claim transition DB setup", "no admin profile");
            return;
        };

        let slug = format!("review-tool-claim-{}", uuid::Uuid::new_v4());
        let tool_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO tools (slug, name, description, approval_status, claim_state, install_risk_level) \
             VALUES ($1, 'Claim Test', 'integration test', 'approved', 'claim_pending', 'low') \
             RETURNING id",
        )
        .bind(&slug)
        .fetch_one(&pool)
        .await
        .expect("insert test tool");

        let payload = ReviewToolPayload {
            slug: slug.clone(),
            action: "mark_claimed".into(),
            reason: "integration test claim".into(),
            override_reason: None,
            expected_updated_at: None,
            snapshot_id: None,
            recommendation_id: None,
        };

        call_run_review_tool(&pool, admin_id, payload)
            .await
            .expect("run_review_tool() must transition claim_pending to claimed");

        let claim_state: String = sqlx::query_scalar("SELECT claim_state FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&pool)
            .await
            .expect("read claim_state");

        assert_eq!(claim_state, "claimed");

        cleanup_tool(&pool, tool_id).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn review_tool_mark_official_after_claim_and_verified_links() {
        let (pool, _config) = match test_pool_and_config().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("review_tool mark_official DB setup", err);
                return;
            }
        };

        let Some(admin_id) = admin_operator_id(&pool).await else {
            skip_or_panic("review_tool mark_official DB setup", "no admin profile");
            return;
        };

        let slug = format!("review-tool-official-{}", uuid::Uuid::new_v4());
        let tool_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO tools (slug, name, description, approval_status, claim_state, trust_state, install_risk_level) \
             VALUES ($1, 'Official Test', 'integration test', 'approved', 'claimed', 'verified', 'low') \
             RETURNING id",
        )
        .bind(&slug)
        .fetch_one(&pool)
        .await
        .expect("insert test tool");

        sqlx::query(
            "INSERT INTO tool_official_links (tool_id, url, link_type, verification_status) \
             VALUES ($1, 'https://example.com', 'website', 'verified')",
        )
        .bind(tool_id)
        .execute(&pool)
        .await
        .expect("insert verified official link");

        let payload = ReviewToolPayload {
            slug: slug.clone(),
            action: "mark_official".into(),
            reason: "integration test official".into(),
            override_reason: None,
            expected_updated_at: None,
            snapshot_id: None,
            recommendation_id: None,
        };

        call_run_review_tool(&pool, admin_id, payload)
            .await
            .expect("run_review_tool() must mark_official after claim and verified links");

        let trust_state: String = sqlx::query_scalar("SELECT trust_state FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&pool)
            .await
            .expect("read trust_state");

        assert_eq!(trust_state, "official");

        let verdict_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM operator_verdicts WHERE tool_id = $1 AND action = 'mark_official'",
        )
        .bind(tool_id)
        .fetch_one(&pool)
        .await
        .expect("count mark_official verdicts after review_tool()");

        assert_eq!(verdict_count, 1);

        cleanup_tool(&pool, tool_id).await;
    }
}
