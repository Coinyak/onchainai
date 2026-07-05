//! Official MCP Registry source crawler.
//!
//! The registry endpoint returns server documents at
//! `https://registry.modelcontextprotocol.io/v0/servers`. OnchainAI stores
//! each entry as an MCP tool and lets the existing relevance, safety, and
//! dedupe pipeline decide whether it is crypto-relevant.

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::crawler::normalizer::RawTool;
use crate::crawler::sources::{http_client, SourceCrawler};

const MCP_REGISTRY_URL: &str = "https://registry.modelcontextprotocol.io/v0/servers";
const SOURCE_NAME: &str = "mcp-registry";

#[derive(Debug, Clone, Deserialize)]
struct RegistryResponse {
    #[serde(default)]
    servers: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryEntry {
    server: RegistryServer,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryServer {
    name: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    repository: Option<Repository>,
    #[serde(default)]
    remotes: Vec<Remote>,
    #[serde(default)]
    packages: Vec<Package>,
}

#[derive(Debug, Clone, Deserialize)]
struct Repository {
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Remote {
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Package {
    #[serde(default)]
    registry: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
}

fn registry_server_to_raw(server: &RegistryServer) -> RawTool {
    let name = server.title.clone().unwrap_or_else(|| server.name.clone());
    let repo_url = server.repository.as_ref().map(|repo| repo.url.clone());
    let mcp_endpoint = server.remotes.first().map(|remote| remote.url.clone());
    let npm_package = npm_package(server);
    let install_command = selected_install_package(server).and_then(package_install_command);

    RawTool {
        name,
        description: server.description.clone(),
        tool_type: "mcp".to_string(),
        repo_url: repo_url.clone(),
        homepage: repo_url.clone().or_else(|| mcp_endpoint.clone()),
        npm_package,
        install_command,
        mcp_endpoint,
        chains: infer_chains(server),
        source: SOURCE_NAME.to_string(),
        source_url: repo_url.or_else(|| Some(MCP_REGISTRY_URL.to_string())),
        ..Default::default()
    }
}

fn npm_package(server: &RegistryServer) -> Option<String> {
    server
        .packages
        .iter()
        .find(|package| package.registry.as_deref() == Some("npm"))
        .and_then(|package| package.name.clone())
}

fn selected_install_package(server: &RegistryServer) -> Option<&Package> {
    server
        .packages
        .iter()
        .find(|package| package.registry.as_deref() == Some("npm"))
        .or_else(|| {
            server
                .packages
                .iter()
                .find(|package| package.command.is_some())
        })
}

fn package_install_command(package: &Package) -> Option<String> {
    if let Some(command) = &package.command {
        let mut parts = vec![command.clone()];
        parts.extend(package.args.iter().cloned());
        return Some(parts.join(" "));
    }
    if package.registry.as_deref() == Some("npm") {
        return package.name.as_ref().map(|name| format!("npx {name}"));
    }
    None
}

fn infer_chains(server: &RegistryServer) -> Vec<String> {
    let text = format!(
        "{} {} {}",
        server.name,
        server.title.as_deref().unwrap_or_default(),
        server.description.as_deref().unwrap_or_default()
    )
    .to_lowercase();

    [
        ("ethereum", "ethereum"),
        ("solana", "solana"),
        ("bitcoin", "bitcoin"),
        ("base", "base"),
        ("polygon", "polygon"),
        ("arbitrum", "arbitrum"),
        ("optimism", "optimism"),
        ("avalanche", "avalanche"),
        ("bnb", "bnb"),
    ]
    .iter()
    .filter(|(needle, _)| text.contains(needle))
    .map(|(_, chain)| (*chain).to_string())
    .collect()
}

pub async fn crawl_url(url: &str) -> Result<Vec<RawTool>> {
    let client = http_client().context("building MCP Registry HTTP client")?;
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("fetching MCP Registry from {url}"))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("MCP Registry returned HTTP {status}");
    }

    let registry: RegistryResponse = response.json().await.context("parsing MCP Registry JSON")?;
    Ok(registry
        .servers
        .iter()
        .map(|entry| registry_server_to_raw(&entry.server))
        .collect())
}

pub async fn crawl() -> Result<Vec<RawTool>> {
    crawl_url(MCP_REGISTRY_URL).await
}

pub async fn run_once(pool: &sqlx::PgPool) {
    match crawl().await {
        Ok(raws) => {
            tracing::info!(source = SOURCE_NAME, count = raws.len(), "crawl completed");
            crate::crawler::persist_crawl_results(pool, SOURCE_NAME, MCP_REGISTRY_URL, raws).await;
        }
        Err(e) => {
            tracing::error!(source = SOURCE_NAME, error = %e, "crawl failed");
            crate::crawler::update_source_status(
                crate::crawler::UpsertTarget::Pool(pool),
                SOURCE_NAME,
                MCP_REGISTRY_URL,
                "error",
                0,
                Some(&e.to_string()),
            )
            .await;
        }
    }
}

pub struct OfficialMcpRegistryCrawler;

#[async_trait::async_trait]
impl SourceCrawler for OfficialMcpRegistryCrawler {
    async fn crawl(&self) -> Result<Vec<RawTool>> {
        crawl().await
    }

    fn source_name(&self) -> &str {
        SOURCE_NAME
    }

    fn interval(&self) -> &'static str {
        "0 15 */12 * * *"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_registry_json() -> &'static str {
        r#"{
            "servers": [
                {
                    "server": {
                        "name": "example/solana-mcp",
                        "title": "Solana MCP",
                        "description": "Solana wallet, token, and DeFi data MCP server.",
                        "repository": { "url": "https://github.com/example/solana-mcp", "source": "github" },
                        "remotes": [{ "type": "streamable-http", "url": "https://solana.example/mcp" }],
                        "packages": [
                            { "registry": "npm", "name": "@example/solana-mcp", "command": "npx", "args": ["@example/solana-mcp"] }
                        ]
                    }
                },
                {
                    "server": {
                        "name": "example/base-mcp",
                        "description": "Base payments MCP server.",
                        "packages": [
                            { "registry": "docker", "name": "example/base-mcp", "command": "docker", "args": ["run", "example/base-mcp"] },
                            { "registry": "npm", "name": "@example/base-mcp", "command": "npx", "args": ["@example/base-mcp"] }
                        ]
                    }
                }
            ]
        }"#
    }

    #[tokio::test]
    async fn crawl_parses_registry_servers() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v0/servers"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sample_registry_json(), "application/json"),
            )
            .mount(&server)
            .await;

        let raws = crawl_url(&format!("{}/v0/servers", server.uri()))
            .await
            .expect("registry crawl should parse");

        assert_eq!(raws.len(), 2);
        assert_eq!(raws[0].name, "Solana MCP");
        assert_eq!(raws[0].tool_type, "mcp");
        assert_eq!(raws[0].source, SOURCE_NAME);
        assert_eq!(
            raws[0].mcp_endpoint.as_deref(),
            Some("https://solana.example/mcp")
        );
        assert_eq!(raws[0].npm_package.as_deref(), Some("@example/solana-mcp"));
        assert!(raws[0].chains.contains(&"solana".to_string()));
        assert_eq!(raws[1].npm_package.as_deref(), Some("@example/base-mcp"));
        assert_eq!(
            raws[1].install_command.as_deref(),
            Some("npx @example/base-mcp")
        );
    }
}
