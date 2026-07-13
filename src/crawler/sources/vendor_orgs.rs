//! Vendor-org GitHub crawler — curated first-party org repo sweep (PR-4).
//!
//! Loads `crawl: true` entries from [`crate::vendor_orgs::vendor_orgs_manifest`],
//! fetches `GET /orgs/{org}/repos?per_page=100&type=public&sort=pushed`, applies
//! §4.6 filters, renames short/generic repo slugs, skips existing `repo_url`
//! rows, and persists via [`crate::crawler::persist_crawl_results_gated`].

use std::collections::HashSet;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::crawler::normalizer::RawTool;
use crate::crawler::sources::github::{github_client, parse_datetime};
use crate::crawler::sources::SourceCrawler;
use crate::vendor_orgs::vendor_orgs_manifest;

const GITHUB_API_BASE: &str = "https://api.github.com";
const SOURCE_NAME: &str = "vendor_orgs";
const VENDOR_ORGS_REGISTRY_URL: &str = "https://api.github.com/orgs";
const MIN_STARS: i32 = 10;
const MAX_REPOS_PER_ORG: usize = 15;
const RECENCY_DAYS: i64 = 12 * 30;
const MAX_ORG_REPO_PAGES: u32 = 3;
const MAX_FAILURE_ORGS_IN_MSG: usize = 8;

const AGENT_TOPICS: &[&str] = &["mcp-server", "crypto-mcp", "web3-mcp", "blockchain-mcp"];

/// Result of a vendor-org sweep; may include partial per-org fetch failures.
pub struct VendorOrgsCrawlOutcome {
    pub raws: Vec<RawTool>,
    pub failed_orgs: Vec<String>,
}

fn format_vendor_org_failures(failed_orgs: &[String]) -> String {
    let shown: Vec<_> = failed_orgs
        .iter()
        .take(MAX_FAILURE_ORGS_IN_MSG)
        .cloned()
        .collect();
    let mut msg = format!(
        "partial org fetch failures ({}): {}",
        failed_orgs.len(),
        shown.join(", ")
    );
    if failed_orgs.len() > MAX_FAILURE_ORGS_IN_MSG {
        msg.push_str(&format!(
            " (+{} more)",
            failed_orgs.len() - MAX_FAILURE_ORGS_IN_MSG
        ));
    }
    msg
}

/// Repo names that collide with common monorepo paths; prepend `{org}-`.
const GENERIC_REPO_NAMES: &[&str] = &[
    "skills",
    "docs",
    "examples",
    "sdk",
    "api",
    "contracts",
    "cli",
    "core",
    "tools",
    "utils",
    "lib",
    "app",
    "web",
    "bot",
    "test",
];

fn is_chain_topic(topic: &str) -> bool {
    matches!(
        topic.to_lowercase().as_str(),
        "ethereum"
            | "bitcoin"
            | "solana"
            | "base"
            | "polygon"
            | "arbitrum"
            | "optimism"
            | "avalanche"
            | "ton"
            | "tron"
    )
}

/// Agent/MCP surface gate — reduces infra-only repo noise in admin queue.
pub(crate) fn has_agent_surface(repo: &OrgRepo) -> bool {
    if repo
        .topics
        .iter()
        .any(|t| AGENT_TOPICS.contains(&t.to_lowercase().as_str()))
    {
        return true;
    }
    let corpus = format!(
        "{} {}",
        repo.name,
        repo.description.as_deref().unwrap_or("")
    )
    .to_lowercase();
    [
        "mcp",
        "agent",
        "skill",
        "cli",
        "x402",
        "payment protocol",
        "payments protocol",
    ]
    .iter()
    .any(|kw| corpus.contains(kw))
}

/// GitHub org repo list item (`GET /orgs/{org}/repos`).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct OrgRepo {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub fork: bool,
    pub archived: bool,
    pub stargazers_count: i32,
    pub pushed_at: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub language: Option<String>,
}

/// Whether a repo name needs `{org}-{repo}` disambiguation before slugging.
pub(crate) fn should_rename_repo_slug(repo_name: &str) -> bool {
    let lower = repo_name.to_lowercase();
    repo_name.len() <= 5 || GENERIC_REPO_NAMES.contains(&lower.as_str())
}

/// Effective display/slug seed name for a vendor-org repo.
pub(crate) fn effective_tool_name(org: &str, repo_name: &str) -> String {
    if should_rename_repo_slug(repo_name) {
        format!("{org}-{repo_name}")
    } else {
        repo_name.to_string()
    }
}

/// Repo name patterns that are NOT developer tools. These are excluded from
/// the vendor-org crawl even when they belong to a first-party org, so the
/// admin queue and verify-tool-official harness never see them as candidates.
const NON_TOOL_REPO_PATTERNS: &[&str] = &[
    "docs",
    "documentation",
    "documentation-en",
    "documentation-zh",
    "doc.",
    "consensus-specs",
    "execution-specs",
    "execution-apis",
    "cryptography-specs",
    "devp2p",
    "walletconnect-specs",
    "teps",
    "program-examples",
    "crypto-primitives-examples",
    "query-examples",
    "haskell-nix-example",
    "example-",
    "demo-",
    "-demo",
    "demos",
    "ansible-role-",
    "helm-charts",
    "eth-phishing-detect",
    "phishing-detect",
    "audits",
    "security-audits",
    "essential-cardano-content",
    "errorprone-checks",
    "docs-template",
    "safe-apps-list",
    "safe-transaction-service",
    "sun-network",
    "x402.chat",
    "pay-skills",
    "contract-deployments",
    "account-policies",
    "action-is-release",
    "action-publish-release",
    "contributor-docs",
    "ouroboros-leios-formal-spec",
    "adnl-tunnel",
    "lz-address-book",
    ".github",
    "esp-website",
    "ethereum-org-website",
    "zkvm-website",
    "steel-website",
    "ton-blockchain.github.io",
];

/// Return true if the repo name matches a known non-tool pattern (docs, specs,
/// demos, examples, infra automation, org-profile repos, audits, etc.).
pub(crate) fn is_non_tool_repo(repo_name: &str) -> bool {
    let name_lower = repo_name.to_lowercase();
    NON_TOOL_REPO_PATTERNS.iter().any(|pattern| {
        let p = pattern.to_lowercase();
        name_lower == p
            || name_lower.starts_with(&p)
            || name_lower.ends_with(&p)
            || (p == ".github" && name_lower == ".github")
    })
}

/// Filter org repos per §4.6: no fork/archived, min stars, recency, top 25 by push.
pub(crate) fn filter_org_repos(repos: &[OrgRepo], now: DateTime<Utc>) -> Vec<&OrgRepo> {
    let cutoff = now - chrono::Duration::days(RECENCY_DAYS);
    let mut qualifying: Vec<&OrgRepo> = repos
        .iter()
        .filter(|repo| !repo.fork && !repo.archived && repo.stargazers_count >= MIN_STARS)
        .filter(|repo| {
            repo.pushed_at
                .as_ref()
                .and_then(|s| parse_datetime(s))
                .is_some_and(|pushed| pushed >= cutoff)
        })
        .filter(|repo| !is_non_tool_repo(&repo.name))
        .filter(|repo| has_agent_surface(repo))
        .collect();

    qualifying.sort_by(|a, b| {
        let star_cmp = b.stargazers_count.cmp(&a.stargazers_count);
        if star_cmp != std::cmp::Ordering::Equal {
            return star_cmp;
        }
        let a_pushed = a
            .pushed_at
            .as_ref()
            .and_then(|s| parse_datetime(s))
            .unwrap_or(DateTime::<Utc>::MIN_UTC);
        let b_pushed = b
            .pushed_at
            .as_ref()
            .and_then(|s| parse_datetime(s))
            .unwrap_or(DateTime::<Utc>::MIN_UTC);
        b_pushed.cmp(&a_pushed)
    });

    qualifying.truncate(MAX_REPOS_PER_ORG);
    qualifying
}

fn infer_tool_type(name: &str, description: Option<&str>, topics: &[String]) -> String {
    let corpus = format!(
        "{} {} {}",
        name,
        description.unwrap_or(""),
        topics.join(" ").to_lowercase()
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
        "sdk".to_string()
    }
}

/// Map a filtered org repo to a [`RawTool`], applying slug-rename policy on `name`.
pub(crate) fn org_repo_to_raw(org: &str, team: &str, repo: &OrgRepo) -> RawTool {
    let name = effective_tool_name(org, &repo.name);
    let repo_url = repo.html_url.clone();
    let tool_type = infer_tool_type(&repo.name, repo.description.as_deref(), &repo.topics);
    let last_commit_at = repo.pushed_at.as_ref().and_then(|s| parse_datetime(s));
    let chains: Vec<String> = repo
        .topics
        .iter()
        .filter(|t| is_chain_topic(t))
        .map(|t| t.to_lowercase())
        .collect();

    RawTool {
        name,
        description: repo.description.clone(),
        tool_type,
        repo_url: Some(repo_url.clone()),
        homepage: Some(repo_url),
        chains,
        stars: repo.stargazers_count,
        last_commit_at,
        source: SOURCE_NAME.to_string(),
        source_url: Some(repo.html_url.clone()),
        official_team: Some(team.to_string()),
        ..Default::default()
    }
}

/// Map org repos to raw tools, excluding URLs already present in `existing_repo_urls`.
pub(crate) fn map_org_repos_to_raws(
    org: &str,
    team: &str,
    repos: &[OrgRepo],
    existing_repo_urls: &HashSet<String>,
    now: DateTime<Utc>,
) -> Vec<RawTool> {
    filter_org_repos(repos, now)
        .into_iter()
        .filter(|repo| !existing_repo_urls.contains(&repo.html_url))
        .map(|repo| org_repo_to_raw(org, team, repo))
        .collect()
}

async fn fetch_org_repos_page(
    client: &reqwest::Client,
    token: Option<&str>,
    org: &str,
    base_url: &str,
    page: u32,
) -> Result<Vec<OrgRepo>> {
    let url = format!("{base_url}/orgs/{org}/repos");
    let mut request = client.get(&url).query(&[
        ("per_page", "100"),
        ("type", "public"),
        ("sort", "pushed"),
        ("page", &page.to_string()),
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
        .with_context(|| format!("GitHub org repos request failed for {org} page {page}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("GitHub org repos for {org} returned HTTP {status}: {body}");
    }

    response
        .json()
        .await
        .context("parsing GitHub org repos JSON")
}

async fn fetch_org_repos_at_url(
    client: &reqwest::Client,
    token: Option<&str>,
    org: &str,
    base_url: &str,
) -> Result<Vec<OrgRepo>> {
    let mut all = Vec::new();
    for page in 1..=MAX_ORG_REPO_PAGES {
        let page_items = fetch_org_repos_page(client, token, org, base_url, page).await?;
        let count = page_items.len();
        all.extend(page_items);
        if count < 100 {
            break;
        }
    }
    Ok(all)
}

async fn load_existing_repo_urls(pool: &sqlx::PgPool) -> Result<HashSet<String>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT repo_url FROM tools WHERE repo_url IS NOT NULL")
            .fetch_all(pool)
            .await
            .context("loading existing repo_url values")?;
    Ok(rows.into_iter().map(|(url,)| url).collect())
}

/// Crawl all `crawl: true` vendor orgs against a configurable GitHub API base.
pub(crate) async fn crawl_orgs_at_base(
    token: Option<&str>,
    base_url: &str,
    existing_repo_urls: &HashSet<String>,
    now: DateTime<Utc>,
) -> Result<VendorOrgsCrawlOutcome> {
    let client = github_client(token)?;
    let manifest = vendor_orgs_manifest();
    let mut out = Vec::new();
    let mut failed_orgs = Vec::new();

    for entry in manifest.orgs.iter().filter(|e| e.crawl) {
        match fetch_org_repos_at_url(&client, token, &entry.github, base_url).await {
            Ok(repos) => {
                let mapped = map_org_repos_to_raws(
                    &entry.github,
                    &entry.team,
                    &repos,
                    existing_repo_urls,
                    now,
                );
                tracing::debug!(
                    org = %entry.github,
                    fetched = repos.len(),
                    kept = mapped.len(),
                    "vendor org repos mapped"
                );
                out.extend(mapped);
            }
            Err(e) => {
                tracing::error!(org = %entry.github, error = %e, "vendor org repo fetch failed");
                failed_orgs.push(entry.github.clone());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    Ok(VendorOrgsCrawlOutcome {
        raws: out,
        failed_orgs,
    })
}

/// Crawl all `crawl: true` vendor orgs using the production GitHub API.
pub(crate) async fn crawl_orgs(
    existing_repo_urls: &HashSet<String>,
    now: DateTime<Utc>,
) -> Result<VendorOrgsCrawlOutcome> {
    let token = std::env::var("GITHUB_API_TOKEN")
        .ok()
        .filter(|s| !s.is_empty());
    crawl_orgs_at_base(token.as_deref(), GITHUB_API_BASE, existing_repo_urls, now).await
}

pub async fn run_once(pool: &sqlx::PgPool) {
    let existing_repo_urls = match load_existing_repo_urls(pool).await {
        Ok(urls) => urls,
        Err(e) => {
            tracing::error!(source = SOURCE_NAME, error = %e, "failed to load repo_url set");
            crate::crawler::update_source_status(
                crate::crawler::UpsertTarget::Pool(pool),
                SOURCE_NAME,
                VENDOR_ORGS_REGISTRY_URL,
                "error",
                0,
                Some(&e.to_string()),
            )
            .await;
            return;
        }
    };

    match crawl_orgs(&existing_repo_urls, Utc::now()).await {
        Ok(outcome) => {
            let count = outcome.raws.len() as i32;
            tracing::info!(
                source = SOURCE_NAME,
                count = outcome.raws.len(),
                failed_orgs = outcome.failed_orgs.len(),
                "crawl completed"
            );
            if !outcome.raws.is_empty() {
                crate::crawler::persist_crawl_results_gated(
                    pool,
                    SOURCE_NAME,
                    VENDOR_ORGS_REGISTRY_URL,
                    outcome.raws,
                )
                .await;
            }
            if !outcome.failed_orgs.is_empty() {
                let msg = format_vendor_org_failures(&outcome.failed_orgs);
                crate::crawler::update_source_status(
                    crate::crawler::UpsertTarget::Pool(pool),
                    SOURCE_NAME,
                    VENDOR_ORGS_REGISTRY_URL,
                    "error",
                    count,
                    Some(&msg),
                )
                .await;
            } else if count == 0 {
                crate::crawler::update_source_status(
                    crate::crawler::UpsertTarget::Pool(pool),
                    SOURCE_NAME,
                    VENDOR_ORGS_REGISTRY_URL,
                    "success",
                    0,
                    None,
                )
                .await;
            }
        }
        Err(e) => {
            tracing::error!(source = SOURCE_NAME, error = %e, "crawl failed");
            crate::crawler::update_source_status(
                crate::crawler::UpsertTarget::Pool(pool),
                SOURCE_NAME,
                VENDOR_ORGS_REGISTRY_URL,
                "error",
                0,
                Some(&e.to_string()),
            )
            .await;
        }
    }
}

/// Production crawl with `repo_url` exclusion loaded from the database.
pub async fn crawl_for_pool(pool: &sqlx::PgPool) -> Result<Vec<RawTool>> {
    let existing_repo_urls = load_existing_repo_urls(pool).await?;
    let outcome = crawl_orgs(&existing_repo_urls, Utc::now()).await?;
    if !outcome.failed_orgs.is_empty() {
        tracing::warn!(
            failed = outcome.failed_orgs.len(),
            error = %format_vendor_org_failures(&outcome.failed_orgs),
            "vendor_orgs partial org failures (returning successful raws)"
        );
    }
    Ok(outcome.raws)
}

pub struct VendorOrgsCrawler;

#[async_trait::async_trait]
impl SourceCrawler for VendorOrgsCrawler {
    async fn crawl(&self) -> Result<Vec<RawTool>> {
        anyhow::bail!(
            "vendor_orgs requires crawl_with_pool(pool) or run_once; pool-less crawl skips repo_url exclusion"
        )
    }

    async fn crawl_with_pool(&self, pool: &sqlx::PgPool) -> Result<Vec<RawTool>> {
        crawl_for_pool(pool).await
    }

    fn source_name(&self) -> &str {
        SOURCE_NAME
    }

    fn interval(&self) -> &'static str {
        "0 45 3 * * *"
    }
}

#[cfg(test)]
#[path = "vendor_orgs_tests.rs"]
mod tests;
