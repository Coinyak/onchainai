//! web3-mcp-hub source crawler.
//!
//! The web3-mcp-hub registry is published at
//! `https://raw.githubusercontent.com/rudazy/web3-mcp-hub/main/registry.json`.
//! It contains a `servers` array where each server has metadata including
//! `id`, `name`, `description`, `category`, `repository`, `networks`,
//! `verified`, `installationType`, and `config` (command/args or url).
//!
//! Mapping to OnchainAI:
//! - `source`: `web3-mcp-hub`
//! - `tool_type`: `mcp`
//! - `repo_url`: `repository`
//! - `homepage`: `repository` (when no dedicated homepage)
//! - `mcp_endpoint`: remote `url` when `installationType` is `remote`
//! - `install_command`: derived from `config` when available
//! - `chains`: `networks` mapped to lowercase
//! - `source_url`: `registryUrl` if provided, else the registry URL

use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::crawler::normalizer::RawTool;
use crate::crawler::sources::{http_client, SourceCrawler};

const WEB3MCP_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/rudazy/web3-mcp-hub/main/registry.json";

/// Source identifier.
const SOURCE_NAME: &str = "web3-mcp-hub";

/// Server installation configuration inside the registry.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ServerConfig {
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    url: Option<String>,
}

/// A single server entry in the web3-mcp-hub registry.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ServerEntry {
    id: String,
    name: String,
    description: String,
    category: String,
    repository: String,
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    networks: Vec<String>,
    #[serde(default)]
    verified: bool,
    #[serde(default)]
    installation_type: String,
    #[serde(default, rename = "installationType")]
    installation_type_alt: String,
    #[serde(default)]
    config: Option<ServerConfig>,
    #[serde(default)]
    registry_url: Option<String>,
    #[serde(default, rename = "registryUrl")]
    registry_url_alt: Option<String>,
}

/// Top-level registry response.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct RegistryResponse {
    #[serde(default)]
    servers: Vec<ServerEntry>,
}

/// Derive an install command from a server config.
fn derive_install_command(
    config: &Option<ServerConfig>,
    installation_type: &str,
) -> Option<String> {
    if let Some(cfg) = config {
        if let Some(url) = &cfg.url {
            return crate::public_install_guide::http_mcp_universal_install_command(url);
        }
        if let Some(command) = &cfg.command {
            let mut parts = vec![command.clone()];
            parts.extend(cfg.args.iter().cloned());
            return Some(parts.join(" "));
        }
    }
    if installation_type.eq_ignore_ascii_case("git") {
        // Fallback generic git clone instruction; better than nothing.
        return Some("git clone <repository> && npm install && npm start".to_string());
    }
    None
}

/// Map a web3-mcp-hub category to an OnchainAI function id.
#[allow(dead_code)]
fn map_category_to_function(category: &str) -> &'static str {
    match category {
        "identity-reputation" => "identity",
        "multi-chain" | "evm-networks" | "solana-ecosystem" | "bitcoin-lightning" | "layer-2"
        | "non-evm" => "dev-tool",
        "defi" => "swap",
        "nft-digital-assets" => "nft",
        "analytics-data" | "market-data" => "data",
        "prediction-markets" => "trading",
        "developer-tools" => "dev-tool",
        _ => "dev-tool",
    }
}

/// Normalize a registry server entry into a [`RawTool`].
fn server_to_raw(server: &ServerEntry) -> RawTool {
    let installation_type = if server.installation_type.is_empty() {
        &server.installation_type_alt
    } else {
        &server.installation_type
    };

    let chains: Vec<String> = server
        .networks
        .iter()
        .map(|n| n.to_lowercase())
        .filter(|n| !n.is_empty())
        .collect();

    let mcp_endpoint = server
        .config
        .as_ref()
        .and_then(|c| c.url.clone())
        .or_else(|| {
            if installation_type.eq_ignore_ascii_case("remote") {
                Some(server.repository.clone())
            } else {
                None
            }
        });

    let install_command = derive_install_command(&server.config, installation_type);
    let source_url = server
        .registry_url
        .clone()
        .or(server.registry_url_alt.clone())
        .or_else(|| Some(server.repository.clone()));

    RawTool {
        name: server.name.clone(),
        description: Some(server.description.clone()),
        tool_type: "mcp".to_string(),
        repo_url: Some(server.repository.clone()),
        homepage: Some(server.repository.clone()),
        install_command,
        mcp_endpoint,
        chains,
        source: SOURCE_NAME.to_string(),
        source_url,
        ..Default::default()
    }
}

/// Fetch and parse the web3-mcp-hub registry from `url`.
pub async fn crawl_url(url: &str) -> Result<Vec<RawTool>> {
    let client = http_client().context("building web3-mcp-hub HTTP client")?;
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("fetching web3-mcp-hub registry from {url}"))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("web3-mcp-hub registry returned HTTP {status}");
    }

    let registry: RegistryResponse = response
        .json()
        .await
        .context("parsing web3-mcp-hub registry JSON")?;

    let mut out = Vec::with_capacity(registry.servers.len());
    for server in &registry.servers {
        out.push(server_to_raw(server));
    }
    Ok(out)
}

/// Fetch and parse the production web3-mcp-hub registry.
pub async fn crawl() -> Result<Vec<RawTool>> {
    crawl_url(WEB3MCP_REGISTRY_URL).await
}

/// Run a full crawl.
///
/// Results are normalized/upserted and the `sources` table is updated.
pub async fn run_once(pool: &sqlx::PgPool) {
    match crawl().await {
        Ok(raws) => {
            tracing::info!(source = SOURCE_NAME, count = raws.len(), "crawl completed");
            crate::crawler::persist_crawl_results(pool, SOURCE_NAME, WEB3MCP_REGISTRY_URL, raws)
                .await;
        }
        Err(e) => {
            tracing::error!(source = SOURCE_NAME, error = %e, "crawl failed");
            crate::crawler::update_source_status(
                crate::crawler::UpsertTarget::Pool(pool),
                SOURCE_NAME,
                WEB3MCP_REGISTRY_URL,
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
pub struct Web3McpHubCrawler;

#[async_trait::async_trait]
impl SourceCrawler for Web3McpHubCrawler {
    async fn crawl(&self) -> Result<Vec<RawTool>> {
        crawl().await
    }
    fn source_name(&self) -> &str {
        SOURCE_NAME
    }
    fn interval(&self) -> &'static str {
        "0 0 */12 * * *"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_registry_json() -> String {
        r#"{
            "name": "Web3 MCP Hub Registry",
            "version": "3.0.0",
            "description": "Definitive MCP server registry for Web3.",
            "stats": { "totalServers": 2, "verifiedServers": 1 },
            "categories": { "defi": "DeFi protocols" },
            "servers": [
                {
                    "id": "intuition-mcp",
                    "name": "Intuition MCP",
                    "description": "Query atoms and triples from Intuition's decentralized knowledge graph.",
                    "category": "identity-reputation",
                    "repository": "https://github.com/0xIntuition/intuition-mcp-server",
                    "features": ["attestations", "trust-scores"],
                    "networks": ["base"],
                    "verified": true,
                    "installationType": "git",
                    "config": { "command": "node", "args": ["dist/index.js"] }
                },
                {
                    "id": "coingecko-trader",
                    "name": "CoinGecko Crypto Trader",
                    "description": "Real-time crypto market data.",
                    "category": "market-data",
                    "repository": "https://github.com/saintdoresh/crypto-trader-mcp-claudedesktop",
                    "features": ["prices"],
                    "networks": ["ethereum", "base"],
                    "verified": false,
                    "installationType": "remote",
                    "config": { "url": "https://mcp.coingecko.trader/sse" }
                }
            ]
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn crawl_parses_servers_with_correct_source_and_type() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/registry.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sample_registry_json()))
            .mount(&server)
            .await;

        let url = format!("{}/registry.json", server.uri());
        let raws = crawl_url(&url).await.expect("crawl should succeed");

        assert_eq!(raws.len(), 2);
        let intuition = raws
            .iter()
            .find(|r| r.name == "Intuition MCP")
            .expect("Intuition MCP present");
        assert_eq!(intuition.source, "web3-mcp-hub");
        assert_eq!(intuition.tool_type, "mcp");
        assert_eq!(
            intuition.repo_url.as_deref(),
            Some("https://github.com/0xIntuition/intuition-mcp-server")
        );
        assert!(intuition.chains.contains(&"base".to_string()));

        let remote = raws
            .iter()
            .find(|r| r.name == "CoinGecko Crypto Trader")
            .expect("CoinGecko present");
        assert_eq!(remote.tool_type, "mcp");
        assert_eq!(
            remote.mcp_endpoint.as_deref(),
            Some("https://mcp.coingecko.trader/sse")
        );
    }

    #[tokio::test]
    async fn http_mock_returns_500_and_crawl_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/registry.json"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let url = format!("{}/registry.json", server.uri());
        let err = crawl_url(&url).await.unwrap_err();
        assert!(err.to_string().contains("500"));
    }

    #[test]
    fn category_mapping_covers_registry_categories() {
        let cases = vec![
            ("identity-reputation", "identity"),
            ("multi-chain", "dev-tool"),
            ("evm-networks", "dev-tool"),
            ("solana-ecosystem", "dev-tool"),
            ("bitcoin-lightning", "dev-tool"),
            ("layer-2", "dev-tool"),
            ("non-evm", "dev-tool"),
            ("defi", "swap"),
            ("nft-digital-assets", "nft"),
            ("analytics-data", "data"),
            ("market-data", "data"),
            ("prediction-markets", "trading"),
            ("developer-tools", "dev-tool"),
            ("unknown", "dev-tool"),
        ];
        for (cat, expected) in cases {
            assert_eq!(map_category_to_function(cat), expected, "category {cat}");
        }
    }

    #[test]
    fn install_command_derived_from_config() {
        let cfg = ServerConfig {
            command: Some("npx".into()),
            args: vec!["@org/server".into()],
            env: HashMap::new(),
            url: None,
        };
        assert_eq!(
            derive_install_command(&Some(cfg), "git"),
            Some("npx @org/server".into())
        );

        let cfg_remote = ServerConfig {
            command: None,
            args: vec![],
            env: HashMap::new(),
            url: Some("https://mcp.example/sse".into()),
        };
        assert_eq!(
            derive_install_command(&Some(cfg_remote), "remote"),
            Some("npx add-mcp https://mcp.example/sse".into())
        );
    }
}
