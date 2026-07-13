//! Tests extracted from `mod.rs` for Code Health scoring.

use super::*;
use crate::crawler::normalizer::RawTool;
use crate::crawler::sources::SourceCrawler;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A stub source that returns a fixed list of raw tools.
struct StubSource {
    name: &'static str,
    interval: &'static str,
    raws: Vec<RawTool>,
}

#[async_trait]
impl SourceCrawler for StubSource {
    async fn crawl(&self) -> anyhow::Result<Vec<RawTool>> {
        Ok(self.raws.clone())
    }
    fn source_name(&self) -> &str {
        self.name
    }
    fn interval(&self) -> &'static str {
        self.interval
    }
}

/// A stub source that always errors.
struct FailingSource {
    name: &'static str,
}

#[async_trait]
impl SourceCrawler for FailingSource {
    async fn crawl(&self) -> anyhow::Result<Vec<RawTool>> {
        Err(anyhow::anyhow!("simulated crawl failure"))
    }
    fn source_name(&self) -> &str {
        self.name
    }
    fn interval(&self) -> &'static str {
        "0 0 * * * *"
    }
}

fn raw(name: &str, repo: Option<&str>, stars: i32, desc: &str) -> RawTool {
    RawTool {
        name: name.into(),
        description: Some(desc.into()),
        tool_type: "mcp".into(),
        repo_url: repo.map(|s| s.to_string()),
        stars,
        source: "stub".into(),
        ..Default::default()
    }
}

#[test]
fn gated_approval_status_forces_pending_for_vendor_orgs_and_bazaar() {
    assert_eq!(gated_approval_status("vendor_orgs", false), "pending");
    assert_eq!(gated_approval_status("bazaar", false), "pending");
    assert_eq!(gated_approval_status("vendor_orgs", true), "pending");
    assert_eq!(gated_approval_status("npm", false), "approved");
    assert_eq!(gated_approval_status("npm", true), "pending");
}

#[test]
fn prepare_merged_crawl_tools_forces_pending_for_bazaar_when_auto_publish() {
    let bazaar_raw = raw("Bazaar Item", None, 1, "x402 payment api");
    let mut bazaar_raw = bazaar_raw;
    bazaar_raw.source = "bazaar".into();
    bazaar_raw.tool_type = "x402".into();
    bazaar_raw.pricing = "x402".into();

    let npm_raw = raw(
        "Npm Tool",
        Some("https://github.com/acme/pkg"),
        5,
        "swap dex",
    );
    let mut npm_raw = npm_raw;
    npm_raw.source = "npm".into();

    let outcomes = vec![
        (
            "bazaar".to_string(),
            default_source_registry_url("bazaar"),
            Ok(vec![bazaar_raw]),
        ),
        (
            "npm".to_string(),
            default_source_registry_url("npm"),
            Ok(vec![npm_raw]),
        ),
    ];
    let tools = prepare_merged_crawl_tools(&outcomes, false);
    assert_eq!(tools.len(), 2);
    let bazaar_tool = tools
        .iter()
        .find(|t| t.source == "bazaar")
        .expect("bazaar tool");
    let npm_tool = tools.iter().find(|t| t.source == "npm").expect("npm tool");
    assert_eq!(bazaar_tool.approval_status, "pending");
    assert_eq!(npm_tool.approval_status, "approved");
}

#[test]
fn prepare_crawled_tools_gated_forces_pending_for_bazaar_when_auto_publish() {
    let tools = prepare_crawled_tools_gated(
        &[raw("Bazaar Item", None, 1, "x402 payment api")],
        "bazaar",
        false,
    );
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].approval_status, "pending");
}

async fn test_pool() -> Option<sqlx::PgPool> {
    connect_test_pool(false).await
}

fn db_test_env_configured() -> bool {
    let _ = dotenvy::dotenv();
    ["SUPABASE_URL_TEST", "DATABASE_URL", "DATABASE_URL_TEST"]
        .iter()
        .any(|key| {
            std::env::var(key)
                .ok()
                .is_some_and(|url| !url.trim().is_empty())
        })
}

/// Skip when no DB env vars are set; fail when vars are set but connection fails.
async fn test_pool_required_if_configured() -> Option<sqlx::PgPool> {
    if !db_test_env_configured() {
        eprintln!(
            "SKIP: SUPABASE_URL_TEST, DATABASE_URL, or DATABASE_URL_TEST must be set for crawler DB integration test"
        );
        return None;
    }
    connect_test_pool(true).await
}

async fn connect_test_pool(required: bool) -> Option<sqlx::PgPool> {
    let _ = dotenvy::dotenv();
    let mut candidates = Vec::new();
    for key in ["SUPABASE_URL_TEST", "DATABASE_URL", "DATABASE_URL_TEST"] {
        if let Ok(url) = std::env::var(key) {
            let url = url.trim().to_string();
            if !url.is_empty() && !candidates.iter().any(|(k, _)| k == &key) {
                candidates.push((key, url));
            }
        }
    }

    if candidates.is_empty() {
        let msg =
            "SUPABASE_URL_TEST, DATABASE_URL, or DATABASE_URL_TEST must be set for crawler DB integration test";
        if required {
            panic!("{msg}");
        }
        eprintln!("SKIP: {msg}");
        return None;
    }

    let mut last_err = None;
    for (key, database_url) in candidates {
        match sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect(&database_url)
            .await
        {
            Ok(pool) => {
                eprintln!("crawler DB integration test connected via {key}");
                return Some(pool);
            }
            Err(e) => {
                eprintln!("crawler DB integration test: {key} connect failed ({e})");
                last_err = Some((key, e));
            }
        }
    }

    if required {
        let (key, err) = last_err.expect("candidate urls were empty");
        panic!("all database URLs failed — last attempt {key}: {err}");
    }
    eprintln!("SKIP: database connection failed — crawler DB integration test");
    None
}

#[tokio::test]
async fn upsert_x402_clobber_guard_preserves_self_listing_metadata() {
    let Some(pool) = test_pool().await else {
        return;
    };

    let mut tx = pool
        .begin()
        .await
        .expect("begin upsert_x402_clobber_guard test tx");

    let slug = format!("x402-clobber-{}", uuid::Uuid::new_v4());
    let reviewed_at = chrono::Utc::now();

    let mut seed_raw = raw(
        "Self Listed",
        Some("https://github.com/self/listed"),
        0,
        "x402 usdc payment checkout",
    );
    seed_raw.pricing = "x402".into();
    seed_raw.x402_price = Some("$0.05".into());
    seed_raw.x402_endpoint = Some("https://api.listed.example/x402/resource".into());
    seed_raw.tool_type = "x402".into();
    let mut seed_tool =
        normalizer::normalize(&seed_raw, &std::collections::HashSet::new(), "approved");
    seed_tool.slug = slug.clone();
    seed_tool.pricing = "x402".into();
    seed_tool.x402_price = Some("$0.05".into());
    seed_tool.x402_endpoint = Some("https://api.listed.example/x402/resource".into());
    seed_tool.relevance_status = "accepted".into();
    seed_tool.last_reviewed_at = Some(reviewed_at);

    eprintln!("upsert_x402_clobber_guard: begin tx, seed slug={slug}");
    upsert_tools(
        UpsertTarget::Connection(&mut tx),
        std::slice::from_ref(&seed_tool),
    )
    .await
    .expect("seed self-listed row via upsert_tools INSERT path");

    let mut crawl_raw = raw("Crawl Overwrite", None, 1, "free dev tool");
    crawl_raw.pricing = "free".into();
    crawl_raw.x402_price = Some("$9.99".into());
    crawl_raw.x402_endpoint = Some("https://evil.example/x402".into());
    let mut crawl_tool =
        normalizer::normalize(&crawl_raw, &std::collections::HashSet::new(), "approved");
    crawl_tool.slug = slug.clone();
    crawl_tool.pricing = "free".into();
    crawl_tool.x402_price = Some("$9.99".into());
    crawl_tool.x402_endpoint = Some("https://evil.example/x402".into());
    crawl_tool.relevance_status = "rejected".into();

    upsert_tools(
        UpsertTarget::Connection(&mut tx),
        std::slice::from_ref(&crawl_tool),
    )
    .await
    .expect("upsert conflicting crawl row on slug conflict");
    eprintln!("upsert_x402_clobber_guard: conflict upsert done, selecting row");

    let row: (String, Option<String>, Option<String>, String) = sqlx::query_as(
        r#"
        SELECT pricing, x402_price, x402_endpoint, relevance_status
        FROM tools WHERE slug = $1
        "#,
    )
    .bind(&slug)
    .fetch_one(&mut *tx)
    .await
    .expect("load post-upsert row");

    assert_eq!(row.0, "x402");
    assert_eq!(row.1.as_deref(), Some("$0.05"));
    assert_eq!(
        row.2.as_deref(),
        Some("https://api.listed.example/x402/resource")
    );
    assert_eq!(row.3, "accepted");
    eprintln!(
        "upsert_x402_clobber_guard: preserved pricing={} x402_price={:?} relevance={}",
        row.0, row.1, row.3
    );

    tx.rollback()
        .await
        .expect("rollback upsert_x402_clobber_guard test");
    eprintln!("upsert_x402_clobber_guard: rollback ok");
}

#[tokio::test]
async fn vendor_orgs_slug_rename_policy() {
    use crate::crawler::sources::vendor_orgs::{effective_tool_name, should_rename_repo_slug};

    assert!(should_rename_repo_slug("skills"));
    assert_eq!(
        effective_tool_name("circlefin", "skills"),
        "circlefin-skills"
    );

    let Some(pool) = test_pool_required_if_configured().await else {
        return;
    };

    let mut tx = pool
        .begin()
        .await
        .expect("begin vendor_orgs_slug_rename_policy test tx");

    let slug = format!("circlefin-skills-{}", uuid::Uuid::new_v4());
    let trusted_repo = "https://github.com/circlefin/skills";
    let trusted_homepage = "https://agents.circle.com/skills";

    let mut seed_tool = normalizer::normalize(
        &RawTool {
            name: "circlefin-skills".into(),
            description: Some("Official Circle skills".into()),
            tool_type: "sdk".into(),
            repo_url: Some(trusted_repo.into()),
            homepage: Some(trusted_homepage.into()),
            source: "manual".into(),
            ..Default::default()
        },
        &std::collections::HashSet::new(),
        "approved",
    );
    seed_tool.slug = slug.clone();
    seed_tool.status = "official".into();
    seed_tool.official_team = Some("Circle".into());

    upsert_tools(
        UpsertTarget::Connection(&mut tx),
        std::slice::from_ref(&seed_tool),
    )
    .await
    .expect("seed trusted official row");

    let mut crawl_tool = normalizer::normalize(
        &RawTool {
            name: "circlefin-skills".into(),
            description: Some("Crawler overwrite attempt".into()),
            tool_type: "mcp".into(),
            repo_url: Some("https://github.com/circlefin/skills-fork".into()),
            homepage: Some("https://evil.example/skills".into()),
            source: "vendor_orgs".into(),
            ..Default::default()
        },
        &std::collections::HashSet::new(),
        "pending",
    );
    crawl_tool.slug = slug.clone();

    upsert_tools(
        UpsertTarget::Connection(&mut tx),
        std::slice::from_ref(&crawl_tool),
    )
    .await
    .expect("upsert vendor_orgs crawl row on slug conflict");

    let row: (String, Option<String>, Option<String>, String) = sqlx::query_as(
        r#"
        SELECT name, repo_url, homepage, status
        FROM tools WHERE slug = $1
        "#,
    )
    .bind(&slug)
    .fetch_one(&mut *tx)
    .await
    .expect("load post-upsert trusted row");

    assert_eq!(row.0, "circlefin-skills");
    assert_eq!(row.1.as_deref(), Some(trusted_repo));
    assert_eq!(row.2.as_deref(), Some(trusted_homepage));
    assert_eq!(row.3, "official");
    eprintln!(
        "vendor_orgs_slug_rename_policy: trusted-row guard preserved name={} repo_url={:?} homepage={:?} status={}",
        row.0, row.1, row.2, row.3
    );

    tx.rollback()
        .await
        .expect("rollback vendor_orgs_slug_rename_policy test");
    eprintln!("vendor_orgs_slug_rename_policy: rollback ok");
}

/// After demotion clears `official_team`, a later crawl must not rehydrate
/// the label via `COALESCE(..., EXCLUDED.official_team)`.
#[tokio::test]
async fn demoted_official_team_not_rehydrated_on_upsert() {
    let Some(pool) = test_pool_required_if_configured().await else {
        return;
    };

    let mut tx = pool
        .begin()
        .await
        .expect("begin demoted_official_team_not_rehydrated_on_upsert tx");

    let slug = format!("demote-team-{}", uuid::Uuid::new_v4());
    let mut seed = normalizer::normalize(
        &RawTool {
            name: "demote-team-tool".into(),
            description: Some("was official, now community".into()),
            tool_type: "sdk".into(),
            repo_url: Some("https://github.com/circlefin/example".into()),
            source: "manual".into(),
            ..Default::default()
        },
        &std::collections::HashSet::new(),
        "approved",
    );
    seed.slug = slug.clone();
    seed.status = "community".into();
    seed.official_team = None;

    upsert_tools(
        UpsertTarget::Connection(&mut tx),
        std::slice::from_ref(&seed),
    )
    .await
    .expect("seed demoted community row");

    let mut crawl = normalizer::normalize(
        &RawTool {
            name: "demote-team-tool".into(),
            description: Some("vendor crawl wants team label back".into()),
            tool_type: "sdk".into(),
            repo_url: Some("https://github.com/circlefin/example".into()),
            source: "vendor_orgs".into(),
            official_team: Some("Circle".into()),
            ..Default::default()
        },
        &std::collections::HashSet::new(),
        "pending",
    );
    crawl.slug = slug.clone();
    crawl.official_team = Some("Circle".into());

    upsert_tools(
        UpsertTarget::Connection(&mut tx),
        std::slice::from_ref(&crawl),
    )
    .await
    .expect("upsert crawl onto demoted row");

    let row: (String, Option<String>) = sqlx::query_as(
        r#"
        SELECT status, official_team
        FROM tools WHERE slug = $1
        "#,
    )
    .bind(&slug)
    .fetch_one(&mut *tx)
    .await
    .expect("load demoted row after crawl");

    assert_eq!(row.0, "community");
    assert_eq!(
        row.1, None,
        "crawl must not rehydrate official_team after demotion"
    );

    tx.rollback()
        .await
        .expect("rollback demoted_official_team_not_rehydrated_on_upsert");
}

#[tokio::test]
async fn persist_crawl_results_gated_respects_force_pending() {
    let Some(pool) = test_pool().await else {
        return;
    };

    let mut tx = pool
        .begin()
        .await
        .expect("begin persist_crawl_results_gated test tx");

    let suffix = uuid::Uuid::new_v4();
    let test_url = format!("https://test.example/vendor-orgs-{suffix}");
    let raws = vec![RawTool {
        name: format!("vendor-org-gated-{suffix}"),
        description: Some("x402 vendor org repo".into()),
        tool_type: "mcp".into(),
        source: "vendor_orgs".into(),
        ..Default::default()
    }];
    let expected_slug = prepare_crawled_tools_gated(&raws, "vendor_orgs", false)[0]
        .slug
        .clone();

    eprintln!(
        "persist_crawl_results_gated: begin tx, require_tool_approval=false injected, slug={expected_slug}"
    );
    persist_crawl_results_gated_with_require(
        UpsertTarget::Connection(&mut tx),
        "vendor_orgs",
        &test_url,
        raws,
        false,
    )
    .await;

    let approval_status: String =
        sqlx::query_scalar("SELECT approval_status FROM tools WHERE slug = $1")
            .bind(&expected_slug)
            .fetch_one(&mut *tx)
            .await
            .expect("gated upsert row exists");

    assert_eq!(approval_status, "pending");
    eprintln!("persist_crawl_results_gated: approval_status={approval_status}");

    tx.rollback()
        .await
        .expect("rollback persist_crawl_results_gated test");
    eprintln!("persist_crawl_results_gated: rollback ok");
}

#[tokio::test]
async fn pipeline_runs_sources_in_parallel_and_merges() {
    let crawlers: Vec<Arc<dyn SourceCrawler>> = vec![
        Arc::new(StubSource {
            name: "alpha",
            interval: "0 0 * * * *",
            raws: vec![
                raw(
                    "Alpha Bridge",
                    Some("https://github.com/a/a"),
                    10,
                    "bridge cross-chain",
                ),
                raw(
                    "Beta Swap",
                    Some("https://github.com/b/b"),
                    20,
                    "uniswap dex swap",
                ),
            ],
        }),
        Arc::new(StubSource {
            name: "beta",
            interval: "0 0 * * * *",
            raws: vec![raw("Gamma Agent", None, 5, "autonomous ai agent eliza")],
        }),
    ];
    let tools = run_pipeline(crawlers).await;
    assert_eq!(tools.len(), 3);
    assert!(tools.iter().any(|t| t.function == "bridge"));
    assert!(tools.iter().any(|t| t.function == "swap"));
    assert!(tools
        .iter()
        .any(|t| t.actor == "ai-agent" && t.name == "Gamma Agent"));
}

#[tokio::test]
async fn pipeline_dedupes_across_sources() {
    // Two sources return the same repo_url; deduper keeps higher stars.
    let crawlers: Vec<Arc<dyn SourceCrawler>> = vec![
        Arc::new(StubSource {
            name: "alpha",
            interval: "0 0 * * * *",
            raws: vec![raw(
                "Low",
                Some("https://github.com/dup/dup"),
                1,
                "swap dex",
            )],
        }),
        Arc::new(StubSource {
            name: "beta",
            interval: "0 0 * * * *",
            raws: vec![raw(
                "High",
                Some("https://github.com/dup/dup"),
                999,
                "swap dex",
            )],
        }),
    ];
    let tools = run_pipeline(crawlers).await;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].stars, 999);
}

#[tokio::test]
async fn pipeline_continues_when_source_fails() {
    let call_count = Arc::new(AtomicUsize::new(0));
    struct CountingStub {
        count: Arc<AtomicUsize>,
    }
    #[async_trait]
    impl SourceCrawler for CountingStub {
        async fn crawl(&self) -> anyhow::Result<Vec<RawTool>> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(vec![raw("Survivor", None, 1, "staking yield")])
        }
        fn source_name(&self) -> &str {
            "survivor"
        }
        fn interval(&self) -> &'static str {
            "0 0 * * * *"
        }
    }
    let crawlers: Vec<Arc<dyn SourceCrawler>> = vec![
        Arc::new(FailingSource { name: "failer" }),
        Arc::new(CountingStub {
            count: call_count.clone(),
        }),
    ];
    let tools = run_pipeline(crawlers).await;
    // Failing source contributed nothing; survivor source still ran.
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "Survivor");
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn pipeline_empty_sources_returns_empty() {
    let crawlers: Vec<Arc<dyn SourceCrawler>> = vec![];
    let tools = run_pipeline(crawlers).await;
    assert!(tools.is_empty());
}

#[test]
fn prepare_crawled_tools_pending_when_approval_required() {
    let tools =
        prepare_crawled_tools(&[raw("Pending Tool", None, 1, "bridge cross-chain")], true);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].approval_status, "pending");
    assert_eq!(tools[0].function, "bridge");
}

#[test]
fn prepare_crawled_tools_approved_when_auto_publish() {
    let tools = prepare_crawled_tools(&[raw("Auto Tool", None, 1, "swap dex")], false);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].approval_status, "approved");
    assert_eq!(tools[0].function, "swap");
}

#[test]
fn count_raws_per_source_groups_by_source_field() {
    let mut a = raw("One", None, 1, "bridge");
    a.source = "npm".into();
    let mut b = raw("Two", None, 2, "swap");
    b.source = "npm".into();
    let mut c = raw("Three", None, 3, "bridge");
    c.source = "github".into();
    let counts = count_raws_per_source(&[a, b, c]);
    assert_eq!(counts.get("npm"), Some(&2));
    assert_eq!(counts.get("github"), Some(&1));
}

#[test]
fn default_source_registry_url_matches_run_once_urls() {
    assert_eq!(
        default_source_registry_url("npm"),
        "https://registry.npmjs.org/"
    );
    assert_eq!(
        default_source_registry_url("github"),
        "https://github.com/topics"
    );
    assert_eq!(
        default_source_registry_url("mcp-registry"),
        "https://registry.modelcontextprotocol.io/v0/servers"
    );
    assert_eq!(
        default_source_registry_url("vendor_orgs"),
        "https://api.github.com/orgs"
    );
    assert_eq!(
        default_source_registry_url("bazaar"),
        "https://api.cdp.coinbase.com/platform/v2/x402/discovery/resources"
    );
    assert_eq!(default_source_registry_url("pypi"), "https://pypi.org/");
}

#[test]
fn prepare_crawled_tools_dedupes_duplicate_repo_urls() {
    let raws = [
        raw(
            "Low Stars",
            Some("https://github.com/dup/dup"),
            1,
            "swap dex",
        ),
        raw(
            "High Stars",
            Some("https://github.com/dup/dup"),
            999,
            "swap dex",
        ),
    ];
    let tools = prepare_crawled_tools(&raws, false);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].stars, 999);
    assert_eq!(tools[0].approval_status, "approved");
}

#[tokio::test]
async fn pipeline_unique_slugs_across_sources() {
    // Two distinct tools with the same name from different sources.
    let crawlers: Vec<Arc<dyn SourceCrawler>> = vec![
        Arc::new(StubSource {
            name: "alpha",
            interval: "0 0 * * * *",
            raws: vec![raw("Same Name", None, 1, "bridge")],
        }),
        Arc::new(StubSource {
            name: "beta",
            interval: "0 0 * * * *",
            raws: vec![raw("Same Name", None, 2, "swap dex")],
        }),
    ];
    let tools = run_pipeline(crawlers).await;
    assert_eq!(tools.len(), 2);
    let slugs: Vec<_> = tools.iter().map(|t| t.slug.as_str()).collect();
    assert!(slugs.contains(&"same-name"));
    assert!(slugs.contains(&"same-name-2"));
}
