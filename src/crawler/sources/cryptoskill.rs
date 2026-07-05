//! CryptoSkill source crawler.
//!
//! CryptoSkill publishes its full registry at `https://cryptoskill.org/skills.json`.
//! Each skill entry has a unique `name` (slug-like), `displayName`,
//! `description`, `category`, `tags`, `author`, `version`, and dates. The
//! skill name doubles as the install slug: `clawhub install {name}`.
//!
//! Mapping to OnchainAI:
//! - `source`: `cryptoskill`
//! - `tool_type`: `skill`
//! - `install_command`: `clawhub install {name}`
//! - `source_url`: `https://cryptoskill.org/skills/{category}/{name}.html`
//! - `function`: CryptoSkill category → OnchainAI function keyword corpus.
//!
//! GitHub URL is not present in the JSON registry, so `repo_url` is left
//! blank; classification still uses the name + description + tags.

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::crawler::normalizer::RawTool;
use crate::crawler::sources::{http_client, SourceCrawler};

const CRYPTOSKILL_REGISTRY_URL: &str = "https://cryptoskill.org/skills.json";
const CRYPTOSKILL_DETAIL_BASE: &str = "https://cryptoskill.org/skills";

/// Source identifier.
const SOURCE_NAME: &str = "cryptoskill";

// Suppress dead-code warnings for helpers only used in tests until the
// normalizer consumes them in a later milestone.
#[allow(dead_code)]
const _CRYPTOSKILL_REGISTRY_URL: &str = CRYPTOSKILL_REGISTRY_URL;

/// CryptoSkill skill entry as returned by `skills.json`.
///
/// Only fields needed for normalization are read after deserialization; the
/// rest are kept for completeness but may be unused.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct SkillEntry {
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
    description: String,
    category: String,
    tags: Vec<String>,
    author: String,
    version: String,
    added_at: String,
    last_updated: String,
}

/// Top-level registry response.
#[derive(Debug, Clone, Deserialize)]
struct RegistryResponse {
    skills: Vec<SkillEntry>,
}

/// Map a CryptoSkill category id to a keyword corpus used for OnchainAI
/// 3-axis classification. Categories that are close to OnchainAI functions
/// are mapped to that function's keywords; the rest use their own text so
/// classification can still pick them up.
#[allow(dead_code)]
fn category_corpus(category: &str, name: &str, description: &str, tags: &[String]) -> String {
    let tags_text = tags.join(" ");
    let base = format!("{name} {description} {category} {tags_text}");
    let extra = match category {
        "exchanges" => " exchange cex dex trading",
        "dev-tools" => " dev-tool sdk rpc hardhat foundry compiler debug",
        "ai-crypto" => " ai agent autonomous eliza",
        "mcp-servers" => " mcp server mcp-server",
        "prediction-markets" => " prediction market trading",
        _ => "",
    };
    format!("{base}{extra}")
}

#[allow(dead_code)]
fn map_category_to_function(category: &str) -> &'static str {
    match category {
        "exchanges" => "swap",
        "dex" => "swap",
        "chains" => "dev-tool",
        "defi" => "swap",
        "wallets" => "wallet",
        "analytics" => "data",
        "dev-tools" => "dev-tool",
        "trading" => "trading",
        "prediction-markets" => "trading",
        "payments" => "payments",
        "social" => "social",
        "ai-crypto" => "ai-agent",
        "identity" => "identity",
        "mcp-servers" => "dev-tool",
        _ => "dev-tool",
    }
}

/// Normalize a CryptoSkill entry into a [`RawTool`].
fn skill_to_raw(skill: &SkillEntry) -> RawTool {
    let source_url = format!(
        "{CRYPTOSKILL_DETAIL_BASE}/{}/{name}.html",
        skill.category,
        name = skill.name
    );

    RawTool {
        name: skill.display_name.clone(),
        description: Some(skill.description.clone()),
        tool_type: "skill".to_string(),
        install_command: Some(format!("clawhub install {}", skill.name)),
        source: SOURCE_NAME.to_string(),
        source_url: Some(source_url),
        ..Default::default()
    }
}

/// Fetch and parse the CryptoSkill registry from `base_url/skills.json`.
pub async fn crawl_with_base(base_url: &str) -> Result<Vec<RawTool>> {
    let url = format!("{base_url}/skills.json");
    let client = http_client().context("building CryptoSkill HTTP client")?;
    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("fetching CryptoSkill registry from {url}"))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("CryptoSkill registry returned HTTP {status}");
    }

    let registry: RegistryResponse = response
        .json()
        .await
        .context("parsing CryptoSkill registry JSON")?;

    let mut out = Vec::with_capacity(registry.skills.len());
    for skill in &registry.skills {
        out.push(skill_to_raw(skill));
    }
    Ok(out)
}

/// Fetch and parse the CryptoSkill registry from the production URL.
pub async fn crawl() -> Result<Vec<RawTool>> {
    crawl_with_base("https://cryptoskill.org").await
}

/// Run a full crawl.
///
/// Results are normalized/upserted and the `sources` table is updated.
pub async fn run_once(pool: &sqlx::PgPool) {
    match crawl().await {
        Ok(raws) => {
            tracing::info!(source = SOURCE_NAME, count = raws.len(), "crawl completed");
            crate::crawler::persist_crawl_results(
                pool,
                SOURCE_NAME,
                "https://cryptoskill.org/skills.json",
                raws,
            )
            .await;
        }
        Err(e) => {
            tracing::error!(source = SOURCE_NAME, error = %e, "crawl failed");
            crate::crawler::update_source_status(
                crate::crawler::UpsertTarget::Pool(pool),
                SOURCE_NAME,
                "https://cryptoskill.org/skills.json",
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
pub struct CryptoSkillCrawler;

#[async_trait::async_trait]
impl SourceCrawler for CryptoSkillCrawler {
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
    use crate::crawler::normalizer::{classify_actor, classify_asset_class, classify_function};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_registry_json() -> String {
        r#"{
            "skills": [
                {
                    "name": "binance-spot-api",
                    "displayName": "Binance Spot API",
                    "description": "Binance spot trading API skill for AI agents.",
                    "category": "exchanges",
                    "tags": ["official", "exchange"],
                    "author": "binance",
                    "version": "1.0.0",
                    "added_at": "2026-01-01",
                    "last_updated": "2026-06-01"
                },
                {
                    "name": "bittensor-sdk",
                    "displayName": "Bittensor SDK",
                    "description": "Bittensor subnet operations and staking.",
                    "category": "dev-tools",
                    "tags": ["bittensor", "tao", "subnet"],
                    "author": "taoleeh",
                    "version": "1.0.2",
                    "added_at": "2026-01-02",
                    "last_updated": "2026-06-02"
                },
                {
                    "name": "ack-reputation",
                    "displayName": "ACK (Agent Consensus Kudos)",
                    "description": "Peer-driven onchain reputation layer for AI agents.",
                    "category": "ai-crypto",
                    "tags": ["official", "ai-crypto"],
                    "author": "discovered",
                    "version": "1.0.0",
                    "added_at": "2026-01-03",
                    "last_updated": "2026-06-03"
                }
            ],
            "categories": {
                "exchanges": { "name": "Exchanges", "icon": "🏦", "description": "CEX & DEX integrations" }
            }
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn crawl_parses_skills_with_correct_source_and_type() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/skills.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sample_registry_json()))
            .mount(&server)
            .await;

        let raws = crawl_with_base(&server.uri())
            .await
            .expect("crawl should succeed");

        assert_eq!(raws.len(), 3);
        let binance = raws.iter().find(|r| r.name == "Binance Spot API").unwrap();
        assert_eq!(binance.source, "cryptoskill");
        assert_eq!(binance.tool_type, "skill");
        assert_eq!(
            binance.install_command.as_deref(),
            Some("clawhub install binance-spot-api")
        );
        assert!(binance
            .source_url
            .as_deref()
            .unwrap()
            .contains("binance-spot-api"));

        let bittensor = raws.iter().find(|r| r.name == "Bittensor SDK").unwrap();
        assert_eq!(bittensor.tool_type, "skill");

        let ack = raws
            .iter()
            .find(|r| r.name == "ACK (Agent Consensus Kudos)")
            .unwrap();
        assert_eq!(ack.tool_type, "skill");
    }

    #[tokio::test]
    async fn http_mock_returns_500_and_crawl_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/skills.json"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let err = crawl_with_base(&server.uri()).await.unwrap_err();
        assert!(err.to_string().contains("500"));
    }

    #[test]
    fn category_mapping_covers_all_cryptoskill_categories() {
        // Known CryptoSkill categories from the live registry (14 categories).
        let cases = vec![
            ("exchanges", "swap"),
            ("dex", "swap"),
            ("chains", "dev-tool"),
            ("defi", "swap"),
            ("wallets", "wallet"),
            ("analytics", "data"),
            ("dev-tools", "dev-tool"),
            ("trading", "trading"),
            ("prediction-markets", "trading"),
            ("payments", "payments"),
            ("social", "social"),
            ("ai-crypto", "ai-agent"),
            ("identity", "identity"),
            ("mcp-servers", "dev-tool"),
        ];
        for (cat, expected) in cases {
            assert_eq!(map_category_to_function(cat), expected, "category {cat}");
        }
    }

    #[test]
    fn corpus_enables_classification() {
        let c = category_corpus(
            "ai-crypto",
            "ack-reputation",
            "Peer-driven onchain reputation layer for AI agents.",
            &["official".into(), "ai-crypto".into()],
        );
        assert_eq!(classify_function(&c), "ai-agent");
        assert_eq!(classify_actor(&c), "ai-agent");
        assert_eq!(classify_asset_class(&c), "crypto");

        let c = category_corpus(
            "payments",
            "moonpay-usdc-checkout",
            "USDC checkout and payment widget.",
            &["official".into()],
        );
        assert_eq!(classify_function(&c), "payments");
    }
}
