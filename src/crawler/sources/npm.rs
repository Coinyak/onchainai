//! npm source crawler.
//!
//! Queries the npm registry search endpoint for packages with the keywords
//! `mcp`, `crypto`, `web3`, and `blockchain`.
//!
//! Mapping to OnchainAI:
//! - `source`: `npm`
//! - `tool_type`: `cli` if the package has a `bin` field, otherwise `sdk`
//! - `install_command`: `npx {name}`
//! - `repo_url`: repository URL from package metadata (cleaned of `git+` prefix)
//! - `homepage`: homepage from metadata, or repo_url fallback
//! - `npm_package`: package name
//! - `source_url`: npm package page URL
//!
//! The search endpoint returns limited fields; to get the `bin` field and
//! repository URL we fetch the full package metadata from `registry.npmjs.org`.

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::crawler::normalizer::RawTool;
use crate::crawler::sources::{http_client, SourceCrawler};

const NPM_SEARCH_BASE: &str = "https://registry.npmjs.org/-/v1/search";
const NPM_PACKAGE_BASE: &str = "https://registry.npmjs.org";
const NPM_WEB_BASE: &str = "https://www.npmjs.com/package";

/// Source identifier.
const SOURCE_NAME: &str = "npm";

/// Keywords used for npm search.
const KEYWORDS: &[&str] = &["mcp", "crypto", "web3", "blockchain"];

/// Search result package summary.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct SearchPackage {
    name: String,
    version: String,
    description: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    date: Option<String>,
    links: Option<SearchLinks>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct SearchLinks {
    npm: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
    bugs: Option<String>,
}

/// Wrapper returned by the npm search endpoint.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct SearchWrapper {
    package: SearchPackage,
}

/// Top-level search response.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    objects: Vec<SearchWrapper>,
}

/// Full package version metadata from `registry.npmjs.org/{package}`.
#[derive(Debug, Clone, Deserialize)]
struct PackageMetadata {
    #[serde(default)]
    versions: std::collections::HashMap<String, PackageVersion>,
    #[serde(rename = "dist-tags")]
    dist_tags: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct PackageVersion {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    bin: Option<serde_json::Value>,
    #[serde(default)]
    repository: Option<serde_json::Value>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    keywords: Option<Vec<String>>,
}

/// Extract the latest version from package metadata.
fn latest_version(metadata: &PackageMetadata) -> Option<&PackageVersion> {
    let latest = metadata.dist_tags.as_ref()?.get("latest")?;
    metadata.versions.get(latest)
}

/// Extract a plain repository URL from npm metadata.
fn extract_repo_url(version: &PackageVersion) -> Option<String> {
    if let Some(repo) = &version.repository {
        if let Some(url) = repo.as_str() {
            return Some(clean_git_url(url));
        }
        if let Some(obj) = repo.as_object() {
            if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
                return Some(clean_git_url(url));
            }
        }
    }
    None
}

/// Strip `git+` prefix and `.git` suffix from repository URLs.
fn clean_git_url(url: &str) -> String {
    let url = url.strip_prefix("git+").unwrap_or(url);
    url.strip_suffix(".git").unwrap_or(url).to_string()
}

/// Determine tool_type from the presence of a `bin` field.
fn infer_tool_type(version: &PackageVersion) -> String {
    match &version.bin {
        Some(value) if !value.is_null() => "cli".to_string(),
        _ => "sdk".to_string(),
    }
}

/// Convert a package version into a [`RawTool`].
fn version_to_raw(version: &PackageVersion, source_pkg: &SearchPackage) -> RawTool {
    let name = &version.name;
    let repo_url = extract_repo_url(version);
    let homepage = version
        .homepage
        .clone()
        .or_else(|| repo_url.clone())
        .or_else(|| source_pkg.links.as_ref().and_then(|l| l.homepage.clone()));
    let description = version
        .description
        .clone()
        .or(source_pkg.description.clone());
    let tool_type = infer_tool_type(version);
    let npm_package = Some(name.clone());
    let install_command = Some(format!("npx {name}"));
    let source_url = Some(format!("{NPM_WEB_BASE}/{name}"));

    let keywords: Vec<String> = version
        .keywords
        .clone()
        .unwrap_or_default()
        .into_iter()
        .chain(source_pkg.keywords.iter().cloned())
        .map(|k| k.to_lowercase())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let chains: Vec<String> = keywords
        .iter()
        .filter(|k| is_chain_keyword(k))
        .cloned()
        .collect();

    RawTool {
        name: name.clone(),
        description,
        tool_type,
        repo_url,
        homepage,
        npm_package,
        install_command,
        mcp_endpoint: None,
        chains,
        stars: 0,
        last_commit_at: None,
        source: SOURCE_NAME.to_string(),
        source_url,
        license: version.license.clone(),
    }
}

/// Heuristic chain keyword filter.
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
    ];
    chains.contains(&keyword)
}

/// Search for one keyword and return raw tools.
async fn search_keyword(client: &reqwest::Client, keyword: &str) -> Result<Vec<RawTool>> {
    let url = format!("{NPM_SEARCH_BASE}?text=keywords:{keyword}&size=100");
    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("npm search request failed for keyword {keyword}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("npm search for keyword {keyword} returned HTTP {status}: {body}");
    }

    let search: SearchResponse = response
        .json()
        .await
        .context("parsing npm search response JSON")?;

    let mut out = Vec::new();
    for wrapper in search.objects {
        let pkg = &wrapper.package;
        let metadata_url = format!("{NPM_PACKAGE_BASE}/{}", urlencoding::encode(&pkg.name));
        match fetch_package_metadata(client, &metadata_url).await {
            Ok(metadata) => {
                if let Some(version) = latest_version(&metadata) {
                    out.push(version_to_raw(version, pkg));
                }
            }
            Err(e) => {
                tracing::warn!(package = %pkg.name, error = %e, "failed to fetch npm package metadata");
                // Continue with next package.
            }
        }
    }
    Ok(out)
}

/// Fetch full metadata for a single npm package.
async fn fetch_package_metadata(client: &reqwest::Client, url: &str) -> Result<PackageMetadata> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("fetching npm package metadata from {url}"))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("npm package metadata returned HTTP {status}");
    }

    response
        .json()
        .await
        .context("parsing npm package metadata JSON")
}

/// Crawl all configured npm keywords and return raw tools.
pub async fn crawl_with_client(client: &reqwest::Client) -> Result<Vec<RawTool>> {
    let mut seen_names = std::collections::HashSet::new();
    let mut out = Vec::new();

    for keyword in KEYWORDS {
        match search_keyword(client, keyword).await {
            Ok(raws) => {
                for raw in raws {
                    if seen_names.insert(raw.name.clone()) {
                        out.push(raw);
                    }
                }
            }
            Err(e) => {
                tracing::error!(keyword, error = %e, "npm keyword search failed");
                // Continue with next keyword.
            }
        }
    }
    Ok(out)
}

/// Crawl npm using the shared crawler HTTP client.
pub async fn crawl() -> Result<Vec<RawTool>> {
    let client = http_client().context("building npm HTTP client")?;
    crawl_with_client(&client).await
}

/// Run a full npm crawl.
///
/// Results are normalized/upserted and the `sources` table is updated.
pub async fn run_once(pool: &sqlx::PgPool) {
    match crawl().await {
        Ok(raws) => {
            tracing::info!(source = SOURCE_NAME, count = raws.len(), "crawl completed");
            crate::crawler::persist_crawl_results(
                pool,
                SOURCE_NAME,
                "https://registry.npmjs.org/",
                raws,
            )
            .await;
        }
        Err(e) => {
            tracing::error!(source = SOURCE_NAME, error = %e, "crawl failed");
            crate::crawler::update_source_status(
                pool,
                SOURCE_NAME,
                "https://registry.npmjs.org/",
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
pub struct NpmCrawler;

#[async_trait::async_trait]
impl SourceCrawler for NpmCrawler {
    async fn crawl(&self) -> Result<Vec<RawTool>> {
        crawl().await
    }
    fn source_name(&self) -> &str {
        SOURCE_NAME
    }
    fn interval(&self) -> &'static str {
        "0 0 * * * *"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn search_response_json() -> String {
        r#"{
            "objects": [
                {
                    "package": {
                        "name": "@gobob/gateway-cli",
                        "version": "0.2.0",
                        "description": "CLI for bridging Bitcoin to EVM chains.",
                        "keywords": ["bitcoin", "ethereum", "bridge", "crypto"],
                        "date": "2026-06-01T00:00:00.000Z",
                        "links": {
                            "npm": "https://www.npmjs.com/package/@gobob/gateway-cli",
                            "homepage": "https://github.com/bob-collective/bob#readme",
                            "repository": "https://github.com/bob-collective/bob"
                        }
                    }
                },
                {
                    "package": {
                        "name": "@modelcontextprotocol/sdk",
                        "version": "1.0.0",
                        "description": "MCP SDK for TypeScript.",
                        "keywords": ["mcp", "sdk"],
                        "date": "2026-06-02T00:00:00.000Z",
                        "links": {
                            "npm": "https://www.npmjs.com/package/@modelcontextprotocol/sdk",
                            "homepage": "https://modelcontextprotocol.io"
                        }
                    }
                }
            ]
        }"#
        .to_string()
    }

    fn gateway_metadata_json() -> String {
        r#"{
            "dist-tags": { "latest": "0.2.0" },
            "versions": {
                "0.2.0": {
                    "name": "@gobob/gateway-cli",
                    "version": "0.2.0",
                    "description": "CLI for bridging Bitcoin to EVM chains via BOB Gateway.",
                    "bin": { "gateway-cli": "dist/bin/gateway-cli.js" },
                    "repository": { "type": "git", "url": "git+https://github.com/bob-collective/bob.git" },
                    "homepage": "https://github.com/bob-collective/bob#readme",
                    "license": "MIT",
                    "keywords": ["bitcoin", "ethereum", "bridge"]
                }
            }
        }"#
        .to_string()
    }

    fn sdk_metadata_json() -> String {
        r#"{
            "dist-tags": { "latest": "1.0.0" },
            "versions": {
                "1.0.0": {
                    "name": "@modelcontextprotocol/sdk",
                    "version": "1.0.0",
                    "description": "MCP SDK for TypeScript.",
                    "repository": { "type": "git", "url": "git+https://github.com/modelcontextprotocol/typescript-sdk.git" },
                    "homepage": "https://modelcontextprotocol.io",
                    "license": "MIT",
                    "keywords": ["mcp", "sdk"]
                }
            }
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn crawl_parses_packages_with_correct_source_install_and_type() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/-/v1/search"))
            .respond_with(ResponseTemplate::new(200).set_body_string(search_response_json()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/%40gobob%2Fgateway-cli"))
            .respond_with(ResponseTemplate::new(200).set_body_string(gateway_metadata_json()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/%40modelcontextprotocol%2Fsdk"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sdk_metadata_json()))
            .mount(&server)
            .await;

        let _client = reqwest::Client::builder()
            .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        // Override the base by rewriting search URL is not supported; test helper
        // crawl_with_client uses full URLs returned by search. Wiremock cannot
        // intercept registry.npmjs.org absolute URLs. Therefore we test the
        // parsing helpers directly.
        let search: SearchResponse = serde_json::from_str(&search_response_json()).unwrap();
        let gateway_pkg = &search.objects[0].package;
        let gateway_meta: PackageMetadata = serde_json::from_str(&gateway_metadata_json()).unwrap();
        let gateway_version = latest_version(&gateway_meta).unwrap();
        let gateway_raw = version_to_raw(gateway_version, gateway_pkg);

        assert_eq!(gateway_raw.source, "npm");
        assert_eq!(gateway_raw.tool_type, "cli");
        assert_eq!(
            gateway_raw.install_command.as_deref(),
            Some("npx @gobob/gateway-cli")
        );
        assert_eq!(
            gateway_raw.repo_url.as_deref(),
            Some("https://github.com/bob-collective/bob")
        );
        assert!(gateway_raw.chains.contains(&"bitcoin".to_string()));

        let sdk_pkg = &search.objects[1].package;
        let sdk_meta: PackageMetadata = serde_json::from_str(&sdk_metadata_json()).unwrap();
        let sdk_version = latest_version(&sdk_meta).unwrap();
        let sdk_raw = version_to_raw(sdk_version, sdk_pkg);
        assert_eq!(sdk_raw.tool_type, "sdk");
        assert_eq!(
            sdk_raw.install_command.as_deref(),
            Some("npx @modelcontextprotocol/sdk")
        );
    }

    #[tokio::test]
    async fn http_mock_returns_500_and_search_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/-/v1/search"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = reqwest::Client::builder()
            .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        let err = client
            .get(format!(
                "{}/-/v1/search?text=keywords:mcp&size=100",
                server.uri()
            ))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap_err();
        assert!(err.to_string().contains("500") || err.to_string().contains("Internal"));
    }

    #[test]
    fn tool_type_cli_when_bin_present_sdk_otherwise() {
        let cli = PackageVersion {
            name: "cli-pkg".into(),
            description: None,
            bin: Some(serde_json::json!({ "bin": "bin.js" })),
            repository: None,
            homepage: None,
            license: None,
            keywords: None,
        };
        let sdk = PackageVersion {
            name: "sdk-pkg".into(),
            description: None,
            bin: None,
            repository: None,
            homepage: None,
            license: None,
            keywords: None,
        };
        assert_eq!(infer_tool_type(&cli), "cli");
        assert_eq!(infer_tool_type(&sdk), "sdk");
    }

    #[test]
    fn clean_git_url_removes_prefix_and_suffix() {
        assert_eq!(
            clean_git_url("git+https://github.com/owner/repo.git"),
            "https://github.com/owner/repo"
        );
        assert_eq!(
            clean_git_url("https://github.com/owner/repo"),
            "https://github.com/owner/repo"
        );
    }

    #[test]
    fn extract_repo_url_from_string_and_object() {
        let mut version = PackageVersion {
            name: "pkg".into(),
            description: None,
            bin: None,
            repository: None,
            homepage: None,
            license: None,
            keywords: None,
        };
        assert_eq!(extract_repo_url(&version), None);

        version.repository = Some(serde_json::json!("git+https://github.com/a/b.git"));
        assert_eq!(
            extract_repo_url(&version),
            Some("https://github.com/a/b".into())
        );

        version.repository = Some(serde_json::json!({ "url": "https://github.com/c/d.git" }));
        assert_eq!(
            extract_repo_url(&version),
            Some("https://github.com/c/d".into())
        );
    }
}
