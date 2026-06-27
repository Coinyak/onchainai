//! Integration tests calling the shipped `review_tool` server function — the same
//! entry point the admin workbench uses via `review_tool(ReviewToolPayload { ... })`.

#[cfg(feature = "ssr")]
mod ssr {
    use axum::http::Request;
    use leptos::prelude::{provide_context, Owner};
    use onchainai::auth::session::{issue_access_token, ACCESS_TOKEN_COOKIE};
    use onchainai::config::Config;
    use onchainai::server::functions::{review_tool, ReviewToolPayload};
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

    /// Invoke the public `review_tool` server fn with Leptos request context (admin UI path).
    async fn call_review_tool_server_fn(
        pool: sqlx::PgPool,
        config: Config,
        parts: axum::http::request::Parts,
        payload: ReviewToolPayload,
    ) -> Result<(), leptos::prelude::ServerFnError> {
        let owner = Owner::new();
        owner.with(|| {
            provide_context(pool);
            provide_context(config);
            provide_context(parts);
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(review_tool(payload))
            })
        })
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
        let _ = sqlx::query("DELETE FROM tool_review_events WHERE tool_id = $1")
            .bind(tool_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM tools WHERE id = $1")
            .bind(tool_id)
            .execute(pool)
            .await;
    }

    async fn insert_claim_pending_tool(pool: &sqlx::PgPool, slug: &str) -> uuid::Uuid {
        sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO tools (
                name, slug, description, function, asset_class, actor, type,
                repo_url, homepage, npm_package, install_command, chains,
                status, trust_score, approval_status, claim_state, relevance_status,
                last_commit_at, source, source_url
            )
            VALUES (
                'Review Tool Server Fn Test', $1, 'test', 'dev-tool', 'crypto',
                'human', 'mcp', 'https://github.com/org/repo', 'https://example.com',
                '@org/pkg', 'npx @org/pkg', ARRAY[]::text[],
                'community', 0, 'pending', 'claim_pending', 'accepted',
                now(), 'manual', 'https://example.com'
            )
            RETURNING id
            "#,
        )
        .bind(slug)
        .fetch_one(pool)
        .await
        .expect("insert claim_pending tool")
    }

    async fn insert_verified_official_links(
        pool: &sqlx::PgPool,
        tool_id: uuid::Uuid,
        operator_id: uuid::Uuid,
    ) {
        for (link_type, url) in [
            ("github", "https://github.com/org/repo"),
            ("website", "https://example.com"),
        ] {
            sqlx::query(
                r#"
                INSERT INTO tool_official_links (
                    tool_id, link_type, url, display_label, verification_status,
                    official_badge_allowed, evidence_strength, verification_method, verified_by
                )
                VALUES ($1, $2, $3, 'Official', 'verified', true, 'strong', 'operator_review', $4)
                "#,
            )
            .bind(tool_id)
            .bind(link_type)
            .bind(url)
            .bind(operator_id)
            .execute(pool)
            .await
            .expect("insert verified official link");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn review_tool_server_fn_approves_claim_pending_into_claimed() {
        let (pool, config) = match test_pool_and_config().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("review_tool server fn DB setup failed", err);
                return;
            }
        };
        let operator_id = match admin_operator_id(&pool).await {
            Some(value) => value,
            None => {
                skip_or_panic(
                    "review_tool server fn DB setup failed",
                    "no admin profile available",
                );
                return;
            }
        };

        let slug = format!("review-tool-server-fn-{}", uuid::Uuid::new_v4());
        let tool_id = insert_claim_pending_tool(&pool, &slug).await;
        let parts = admin_request_parts(operator_id, &config);

        call_review_tool_server_fn(
            pool.clone(),
            config,
            parts,
            ReviewToolPayload {
                slug: slug.clone(),
                action: "approved".into(),
                reason: "operator approved claim via review_tool server fn".into(),
                override_reason: None,
                expected_updated_at: None,
                snapshot_id: None,
                recommendation_id: None,
            },
        )
        .await
        .expect("review_tool() must approve claim_pending listing");

        let claim_state: String = sqlx::query_scalar("SELECT claim_state FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&pool)
            .await
            .expect("read claim_state after review_tool()");
        assert_eq!(claim_state, "claimed");

        let verdict_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM operator_verdicts WHERE tool_id = $1 AND action = 'approved'",
        )
        .bind(tool_id)
        .fetch_one(&pool)
        .await
        .expect("count operator verdicts after review_tool()");
        assert_eq!(verdict_count, 1);

        cleanup_tool(&pool, tool_id).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn review_tool_server_fn_mark_official_after_claim_and_verified_links() {
        let (pool, config) = match test_pool_and_config().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("review_tool mark_official DB setup failed", err);
                return;
            }
        };
        let operator_id = match admin_operator_id(&pool).await {
            Some(value) => value,
            None => {
                skip_or_panic(
                    "review_tool mark_official DB setup failed",
                    "no admin profile available",
                );
                return;
            }
        };

        let slug = format!("review-tool-official-{}", uuid::Uuid::new_v4());
        let tool_id = insert_claim_pending_tool(&pool, &slug).await;
        let parts = admin_request_parts(operator_id, &config);

        call_review_tool_server_fn(
            pool.clone(),
            config.clone(),
            parts.clone(),
            ReviewToolPayload {
                slug: slug.clone(),
                action: "approved".into(),
                reason: "claim proof accepted via review_tool".into(),
                override_reason: None,
                expected_updated_at: None,
                snapshot_id: None,
                recommendation_id: None,
            },
        )
        .await
        .expect("review_tool() must transition claim_pending to claimed");

        let claim_state: String = sqlx::query_scalar("SELECT claim_state FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&pool)
            .await
            .expect("read claim_state after review_tool approval");
        assert_eq!(claim_state, "claimed");

        insert_verified_official_links(&pool, tool_id, operator_id).await;

        call_review_tool_server_fn(
            pool.clone(),
            config,
            parts,
            ReviewToolPayload {
                slug: slug.clone(),
                action: "mark_official".into(),
                reason: "two strongly verified official links on file".into(),
                override_reason: None,
                expected_updated_at: None,
                snapshot_id: None,
                recommendation_id: None,
            },
        )
        .await
        .expect("review_tool() must mark_official after claim and verified links");

        let listing_status: String = sqlx::query_scalar("SELECT status FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&pool)
            .await
            .expect("read listing status after review_tool mark_official");
        assert_eq!(listing_status, "official");

        let verdict_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM operator_verdicts WHERE tool_id = $1 AND action = 'mark_official'",
        )
        .bind(tool_id)
        .fetch_one(&pool)
        .await
        .expect("count mark_official verdicts after review_tool()");
        assert_eq!(verdict_count, 1);

        cleanup_tool(&pool, tool_id).await;
    }
}
