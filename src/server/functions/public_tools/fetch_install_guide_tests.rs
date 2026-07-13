//! Tests for public_tools::fetch_install_guide_tests

use super::*;

fn require_db_tests() -> bool {
    std::env::var("ONCHAINAI_REQUIRE_DB_TESTS")
        .ok()
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

async fn test_pool() -> Option<sqlx::PgPool> {
    let database_url = std::env::var("SUPABASE_URL_TEST")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()?;
    sqlx::PgPool::connect(&database_url).await.ok()
}

#[tokio::test]
async fn fetch_public_install_guide_loads_approved_tool_from_db() {
    let Some(pool) = test_pool().await else {
        if require_db_tests() {
            panic!("ONCHAINAI_REQUIRE_DB_TESTS=1 but DATABASE_URL is unavailable");
        }
        eprintln!("SKIP: DATABASE_URL not set — fetch_public_install_guide DB test");
        return;
    };

    let slug: Option<String> = sqlx::query_scalar(
        "SELECT slug FROM tools \
         WHERE approval_status = 'approved' \
           AND quarantined_at IS NULL \
           AND install_risk_level <> 'critical' \
           AND (install_command IS NOT NULL OR mcp_endpoint IS NOT NULL) \
         LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .expect("query approved tool slug");

    let Some(slug) = slug else {
        if require_db_tests() {
            panic!("ONCHAINAI_REQUIRE_DB_TESTS=1 but no eligible approved tool found");
        }
        eprintln!("SKIP: no eligible approved tool in database");
        return;
    };

    let guide = fetch_public_install_guide(&pool, &slug, "claude")
        .await
        .unwrap_or_else(|error| panic!("fetch_public_install_guide failed for {slug}: {error}"));

    assert_eq!(guide.slug, slug);
    assert_eq!(guide.platform, "claude");
    assert!(!guide.blocked);
    assert!(
        guide.copy_text.is_some() || guide.config_json.is_some(),
        "expected copy output for low/medium-risk tool"
    );

    let local = crate::public_install_guide::build_install_guide_for_platform(
        &fetch_tool_by_slug(&pool, &slug)
            .await
            .expect("reload tool")
            .expect("tool exists"),
        &slug,
        "claude",
    )
    .expect("local builder");
    assert_eq!(guide.copy_text, local.copy_text);
    assert_eq!(guide.config_json, local.config_json);
}

#[tokio::test]
async fn fetch_public_install_guide_returns_not_found_for_missing_slug() {
    let Some(pool) = test_pool().await else {
        eprintln!("SKIP: DATABASE_URL not set — missing slug test");
        return;
    };

    let missing = format!("missing-mcp-tool-{}", uuid::Uuid::new_v4());
    let result = fetch_public_install_guide(&pool, &missing, "claude").await;
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("tool not found"),
        "expected not-found error"
    );
}
