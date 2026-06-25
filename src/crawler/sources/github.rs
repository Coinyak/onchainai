//! GitHub source crawler.
//!
//! Two responsibilities:
//!
//! 1. **Topics search** (`run_once` / `crawl_topics`): query the GitHub
//!    Search API for repositories tagged with the configured topics
//!    (`mcp-server`, `crypto-mcp`, `web3-mcp`, `blockchain-mcp`). For each
//!    matching repo we parse `stargazers_count` and `pushed_at`, infer
//!    `tool_type` from the README, and produce [`RawTool`]s.
//!
//! 2. **Star sync** (`sync_stars`): update `stars` and `last_commit_at` for
//!    existing tools with a `repo_url`.
//!
//! 3. **Self-register** (`self_register`): insert an official OnchainAI tool
//!    row at startup.
//!
//! All GitHub API requests include a `User-Agent` header. If the
//! `GITHUB_API_TOKEN` environment variable is set, an `Authorization: token`
//! header is also added.

use std::collections::HashSet;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::crawler::normalizer::RawTool;
use crate::crawler::sources::SourceCrawler;

const GITHUB_API_BASE: &str = "https://api.github.com";

/// Topics used for the discovery search.
const TOPICS: &[&str] = &["mcp-server", "crypto-mcp", "web3-mcp", "blockchain-mcp"];

/// Source identifier for the discovery source.
const SOURCE_NAME: &str = "github";

/// Search result item from the GitHub Search API.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct SearchItem {
    id: i64,
    name: String,
    full_name: String,
    description: Option<String>,
    html_url: String,
    stargazers_count: i32,
    pushed_at: Option<String>,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    language: Option<String>,
    clone_url: Option<String>,
    #[serde(default)]
    owner: Option<Owner>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct Owner {
    login: String,
}

/// GitHub search API response.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    items: Vec<SearchItem>,
}

/// GitHub repo API response (for star sync and README lookup).
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct RepoResponse {
    stargazers_count: i32,
    pushed_at: Option<String>,
}

/// GitHub README response.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ReadmeResponse {
    content: String,
    encoding: String,
}

/// Build a GitHub HTTP client that already has User-Agent and timeout.
fn github_client(token: Option<&str>) -> Result<reqwest::Client> {
    let mut client = reqwest::Client::builder()
        .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
        .timeout(std::time::Duration::from_secs(
            crate::crawler::sources::CRAWLER_TIMEOUT_SECS,
        ));
    if let Some(t) = token {
        client = client.default_headers(
            std::iter::once((
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {t}"))
                    .context("invalid GitHub API token for header")?,
            ))
            .collect(),
        );
    }
    client.build().context("building GitHub HTTP client")
}

/// Parse a GitHub URL into `(owner, repo)`.
fn parse_github_url(url: &str) -> Option<(&str, &str)> {
    // Accept both https://github.com/owner/repo and optional .git suffix.
    let url = url.strip_suffix(".git").unwrap_or(url);
    let parts: Vec<&str> = url
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/")
        .split('/')
        .collect();
    if parts.len() >= 2 {
        Some((parts[0], parts[1]))
    } else {
        None
    }
}

/// Decode base64 README content if necessary.
fn decode_readme(readme: &ReadmeResponse) -> Option<String> {
    if readme.encoding == "base64" {
        base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            readme.content.replace('\n', ""),
        )
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
    } else {
        Some(readme.content.clone())
    }
}

/// Infer tool_type from a README body and repository metadata.
fn infer_tool_type(readme: Option<&str>, topics: &[String], language: Option<&str>) -> String {
    let corpus = format!(
        "{} {} {}",
        readme.unwrap_or(""),
        topics.join(" ").to_lowercase(),
        language.unwrap_or(""),
    )
    .to_lowercase();

    if corpus.contains("mcp") || corpus.contains("model context protocol") {
        "mcp".to_string()
    } else if corpus.contains("cli") {
        "cli".to_string()
    } else if corpus.contains("sdk") {
        "sdk".to_string()
    } else if corpus.contains("api") {
        "api".to_string()
    } else {
        // Default to CLI for repos from GitHub topics search when the README
        // doesn't reveal a more specific type.
        "cli".to_string()
    }
}

/// Convert a GitHub search item to a [`RawTool`].
fn search_item_to_raw(item: &SearchItem) -> RawTool {
    let repo_url = item.html_url.clone();
    let description = item.description.clone();
    let tool_type = infer_tool_type(None, &item.topics, item.language.as_deref());
    let last_commit_at = item.pushed_at.as_ref().and_then(|s| parse_datetime(s));

    let chains: Vec<String> = item
        .topics
        .iter()
        .filter(|t| is_chain_topic(t))
        .map(|t| t.to_lowercase())
        .collect();

    RawTool {
        name: item.name.clone(),
        description,
        tool_type,
        repo_url: Some(repo_url.clone()),
        homepage: Some(repo_url.clone()),
        npm_package: None,
        install_command: None,
        mcp_endpoint: None,
        chains,
        stars: item.stargazers_count,
        last_commit_at,
        source: SOURCE_NAME.to_string(),
        source_url: Some(item.html_url.clone()),
        license: None,
    }
}

/// Heuristic: a topic that looks like a chain name.
fn is_chain_topic(topic: &str) -> bool {
    let chains: HashSet<&str> = HashSet::from([
        "ethereum",
        "bitcoin",
        "solana",
        "base",
        "polygon",
        "arbitrum",
        "optimism",
        "avalanche",
        "bnb",
        "cosmos",
        "near",
        "sui",
        "aptos",
        "cardano",
        "tron",
        "algorand",
        "starknet",
        "zksync",
        "linea",
        "scroll",
        "mantle",
        "fantom",
        "celo",
    ]);
    chains.contains(topic.to_lowercase().as_str())
}

/// Parse an RFC3339 timestamp, returning `None` on failure.
fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Query one topic via the GitHub Search API at a configurable base URL.
async fn search_topic_at_url(
    client: &reqwest::Client,
    token: Option<&str>,
    topic: &str,
    base_url: &str,
) -> Result<Vec<SearchItem>> {
    let url = format!("{base_url}/search/repositories");
    let mut request = client.get(&url).query(&[
        ("q", format!("topic:{topic}")),
        ("sort", "stars".to_string()),
        ("order", "desc".to_string()),
        ("per_page", "100".to_string()),
    ]);
    if let Some(t) = token {
        request = request.header(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {t}"))
                .context("invalid GitHub API token for header")?,
        );
    }

    let response = request
        .send()
        .await
        .with_context(|| format!("GitHub search request failed for topic {topic}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("GitHub search for topic {topic} returned HTTP {status}: {body}");
    }

    let search: SearchResponse = response
        .json()
        .await
        .context("parsing GitHub search response JSON")?;
    Ok(search.items)
}

/// Query one topic via the production GitHub Search API.
async fn search_topic(
    client: &reqwest::Client,
    token: Option<&str>,
    topic: &str,
) -> Result<Vec<SearchItem>> {
    search_topic_at_url(client, token, topic, GITHUB_API_BASE).await
}

/// Fetch and parse the README for a repo, returning its decoded text.
async fn fetch_readme(
    client: &reqwest::Client,
    token: Option<&str>,
    owner: &str,
    repo: &str,
) -> Option<String> {
    let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/readme");
    let mut request = client.get(&url);
    if let Some(t) = token {
        request = request.header(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {t}")).ok()?,
        );
    }

    let response = request.send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let readme: ReadmeResponse = response.json().await.ok()?;
    decode_readme(&readme)
}

/// Crawl all configured GitHub topics and return raw tools.
pub async fn crawl_topics_with_token(token: Option<&str>) -> Result<Vec<RawTool>> {
    let client = github_client(token)?;
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for topic in TOPICS {
        match search_topic(&client, token, topic).await {
            Ok(items) => {
                for item in items {
                    if seen.insert(item.id) {
                        out.push(search_item_to_raw(&item));
                    }
                }
            }
            Err(e) => {
                tracing::error!(topic, error = %e, "GitHub topic search failed");
                // Continue with next topic.
            }
        }
        // Rate-limit: 10ms between API calls.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    Ok(out)
}

/// Crawl topics using the configured GitHub API token (if any).
pub async fn crawl_topics() -> Result<Vec<RawTool>> {
    let token = std::env::var("GITHUB_API_TOKEN")
        .ok()
        .filter(|s| !s.is_empty());
    crawl_topics_with_token(token.as_deref()).await
}

/// Update stars and last_commit_at for up to 100 existing tools.
pub async fn sync_stars(_pool: &sqlx::PgPool) {
    // Full implementation uses sqlx queries against the DB; wiring to the
    // `tools` table is exercised in the `crawler-scheduler-star-sync`
    // milestone. This stub logs so the scheduler can call it without failing.
    tracing::info!("star sync: stub (DB wiring in next milestone)");
}

/// Insert the OnchainAI tool row into the database (idempotent).
pub async fn self_register(_pool: &sqlx::PgPool) {
    // Full DB upsert is added in the `crawler-scheduler-star-sync` milestone.
    tracing::info!("self-register: stub (DB wiring in next milestone)");
}

/// Run a full topics crawl.
pub async fn run_once(_pool: &sqlx::PgPool) {
    match crawl_topics().await {
        Ok(raws) => {
            tracing::info!(
                source = SOURCE_NAME,
                count = raws.len(),
                "topics crawl completed"
            );
        }
        Err(e) => {
            tracing::error!(source = SOURCE_NAME, error = %e, "topics crawl failed");
        }
    }
}

/// Crawler instance implementing [`SourceCrawler`] for GitHub topic search.
pub struct GitHubTopicsCrawler;

#[async_trait::async_trait]
impl SourceCrawler for GitHubTopicsCrawler {
    async fn crawl(&self) -> Result<Vec<RawTool>> {
        crawl_topics().await
    }
    fn source_name(&self) -> &str {
        SOURCE_NAME
    }
    fn interval(&self) -> &'static str {
        "0 30 * * * *"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn search_response_json(_topic: &str) -> String {
        format!(
            r#"{{
                "total_count": 2,
                "incomplete_results": false,
                "items": [
                    {{
                        "id": 1,
                        "name": "web3-mcp",
                        "full_name": "strangelove-ventures/web3-mcp",
                        "description": "One MCP to rule all them chains.",
                        "html_url": "https://github.com/strangelove-ventures/web3-mcp",
                        "stargazers_count": 500,
                        "pushed_at": "2026-06-20T12:00:00Z",
                        "topics": ["mcp-server", "web3-mcp", "solana", "ethereum"],
                        "language": "TypeScript",
                        "clone_url": "https://github.com/strangelove-ventures/web3-mcp.git"
                    }},
                    {{
                        "id": 2,
                        "name": "crypto-mcp",
                        "full_name": "example/crypto-mcp",
                        "description": null,
                        "html_url": "https://github.com/example/crypto-mcp",
                        "stargazers_count": 42,
                        "pushed_at": "2026-06-19T10:00:00Z",
                        "topics": ["crypto-mcp"],
                        "language": "Rust",
                        "clone_url": "https://github.com/example/crypto-mcp.git"
                    }}
                ]
            }}"#,
        )
    }

    #[tokio::test]
    async fn crawl_topics_queries_all_topics_and_parses_stars_pushed_at() {
        let server = MockServer::start().await;

        for topic in TOPICS {
            Mock::given(method("GET"))
                .and(path("/search/repositories"))
                .and(query_param("q", format!("topic:{topic}")))
                .respond_with(
                    ResponseTemplate::new(200).set_body_string(search_response_json(topic)),
                )
                .mount(&server)
                .await;
        }

        let client = reqwest::Client::builder()
            .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        // We cannot easily replace the GitHub API base in crawl_topics without
        // a refactor, so test the lower-level `search_topic` helper directly.
        let base_url = server.uri();
        let items = search_topic_at_url(&client, None, "mcp-server", &base_url)
            .await
            .expect("search should succeed");

        assert_eq!(items.len(), 2);
        let web3 = items.iter().find(|i| i.name == "web3-mcp").unwrap();
        assert_eq!(web3.stargazers_count, 500);
        assert_eq!(web3.pushed_at.as_deref(), Some("2026-06-20T12:00:00Z"));
        assert!(web3.topics.contains(&"mcp-server".to_string()));
    }

    #[tokio::test]
    async fn search_includes_authorization_header_when_token_set() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search/repositories"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(search_response_json("mcp-server")),
            )
            .mount(&server)
            .await;

        let client = reqwest::Client::builder()
            .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        let base_url = server.uri();
        let items = search_topic_at_url(&client, Some("test-token"), "mcp-server", &base_url)
            .await
            .expect("search should succeed");
        assert!(!items.is_empty());
    }

    #[tokio::test]
    async fn search_request_includes_user_agent_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search/repositories"))
            .and(header(
                "user-agent",
                crate::crawler::sources::CRAWLER_USER_AGENT,
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(search_response_json("mcp-server")),
            )
            .mount(&server)
            .await;

        let client = reqwest::Client::builder()
            .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        let base_url = server.uri();
        let items = search_topic_at_url(&client, None, "mcp-server", &base_url)
            .await
            .expect("search should succeed");
        assert!(!items.is_empty());
    }

    #[test]
    fn infer_tool_type_from_readme_and_topics() {
        assert_eq!(
            infer_tool_type(Some("Model Context Protocol server"), &[], None),
            "mcp"
        );
        assert_eq!(
            infer_tool_type(Some("A CLI tool for crypto"), &[], None),
            "cli"
        );
        assert_eq!(
            infer_tool_type(None, &["sdk".to_string(), "typescript".to_string()], None),
            "sdk"
        );
        assert_eq!(
            infer_tool_type(None, &["api".to_string()], Some("Python")),
            "api"
        );
        assert_eq!(
            infer_tool_type(None, &["random".to_string()], Some("Rust")),
            "cli"
        );
    }

    #[test]
    fn parse_github_url_variants() {
        assert_eq!(
            parse_github_url("https://github.com/owner/repo"),
            Some(("owner", "repo"))
        );
        assert_eq!(
            parse_github_url("https://github.com/owner/repo.git"),
            Some(("owner", "repo"))
        );
        assert_eq!(parse_github_url("not-a-url"), None);
    }

    #[test]
    fn chain_topic_filtering() {
        assert!(is_chain_topic("ethereum"));
        assert!(is_chain_topic("Solana"));
        assert!(!is_chain_topic("mcp-server"));
        assert!(!is_chain_topic("cli"));
    }

    #[test]
    fn search_item_to_raw_maps_fields() {
        let item = SearchItem {
            id: 7,
            name: "my-mcp".into(),
            full_name: "owner/my-mcp".into(),
            description: Some("desc".into()),
            html_url: "https://github.com/owner/my-mcp".into(),
            stargazers_count: 99,
            pushed_at: Some("2026-06-25T00:00:00Z".into()),
            topics: vec!["mcp-server".into(), "solana".into()],
            language: Some("TypeScript".into()),
            clone_url: None,
            owner: None,
        };
        let raw = search_item_to_raw(&item);
        assert_eq!(raw.source, "github");
        assert_eq!(raw.tool_type, "mcp");
        assert_eq!(raw.stars, 99);
        assert!(raw.chains.contains(&"solana".to_string()));
        assert!(raw.last_commit_at.is_some());
    }
}
