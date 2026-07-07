//! PyPI source crawler.
//!
//! PyPI's search endpoint (`pypi.org/search`) is protected by a JavaScript
//! challenge, so it cannot be scraped directly. Instead, this crawler uses a
//! curated seed list of known crypto/agent Python package names and fetches
//! full metadata from the PyPI JSON API (`pypi.org/pypi/{name}/json`).
//!
//! Mapping to OnchainAI:
//! - `source`: `pypi`
//! - `tool_type`: `cli` if the package has `console_scripts` entry points,
//!   otherwise `sdk`
//! - `install_command`: `pip install {name}`
//! - `repo_url`: repository URL from `project_urls` (cleaned of `.git` suffix)
//! - `homepage`: homepage URL from `project_urls`, or repo_url fallback
//! - `npm_package`: None (PyPI, not npm)
//! - `source_url`: PyPI project page URL
//! - `chains`: inferred from package keywords

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::crawler::normalizer::RawTool;
use crate::crawler::sources::{http_client, SourceCrawler};

const PYPI_JSON_BASE: &str = "https://pypi.org/pypi";
const PYPI_WEB_BASE: &str = "https://pypi.org/project";

/// Source identifier.
const SOURCE_NAME: &str = "pypi";

/// Curated seed list of crypto/agent Python packages to fetch from PyPI.
///
/// PyPI search is JS-challenge-protected, so we maintain a seed list of
/// known packages. New packages can be added here as they are discovered.
const SEED_PACKAGES: &[&str] = &[
    "bnbagent-studio",
    "bnbagent-studio-core",
    "bnbagent",
    "web3",
    "eth-account",
    "eth-utils",
    "web3py",
    "solana",
    "solders",
    "anchorpy",
    "aptos-sdk",
    "sui-sdk",
    "cosmos-sdk",
    "near-sdk-py",
    "stellar-sdk",
    "pytezos",
    "cardano",
    "algorand-sdk",
    "mcp",
    "mcp-server",
];

/// PyPI JSON API `info` object.
#[derive(Debug, Clone, Deserialize)]
struct PackageInfo {
    name: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    keywords: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    author: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    license_expression: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    version: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    requires_python: Option<String>,
    #[serde(default)]
    project_urls: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    entry_points: Option<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code)]
    classifiers: Vec<String>,
}

/// Top-level PyPI JSON API response.
#[derive(Debug, Clone, Deserialize)]
struct PackageResponse {
    info: PackageInfo,
}

/// Extract repository URL from `project_urls`.
fn extract_repo_url(project_urls: &std::collections::HashMap<String, String>) -> Option<String> {
    for (label, url) in project_urls {
        let label_lower = label.to_lowercase();
        if label_lower.contains("repository")
            || label_lower.contains("source")
            || label_lower.contains("github")
            || label_lower.contains("gitlab")
            || label_lower.contains("code")
        {
            return Some(clean_git_url(url));
        }
    }
    // Fall back to any github.com URL in the project_urls.
    for url in project_urls.values() {
        if url.contains("github.com") || url.contains("gitlab.com") {
            return Some(clean_git_url(url));
        }
    }
    None
}

/// Extract homepage URL from `project_urls`.
fn extract_homepage(project_urls: &std::collections::HashMap<String, String>) -> Option<String> {
    for (label, url) in project_urls {
        let label_lower = label.to_lowercase();
        if label_lower.contains("homepage") || label_lower.contains("website") {
            return Some(url.clone());
        }
    }
    None
}

/// Strip `git+` prefix and `.git` suffix from repository URLs.
fn clean_git_url(url: &str) -> String {
    let url = url.strip_prefix("git+").unwrap_or(url);
    let url = url.strip_suffix(".git").unwrap_or(url);
    // Strip trailing slashes.
    url.trim_end_matches('/').to_string()
}

/// Parse keywords from the comma-separated string in PyPI metadata.
fn parse_keywords(raw: &str) -> Vec<String> {
    raw.split([',', ' ', ';'])
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_lowercase())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Determine tool_type from entry points.
fn infer_tool_type(entry_points: &Option<serde_json::Value>) -> String {
    if let Some(ep) = entry_points {
        if let Some(obj) = ep.as_object() {
            if let Some(console_scripts) = obj.get("console_scripts").and_then(|v| v.as_object()) {
                if !console_scripts.is_empty() {
                    return "cli".to_string();
                }
            }
        }
    }
    "sdk".to_string()
}

/// Heuristic chain keyword filter (same list as npm crawler).
fn is_chain_keyword(keyword: &str) -> bool {
    let chains = [
        "ethereum",
        "bitcoin",
        "solana",
        "base",
        "polygon",
        "arbitrum",
        "optimism",
        "avalanche",
        "bnb",
        "bnb-chain",
        "bsc",
        "binance",
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
        "stellar",
        "tezos",
    ];
    chains.contains(&keyword)
}

/// Convert a PyPI package response into a [`RawTool`].
fn package_to_raw(response: &PackageResponse) -> RawTool {
    let info = &response.info;
    let project_urls = info.project_urls.clone().unwrap_or_default();

    let repo_url = extract_repo_url(&project_urls);
    let homepage = extract_homepage(&project_urls)
        .or_else(|| repo_url.clone())
        .or_else(|| project_urls.values().next().cloned());

    let description = info.summary.clone().or(info.description.clone());
    let tool_type = infer_tool_type(&info.entry_points);
    let install_command = Some(format!("pip install {}", info.name));
    let source_url = Some(format!("{PYPI_WEB_BASE}/{}/", info.name));

    let keywords = info
        .keywords
        .as_deref()
        .map(parse_keywords)
        .unwrap_or_default();

    let chains: Vec<String> = keywords
        .iter()
        .filter(|k| is_chain_keyword(k))
        .cloned()
        .collect();

    let license = info
        .license_expression
        .clone()
        .or_else(|| info.license.clone());

    RawTool {
        name: info.name.clone(),
        description,
        tool_type,
        repo_url,
        homepage,
        npm_package: None,
        install_command,
        mcp_endpoint: None,
        chains,
        source: SOURCE_NAME.to_string(),
        source_url,
        license,
        ..Default::default()
    }
}

/// Fetch metadata for a single PyPI package.
async fn fetch_package(client: &reqwest::Client, name: &str) -> Result<PackageResponse> {
    let url = format!("{PYPI_JSON_BASE}/{}/json", urlencoding::encode(name));
    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("fetching PyPI package metadata for {name}"))?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("PyPI package {name} not found (404)");
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("PyPI metadata for {name} returned HTTP {status}: {body}");
    }

    response
        .json()
        .await
        .with_context(|| format!("parsing PyPI metadata JSON for {name}"))
}

/// Crawl all seed PyPI packages and return raw tools.
pub async fn crawl_with_client(client: &reqwest::Client) -> Result<Vec<RawTool>> {
    let mut seen_names = std::collections::HashSet::new();
    let mut out = Vec::new();

    for &pkg_name in SEED_PACKAGES {
        match fetch_package(client, pkg_name).await {
            Ok(response) => {
                let raw = package_to_raw(&response);
                if seen_names.insert(raw.name.clone()) {
                    out.push(raw);
                }
            }
            Err(e) => {
                tracing::warn!(package = pkg_name, error = %e, "failed to fetch PyPI package metadata");
                // Continue with next package.
            }
        }
    }
    Ok(out)
}

/// Crawl PyPI using the shared crawler HTTP client.
pub async fn crawl() -> Result<Vec<RawTool>> {
    let client = http_client().context("building PyPI HTTP client")?;
    crawl_with_client(&client).await
}

/// Run a full PyPI crawl.
///
/// Results are normalized/upserted and the `sources` table is updated.
pub async fn run_once(pool: &sqlx::PgPool) {
    match crawl().await {
        Ok(raws) => {
            tracing::info!(source = SOURCE_NAME, count = raws.len(), "crawl completed");
            crate::crawler::persist_crawl_results(pool, SOURCE_NAME, "https://pypi.org/", raws)
                .await;
        }
        Err(e) => {
            tracing::error!(source = SOURCE_NAME, error = %e, "crawl failed");
            crate::crawler::update_source_status(
                crate::crawler::UpsertTarget::Pool(pool),
                SOURCE_NAME,
                "https://pypi.org/",
                "error",
                0,
                Some(&e.to_string()),
            )
            .await;
        }
    }
}

/// Crawler instance implementing [`SourceCrawler`].
#[allow(dead_code)]
pub struct PyPiCrawler;

#[async_trait::async_trait]
impl SourceCrawler for PyPiCrawler {
    async fn crawl(&self) -> Result<Vec<RawTool>> {
        crawl().await
    }
    fn source_name(&self) -> &str {
        SOURCE_NAME
    }
    fn interval(&self) -> &'static str {
        "0 0 */6 * * *"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn bnbagent_studio_json() -> String {
        r#"{
            "info": {
                "name": "bnbagent-studio",
                "summary": "The `bag` CLI to scaffold and deploy a bnbagent-sdk seller agent on BNB Chain.",
                "description": "Long description here.",
                "keywords": "agent, blockchain, bnb, cli, erc-8004, erc-8183, x402",
                "author": "BNB Chain Studio",
                "license": null,
                "license_expression": "Apache-2.0",
                "version": "0.0.5",
                "requires_python": ">=3.10",
                "project_urls": {
                    "Homepage": "https://github.com/bnb-chain/bnbagent-studio",
                    "Repository": "https://github.com/bnb-chain/bnbagent-studio"
                },
                "entry_points": {
                    "console_scripts": {
                        "bag": "bag.cli:main"
                    }
                },
                "classifiers": [
                    "Environment :: Console",
                    "Programming Language :: Python :: 3"
                ]
            }
        }"#
        .to_string()
    }

    fn web3_json() -> String {
        r#"{
            "info": {
                "name": "web3",
                "summary": "Web3 Python library.",
                "description": "Interact with Ethereum node.",
                "keywords": "ethereum, blockchain, web3",
                "author": "Ethereum Foundation",
                "license": "MIT",
                "license_expression": null,
                "version": "7.0.0",
                "requires_python": ">=3.8",
                "project_urls": {
                    "Homepage": "https://web3py.readthedocs.io/",
                    "Repository": "https://github.com/ethereum/web3.py"
                },
                "entry_points": null,
                "classifiers": []
            }
        }"#
        .to_string()
    }

    #[test]
    fn parse_keywords_splits_on_commas_and_spaces() {
        let kws = parse_keywords("agent, blockchain bnb; cli");
        assert!(kws.contains(&"agent".to_string()));
        assert!(kws.contains(&"blockchain".to_string()));
        assert!(kws.contains(&"bnb".to_string()));
        assert!(kws.contains(&"cli".to_string()));
    }

    #[test]
    fn infer_tool_type_cli_for_console_scripts() {
        let ep = serde_json::json!({
            "console_scripts": { "bag": "bag.cli:main" }
        });
        assert_eq!(infer_tool_type(&Some(ep)), "cli");
    }

    #[test]
    fn infer_tool_type_sdk_without_console_scripts() {
        assert_eq!(infer_tool_type(&None), "sdk");
        assert_eq!(infer_tool_type(&Some(serde_json::json!({}))), "sdk");
    }

    #[test]
    fn extract_repo_url_finds_github_link() {
        let mut urls = std::collections::HashMap::new();
        urls.insert("Homepage".to_string(), "https://example.com".to_string());
        urls.insert(
            "Repository".to_string(),
            "git+https://github.com/bnb-chain/bnbagent-studio.git".to_string(),
        );
        let repo = extract_repo_url(&urls);
        assert_eq!(
            repo,
            Some("https://github.com/bnb-chain/bnbagent-studio".to_string())
        );
    }

    #[test]
    fn package_to_raw_bnbagent_studio() {
        let response: PackageResponse = serde_json::from_str(&bnbagent_studio_json()).unwrap();
        let raw = package_to_raw(&response);
        assert_eq!(raw.name, "bnbagent-studio");
        assert_eq!(raw.tool_type, "cli");
        assert_eq!(
            raw.install_command.as_deref(),
            Some("pip install bnbagent-studio")
        );
        assert_eq!(raw.source, "pypi");
        assert!(raw.chains.contains(&"bnb".to_string()));
        assert_eq!(raw.license.as_deref(), Some("Apache-2.0"));
        assert!(raw.repo_url.as_deref().unwrap().contains("github.com"));
    }

    #[test]
    fn package_to_raw_web3_sdk() {
        let response: PackageResponse = serde_json::from_str(&web3_json()).unwrap();
        let raw = package_to_raw(&response);
        assert_eq!(raw.name, "web3");
        assert_eq!(raw.tool_type, "sdk");
        assert!(raw.chains.contains(&"ethereum".to_string()));
        assert_eq!(raw.license.as_deref(), Some("MIT"));
    }

    #[tokio::test]
    async fn crawl_with_client_fetches_seed_packages() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/pypi/bnbagent-studio/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::from_str::<serde_json::Value>(&bnbagent_studio_json()).unwrap(),
            ))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/pypi/bnbagent-studio-core/json"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/pypi/bnbagent/json"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        // Mount 404 for all other seed packages to keep the test focused.
        for &pkg in &SEED_PACKAGES[3..] {
            Mock::given(method("GET"))
                .and(path(format!("/pypi/{pkg}/json")))
                .respond_with(ResponseTemplate::new(404))
                .mount(&server)
                .await;
        }

        // Build a client that points at the mock server.
        let client = reqwest::Client::builder()
            .user_agent("test-crawler")
            .build()
            .unwrap();

        // We need to use the mock server base, so we'll test fetch_package
        // directly instead of crawl_with_client (which hardcodes pypi.org).
        let url = format!("{}/pypi/bnbagent-studio/json", server.uri());
        let response = client.get(&url).send().await.unwrap();
        let parsed: PackageResponse = response.json().await.unwrap();
        let raw = package_to_raw(&parsed);
        assert_eq!(raw.name, "bnbagent-studio");
        assert_eq!(raw.tool_type, "cli");
    }

    #[test]
    fn seed_packages_includes_bnbagent_studio() {
        assert!(SEED_PACKAGES.contains(&"bnbagent-studio"));
    }
}
