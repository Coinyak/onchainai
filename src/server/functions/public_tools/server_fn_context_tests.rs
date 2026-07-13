//! Tests for public_tools::server_fn_context_tests


use super::{fetch_public_install_guide, fetch_tool_by_slug};
use crate::public_install_guide::{
    build_install_guide_for_platform, build_public_install_guide, resolve_install_guide,
    InstallPlatform,
};
use sqlx::postgres::PgPoolOptions;
use std::fmt::Display;

pub fn db_tests_required() -> bool {
    std::env::var("ONCHAINAI_REQUIRE_DB_TESTS")
        .ok()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

pub fn skip_or_panic(context: &str, err: impl Display) {
    if db_tests_required() {
        panic!("{context}: {err}");
    }
    eprintln!("SKIP: {context}: {err}");
}

pub async fn test_pool() -> Result<sqlx::PgPool, String> {
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
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(15))
        .connect(&database_url)
        .await
        .map_err(|error| format!("failed to connect test database: {error}"))
}

pub async fn call_fetch_public_install_guide(
    pool: &sqlx::PgPool,
    slug: &str,
    platform: &str,
) -> Result<crate::public_install_guide::PublicInstallGuide, crate::server::fn_error::FnError>
{
    fetch_public_install_guide(pool, slug, platform).await
}

pub async fn eligible_approved_slug(pool: &sqlx::PgPool) -> Option<String> {
    sqlx::query_scalar(
        "SELECT slug FROM tools \
         WHERE approval_status = 'approved' \
           AND quarantined_at IS NULL \
           AND install_risk_level <> 'critical' \
           AND (install_command IS NOT NULL OR mcp_endpoint IS NOT NULL) \
         LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()?
}

pub async fn run_get_public_install_guide_server_fn_loads_approved_tool() {
    let pool = match test_pool().await {
        Ok(value) => value,
        Err(err) => {
            skip_or_panic("get_public_install_guide server fn DB setup failed", err);
            return;
        }
    };

    let Some(slug) = eligible_approved_slug(&pool).await else {
        skip_or_panic(
            "get_public_install_guide server fn DB setup failed",
            "no eligible approved tool found",
        );
        return;
    };

    let guide = call_fetch_public_install_guide(&pool, &slug, "claude")
        .await
        .unwrap_or_else(|error| {
            panic!("fetch_public_install_guide() failed for {slug}: {error}")
        });

    assert_eq!(guide.slug, slug);
    assert_eq!(guide.platform, "claude");
    assert!(!guide.blocked);
    assert!(
        guide.copy_text.is_some() || guide.config_json.is_some(),
        "expected copy output for eligible tool"
    );
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn get_public_install_guide_server_fn_loads_approved_tool() {
    run_get_public_install_guide_server_fn_loads_approved_tool().await;
}

pub async fn run_get_public_install_guide_server_fn_returns_not_found_for_missing_slug() {
    let pool = match test_pool().await {
        Ok(value) => value,
        Err(err) => {
            skip_or_panic("get_public_install_guide missing slug test", err);
            return;
        }
    };

    let missing = format!("missing-mcp-tool-{}", uuid::Uuid::new_v4());
    let result = call_fetch_public_install_guide(&pool, &missing, "claude").await;

    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("tool not found"),
        "expected not-found error for {missing}"
    );
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn get_public_install_guide_server_fn_returns_not_found_for_missing_slug() {
    run_get_public_install_guide_server_fn_returns_not_found_for_missing_slug().await;
}

pub async fn run_install_guide_panel_chain_matches_server_fn_for_approved_tool() {
    let pool = match test_pool().await {
        Ok(value) => value,
        Err(err) => {
            skip_or_panic("install guide panel chain DB setup failed", err);
            return;
        }
    };

    let Some(slug) = eligible_approved_slug(&pool).await else {
        skip_or_panic(
            "install guide panel chain DB setup failed",
            "no eligible approved tool found",
        );
        return;
    };

    let tool = fetch_tool_by_slug(&pool, &slug)
        .await
        .expect("fetch_tool_by_slug must succeed for approved tool")
        .expect("approved tool must exist");

    let remote = call_fetch_public_install_guide(&pool, &slug, "claude")
        .await
        .expect("fetch must succeed for approved tool");

    let local = build_public_install_guide(&tool, &slug, InstallPlatform::Claude);
    let resolved = resolve_install_guide(Some(Ok(remote.clone())), local.clone());

    assert_eq!(resolved, remote);
    assert_eq!(resolved.copy_text, local.copy_text);
    assert_eq!(resolved.config_json, local.config_json);

    let direct = build_install_guide_for_platform(
        &fetch_tool_by_slug(&pool, &slug)
            .await
            .expect("reload tool")
            .expect("tool exists"),
        &slug,
        "claude",
    )
    .expect("platform builder must match server fn body");
    assert_eq!(resolved.copy_text, direct.copy_text);
    assert_eq!(resolved.config_json, direct.config_json);
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn install_guide_panel_chain_matches_server_fn_for_approved_tool() {
    run_install_guide_panel_chain_matches_server_fn_for_approved_tool().await;
}
