//! Source crawlers — one module per data source.
//!
//! The [`SourceCrawler`] trait is the contract each source implements.
//! Source implementations (cryptoskill, web3mcp, github, npm) are added in
//! the `crawler-sources` milestone; this module defines the trait and the
//! shared HTTP client helpers used by all sources.

pub mod cryptoskill;
pub mod github;
pub mod npm;
pub mod web3mcp;

use async_trait::async_trait;

use crate::crawler::normalizer::RawTool;

/// Shared User-Agent string for all crawl HTTP requests.
///
/// Setting an explicit User-Agent avoids 403s from the GitHub API and is
/// courteous to other sources. See `docs/SECURITY.md` §6.2.
pub const CRAWLER_USER_AGENT: &str = concat!(
    "OnchainAI-Crawler/",
    env!("CARGO_PKG_VERSION"),
    " (+https://onchainai.xyz)"
);

/// HTTP request timeout for all crawl requests (30s per MVP_DESIGN.md §3).
pub const CRAWLER_TIMEOUT_SECS: u64 = 30;

/// Build a `reqwest::Client` pre-configured with the crawler User-Agent and
/// 30s timeout. All source crawlers should use this to ensure consistent
/// headers and timeouts.
pub fn http_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(CRAWLER_USER_AGENT)
        .timeout(std::time::Duration::from_secs(CRAWLER_TIMEOUT_SECS))
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build crawler HTTP client: {e}"))
}

/// Contract for a single data source crawler.
///
/// Each source (CryptoSkill, web3-mcp-hub, GitHub topics, npm) implements this
/// trait. The orchestrator runs all sources in parallel via `tokio::spawn`.
#[async_trait]
#[allow(dead_code)]
pub trait SourceCrawler: Send + Sync {
    /// Crawl the source, returning raw (pre-normalization) tools.
    ///
    /// Errors are returned to the orchestrator, which logs them and updates
    /// the `sources` table without crashing the scheduler.
    async fn crawl(&self) -> anyhow::Result<Vec<RawTool>>;

    /// Stable source identifier (e.g. `cryptoskill`, `github`).
    fn source_name(&self) -> &str;

    /// Cron-compatible interval description, for diagnostics/logging.
    fn interval(&self) -> &'static str;
}

// Force the compiler to consider the trait "used" at the crate level so that
// dead-code warnings are not emitted for the trait definition itself. Concrete
// crawler structs are registered by the scheduler.
#[doc(hidden)]
#[allow(dead_code)]
pub trait _SourceCrawlerSealed: SourceCrawler {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_contains_name_and_version() {
        assert!(CRAWLER_USER_AGENT.starts_with("OnchainAI-Crawler/"));
        assert!(CRAWLER_USER_AGENT.contains("onchainai.xyz"));
    }

    #[test]
    fn http_client_sets_user_agent_and_timeout() -> anyhow::Result<()> {
        let client = http_client()?;
        // We can't directly inspect the timeout on a reqwest::Client, but we
        // can confirm the client was built successfully. The User-Agent is
        // baked in at build time and not inspectable either; this test guards
        // against builder regressions (e.g. bad env var at compile time).
        let _ = client;
        Ok(())
    }
}
