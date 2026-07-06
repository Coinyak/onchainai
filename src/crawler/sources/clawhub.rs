//! ClawHub source crawler.
//!
//! Queries the public ClawHub REST API for crypto-relevant agent skills.
//! CryptoSkill references `clawhub install {slug}` but does not ingest the
//! ClawHub catalog — this source closes that gap (e.g. `tinyplace`).

use std::collections::HashSet;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::crawler::normalizer::RawTool;
use crate::crawler::sources::{http_client, SourceCrawler};

const CLAWHUB_API_BASE: &str = "https://clawhub.ai/api/v1";
const CLAWHUB_WEB_BASE: &str = "https://clawhub.ai/skills";
const SOURCE_NAME: &str = "clawhub";
const MAX_FAILURE_QUERIES_IN_MSG: usize = 8;

/// Crypto/agent search queries run against `GET /api/v1/search`.
const SEARCH_QUERIES: &[&str] = &[
    "x402",
    "crypto",
    "solana",
    "web3",
    "defi",
    "blockchain",
    "agent wallet",
    "onchain",
    "mcp",
    "trading",
    "dex",
    "wallet",
    "tinyplace",
];

const SEARCH_LIMIT: u32 = 50;

/// Outcome of a ClawHub sweep; may include partial per-query failures.
#[derive(Debug)]
pub struct ClawHubCrawlOutcome {
    pub raws: Vec<RawTool>,
    pub failed_queries: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    results: Vec<SearchHit>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct SearchHit {
    slug: String,
    #[serde(rename = "displayName")]
    display_name: String,
    summary: Option<String>,
    #[serde(rename = "ownerHandle")]
    owner_handle: Option<String>,
}

fn skill_page_url(slug: &str) -> String {
    format!("{CLAWHUB_WEB_BASE}/{slug}")
}

fn format_failed_queries(failed_queries: &[String]) -> String {
    let shown: Vec<_> = failed_queries
        .iter()
        .take(MAX_FAILURE_QUERIES_IN_MSG)
        .cloned()
        .collect();
    let mut msg = format!(
        "partial query failures ({}): {}",
        failed_queries.len(),
        shown.join(", ")
    );
    if failed_queries.len() > MAX_FAILURE_QUERIES_IN_MSG {
        msg.push_str(&format!(
            " (+{} more)",
            failed_queries.len() - MAX_FAILURE_QUERIES_IN_MSG
        ));
    }
    msg
}

fn hit_to_raw(hit: &SearchHit) -> RawTool {
    let description = hit
        .summary
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .map(str::to_string);

    RawTool {
        name: hit.display_name.clone(),
        description,
        tool_type: "skill".to_string(),
        install_command: Some(format!("clawhub install {}", hit.slug)),
        // ClawHub slugs do not reliably map to GitHub repo names (e.g. tinyplace vs tiny.place).
        repo_url: None,
        source: SOURCE_NAME.to_string(),
        source_url: Some(skill_page_url(&hit.slug)),
        ..Default::default()
    }
}

fn insert_raw(raw: RawTool, seen: &mut HashSet<String>, out: &mut Vec<RawTool>) {
    let slug = raw
        .install_command
        .as_deref()
        .and_then(|cmd| cmd.strip_prefix("clawhub install "))
        .unwrap_or(&raw.name);
    if seen.insert(slug.to_string()) {
        out.push(raw);
    }
}

async fn search_query(
    client: &reqwest::Client,
    base_url: &str,
    query: &str,
) -> Result<Vec<RawTool>> {
    let response = client
        .get(format!("{base_url}/search"))
        .query(&[("q", query), ("limit", &SEARCH_LIMIT.to_string())])
        .send()
        .await
        .with_context(|| format!("fetching ClawHub search for {query}"))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("ClawHub search for {query} returned HTTP {status}");
    }

    let body: SearchResponse = response
        .json()
        .await
        .with_context(|| format!("parsing ClawHub search JSON for {query}"))?;

    Ok(body.results.iter().map(hit_to_raw).collect())
}

/// Crawl ClawHub using the production API base URL.
pub async fn crawl_with_base(base_url: &str) -> Result<ClawHubCrawlOutcome> {
    let client = http_client().context("building ClawHub HTTP client")?;
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut failed_queries = Vec::new();
    let mut success_count = 0usize;

    for query in SEARCH_QUERIES {
        match search_query(&client, base_url, query).await {
            Ok(hits) => {
                success_count += 1;
                for raw in hits {
                    insert_raw(raw, &mut seen, &mut out);
                }
            }
            Err(e) => {
                tracing::warn!(source = SOURCE_NAME, query, error = %e, "search query failed");
                failed_queries.push((*query).to_string());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    if success_count == 0 {
        anyhow::bail!(
            "all ClawHub search queries failed ({} queries)",
            SEARCH_QUERIES.len()
        );
    }

    Ok(ClawHubCrawlOutcome {
        raws: out,
        failed_queries,
    })
}

/// Fetch crypto-relevant skills from the production ClawHub API.
pub async fn crawl() -> Result<Vec<RawTool>> {
    let outcome = crawl_with_base(CLAWHUB_API_BASE).await?;
    if !outcome.failed_queries.is_empty() {
        tracing::warn!(
            source = SOURCE_NAME,
            failed = outcome.failed_queries.len(),
            error = %format_failed_queries(&outcome.failed_queries),
            "clawhub partial query failures (returning successful raws)"
        );
    }
    Ok(outcome.raws)
}

/// Run a full crawl and persist results.
pub async fn run_once(pool: &sqlx::PgPool) {
    match crawl_with_base(CLAWHUB_API_BASE).await {
        Ok(outcome) => {
            let count = outcome.raws.len() as i32;
            tracing::info!(
                source = SOURCE_NAME,
                count = outcome.raws.len(),
                failed_queries = outcome.failed_queries.len(),
                "crawl completed"
            );
            if !outcome.raws.is_empty() {
                crate::crawler::persist_crawl_results(
                    pool,
                    SOURCE_NAME,
                    CLAWHUB_API_BASE,
                    outcome.raws,
                )
                .await;
            }
            if !outcome.failed_queries.is_empty() {
                let msg = format_failed_queries(&outcome.failed_queries);
                crate::crawler::update_source_status(
                    crate::crawler::UpsertTarget::Pool(pool),
                    SOURCE_NAME,
                    CLAWHUB_API_BASE,
                    "error",
                    count,
                    Some(&msg),
                )
                .await;
            }
        }
        Err(e) => {
            tracing::error!(source = SOURCE_NAME, error = %e, "crawl failed");
            crate::crawler::update_source_status(
                crate::crawler::UpsertTarget::Pool(pool),
                SOURCE_NAME,
                CLAWHUB_API_BASE,
                "error",
                0,
                Some(&e.to_string()),
            )
            .await;
        }
    }
}

/// Crawler instance implementing [`SourceCrawler`].
pub struct ClawHubCrawler;

#[async_trait::async_trait]
impl SourceCrawler for ClawHubCrawler {
    async fn crawl(&self) -> Result<Vec<RawTool>> {
        crawl().await
    }

    fn source_name(&self) -> &str {
        SOURCE_NAME
    }

    fn interval(&self) -> &'static str {
        "0 10 */6 * * *"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_search_json() -> String {
        r#"{
            "results": [
                {
                    "score": 2.5,
                    "slug": "tinyplace",
                    "displayName": "tiny.place",
                    "summary": "Agent-to-agent social network with x402 payments on Solana.",
                    "ownerHandle": "tinyhumansai"
                },
                {
                    "score": 1.2,
                    "slug": "crypto-market-data",
                    "displayName": "Crypto Market Data",
                    "summary": "Realtime crypto prices for AI agents.",
                    "ownerHandle": "example"
                }
            ]
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn crawl_parses_search_hits_with_skill_install_command() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search"))
            .and(query_param("q", "x402"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sample_search_json()))
            .mount(&server)
            .await;

        let outcome = crawl_with_base(&server.uri())
            .await
            .expect("crawl should succeed");

        let tiny = outcome
            .raws
            .iter()
            .find(|r| r.name == "tiny.place")
            .expect("tiny.place hit");
        assert_eq!(tiny.source, "clawhub");
        assert_eq!(tiny.tool_type, "skill");
        assert_eq!(
            tiny.install_command.as_deref(),
            Some("clawhub install tinyplace")
        );
        assert!(tiny.repo_url.is_none());
        assert!(tiny
            .source_url
            .as_deref()
            .is_some_and(|u| u.contains("clawhub")));
    }

    #[tokio::test]
    async fn crawl_returns_err_when_all_queries_fail() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let err = crawl_with_base(&server.uri())
            .await
            .expect_err("all failed queries should error");
        assert!(err.to_string().contains("all ClawHub search queries failed"));
    }

    #[test]
    fn format_failed_queries_truncates_long_lists() {
        let failed: Vec<String> = (0..12).map(|i| format!("q{i}")).collect();
        let msg = format_failed_queries(&failed);
        assert!(msg.contains("partial query failures (12)"));
        assert!(msg.contains("(+4 more)"));
    }

    #[test]
    fn search_queries_include_x402_and_tinyplace() {
        assert!(SEARCH_QUERIES.contains(&"x402"));
        assert!(SEARCH_QUERIES.contains(&"tinyplace"));
    }
}