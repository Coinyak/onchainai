//! Source crawlers — one module per data source.
//!
//! The [`SourceCrawler`] trait is the contract each source implements.

pub mod cryptoskill;
pub mod github;
pub mod npm;
pub mod web3mcp;

use async_trait::async_trait;

/// Contract for a single data source crawler.
#[allow(dead_code)]
#[async_trait]
pub trait SourceCrawler: Send + Sync {
    /// Crawl the source, returning normalized raw tools.
    async fn crawl(&self) -> anyhow::Result<Vec<crate::crawler::normalizer::RawTool>>;
    /// Stable source identifier (e.g. `cryptoskill`, `github`).
    fn source_name(&self) -> &str;
    /// Cron-compatible interval description.
    fn interval(&self) -> &'static str;
}
