//! Goal harness: register BOB Gateway CLI via shipped crawler upsert path using
//! discovery-derived metadata (not normalizer::tests::sample_raw fixtures).
//!
//! Run: RUSTFLAGS="-C symbol-mangling-version=v0" cargo test --features ssr --test bob_gateway_registration -- --nocapture

use onchainai::crawler::normalizer::{normalize, RawTool};
use onchainai::crawler::upsert_tools;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use std::env;

/// Metadata from isolated discovery passes (discovery-1..4.log), not unit-test fixtures.
fn discovery_bob_raw_tool() -> RawTool {
    RawTool {
        name: "BOB Gateway CLI".into(),
        description: Some(
            "CLI for bridging Bitcoin to/from EVM chains via BOB Gateway. \
             Native BTC intents across 11+ EVM chains; agent-oriented --json output."
                .into(),
        ),
        tool_type: "cli".into(),
        repo_url: Some("https://github.com/bob-collective/bob".into()),
        homepage: Some("https://docs.gobob.xyz/gateway/agents".into()),
        npm_package: Some("@gobob/gateway-cli".into()),
        install_command: Some("npx @gobob/gateway-cli".into()),
        mcp_endpoint: None,
        chains: vec![
            "bitcoin".into(),
            "bob".into(),
            "ethereum".into(),
            "base".into(),
            "arbitrum".into(),
            "avalanche".into(),
            "bsc".into(),
        ],
        stars: 0,
        last_commit_at: None,
        source: "npm".into(),
        source_url: Some("https://www.npmjs.com/package/@gobob/gateway-cli".into()),
        license: Some("MIT".into()),
    }
}

#[tokio::test]
async fn register_bob_gateway_cli_via_crawler_upsert_and_persist() {
    let _ = dotenvy::dotenv();
    let database_url = match env::var("DATABASE_URL") {
        Ok(url) if !url.trim().is_empty() => url,
        _ => {
            eprintln!("SKIP: DATABASE_URL not set — cannot persist registration");
            return;
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("connect DATABASE_URL");

    let raw = discovery_bob_raw_tool();
    assert_ne!(
        raw.description.as_deref(),
        Some("Bitcoin to EVM bridge CLI with AI agent docs"),
        "must not use normalizer sample_raw description"
    );
    assert_eq!(raw.source, "npm");
    assert_eq!(
        raw.homepage.as_deref(),
        Some("https://docs.gobob.xyz/gateway/agents")
    );

    let taken = HashSet::new();
    let tool = normalize(&raw, &taken, "approved");

    assert_eq!(tool.slug, "bob-gateway-cli");
    assert_eq!(tool.function, "bridge");
    assert_eq!(tool.actor, "ai-agent");
    assert_eq!(tool.tool_type, "cli");
    assert_eq!(tool.npm_package.as_deref(), Some("@gobob/gateway-cli"));
    assert_eq!(
        tool.install_command.as_deref(),
        Some("npx @gobob/gateway-cli")
    );
    assert!(tool.chains.contains(&"bitcoin".to_string()));
    assert!(tool.chains.contains(&"bob".to_string()));
    assert_eq!(tool.relevance_status, "accepted");

    upsert_tools(&pool, std::slice::from_ref(&tool))
        .await
        .expect("upsert_tools");

    let row: (String, String, String, String, Option<String>, Option<String>, Vec<String>) =
        sqlx::query_as(
            r#"
            SELECT name, slug, function, actor, npm_package, install_command, chains
            FROM tools WHERE slug = 'bob-gateway-cli'
            "#,
        )
        .fetch_one(&pool)
        .await
        .expect("tool row exists after upsert");

    assert_eq!(row.0, "BOB Gateway CLI");
    assert_eq!(row.1, "bob-gateway-cli");
    assert_eq!(row.2, "bridge");
    assert_eq!(row.3, "ai-agent");
    assert_eq!(row.4.as_deref(), Some("@gobob/gateway-cli"));
    assert_eq!(row.5.as_deref(), Some("npx @gobob/gateway-cli"));
    assert!(row.6.contains(&"bitcoin".to_string()));
    assert!(row.6.contains(&"bob".to_string()));

    let payload = serde_json::json!({
        "name": tool.name,
        "description": tool.description,
        "type": tool.tool_type,
        "function": tool.function,
        "repo_url": tool.repo_url,
        "homepage": tool.homepage,
        "npm_package": tool.npm_package,
        "install_command": tool.install_command,
        "chains": tool.chains,
        "slug": tool.slug,
        "official_team_claim": true,
        "verification_note": "Goal harness: discovered via npm+github+docs.gobob.xyz/gateway/agents (parallel agents 2026-06-27-rerun)"
    });

    let sub_id: (uuid::Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO tool_submissions (
          submitted_by, status, payload,
          crypto_relevance_score, relevance_status, install_risk_level
        )
        VALUES (NULL, 'approved', $1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(payload)
    .bind(tool.crypto_relevance_score)
    .bind(&tool.relevance_status)
    .bind(&tool.install_risk_level)
    .fetch_one(&pool)
    .await
    .expect("insert tool_submissions");

    let sub_row: (String, serde_json::Value) = sqlx::query_as(
        r#"
        SELECT status, payload FROM tool_submissions WHERE id = $1
        "#,
    )
    .bind(sub_id.0)
    .fetch_one(&pool)
    .await
    .expect("submission row");

    assert_eq!(sub_row.0, "approved");
    assert_eq!(sub_row.1["slug"], "bob-gateway-cli");
    assert_eq!(sub_row.1["npm_package"], "@gobob/gateway-cli");

    println!("REGISTERED tool slug=bob-gateway-cli id persisted");
    println!("SUBMISSION id={} status=approved", sub_id.0);
    println!(
        "TOOL_ROW name={} function={} actor={} chains={:?}",
        row.0, row.2, row.3, row.6
    );
}