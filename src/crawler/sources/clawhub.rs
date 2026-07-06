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

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    results: Vec<SearchHit>,
}

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

fn infer_repo_url(owner: Option<&str>, slug: &str) -> Option<String> {
    let owner = owner?.trim();
    if owner.is_empty() {
        return None;
    }
    Some(format!("https://github.com/{owner}/{slug}"))
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
        repo_url: infer_repo_url(hit.owner_handle.as_deref(), &hit.slug),
        source: SOURCE_NAME.to_string(),
        source_url: Some(skill_page_url(&hit.slug)),
        ..Default::default()
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
pub async fn crawl_with_base(base_url: &str) -> Result<Vec<RawTool>> {
    let client = http_client().context("building ClawHub HTTP client")?;
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for query in SEARCH_QUERIES {
        match search_query(&client, base_url, query).await {
            Ok(hits) => {
                for raw in hits {
                    let slug = raw
                        .install_command
                        .as_deref()
                        .and_then(|cmd| cmd.strip_prefix("clawhub install "))
                        .unwrap_or(&raw.name);
                    if seen.insert(slug.to_string()) {
                        out.push(raw);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(source = SOURCE_NAME, query, error = %e, "search query failed");
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    Ok(out)
}

/// Fetch crypto-relevant skills from the production ClawHub API.
pub async fn crawl() -> Result<Vec<RawTool>> {
    crawl_with_base(CLAWHUB_API_BASE).await
}

/// Run a full crawl and persist results.
pub async fn run_once(pool: &sqlx::PgPool) {
    match crawl().await {
        Ok(raws) => {
            tracing::info!(source = SOURCE_NAME, count = raws.len(), "crawl completed");
            crate::crawler::persist_crawl_results(pool, SOURCE_NAME, CLAWHUB_API_BASE, raws).await;
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

        let raws = crawl_with_base(&server.uri())
            .await
            .expect("crawl should succeed");

        let tiny = raws
            .iter()
            .find(|r| r.name == "tiny.place")
            .expect("tiny.place hit");
        assert_eq!(tiny.source, "clawhub");
        assert_eq!(tiny.tool_type, "skill");
        assert_eq!(
            tiny.install_command.as_deref(),
            Some("clawhub install tinyplace")
        );
        assert_eq!(
            tiny.repo_url.as_deref(),
            Some("https://github.com/tinyhumansai/tinyplace")
        );
        assert!(tiny
            .source_url
            .as_deref()
            .is_some_and(|u| u.contains("clawhub")));
    }

    #[test]
    fn search_queries_include_x402_and_tinyplace() {
        assert!(SEARCH_QUERIES.contains(&"x402"));
        assert!(SEARCH_QUERIES.contains(&"tinyplace"));
    }
}
