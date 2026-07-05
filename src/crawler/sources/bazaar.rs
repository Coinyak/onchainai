//! CDP Bazaar x402 discovery crawler (PR-5).
//!
//! Fetches paginated resources from Coinbase CDP discovery, applies a spam
//! floor (`l30DaysUniquePayers >= 5`), groups by host (≤100 hosts/run), probes
//! the representative endpoint at ingest, and persists via
//! [`crate::crawler::persist_crawl_results_gated`].

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::crawler::normalizer::RawTool;
use crate::crawler::sources::SourceCrawler;
use crate::server::x402_verify::{self, ProbeOutcome};

const SOURCE_NAME: &str = "bazaar";
pub(crate) const BAZAAR_DISCOVERY_URL: &str =
    "https://api.cdp.coinbase.com/platform/v2/x402/discovery/resources";
const MAX_PAGES: u32 = 3;
const PAGE_LIMIT: u32 = 100;
const MIN_UNIQUE_PAYERS: u32 = 5;
const MAX_HOSTS_PER_RUN: usize = 100;
/// Match [`crate::server::x402_verify`] scheduled probe concurrency.
const BAZAAR_PROBE_CONCURRENCY: usize = 4;

/// Mainnet chain IDs accepted for catalog ingest (§4.5).
const MAINNET_CHAIN_MAP: &[(u32, &str)] = &[
    (8453, "base"),
    (1, "ethereum"),
    (137, "polygon"),
    (42161, "arbitrum"),
    (10, "optimism"),
    (43114, "avalanche"),
];

/// CDP discovery list response.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(crate) struct DiscoveryResponse {
    pub items: Vec<BazaarResource>,
    #[serde(default)]
    pub pagination: Option<DiscoveryPagination>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct DiscoveryPagination {
    pub limit: u32,
    pub offset: u32,
    pub total: u32,
}

/// Single Bazaar discovery resource.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(crate) struct BazaarResource {
    pub resource: String,
    #[serde(rename = "serviceName", default)]
    pub service_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub accepts: Vec<BazaarAccept>,
    #[serde(default)]
    pub quality: Option<BazaarQuality>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct BazaarAccept {
    pub network: String,
    #[serde(default)]
    pub amount: Option<String>,
    #[serde(default)]
    pub asset: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct BazaarQuality {
    #[serde(rename = "l30DaysUniquePayers", default)]
    pub l30_days_unique_payers: Option<u32>,
}

/// Parse `eip155:<chainId>` from a CDP `accepts[].network` value.
pub(crate) fn parse_chain_id_from_network(network: &str) -> Option<u32> {
    let rest = network.strip_prefix("eip155:")?;
    rest.parse().ok()
}

/// Map a mainnet chain ID to the catalog chain slug, if known.
pub(crate) fn map_mainnet_chain_id(chain_id: u32) -> Option<&'static str> {
    MAINNET_CHAIN_MAP
        .iter()
        .find(|(id, _)| *id == chain_id)
        .map(|(_, slug)| *slug)
}

/// True when at least one accept maps to a supported mainnet chain.
pub(crate) fn resource_has_mainnet_chain(accepts: &[BazaarAccept]) -> bool {
    accepts
        .iter()
        .filter_map(|accept| parse_chain_id_from_network(&accept.network))
        .any(|id| map_mainnet_chain_id(id).is_some())
}

/// Map accepts to deduplicated mainnet chain slugs.
pub(crate) fn map_resource_chains(accepts: &[BazaarAccept]) -> Vec<String> {
    let mut chains = Vec::new();
    for accept in accepts {
        if let Some(id) = parse_chain_id_from_network(&accept.network) {
            if let Some(slug) = map_mainnet_chain_id(id) {
                if !chains.iter().any(|c| c == slug) {
                    chains.push(slug.to_string());
                }
            }
        }
    }
    chains
}

/// Spam floor: `l30DaysUniquePayers >= 5`.
pub(crate) fn meets_payers_floor(quality: &BazaarQuality) -> bool {
    quality.l30_days_unique_payers.unwrap_or(0) >= MIN_UNIQUE_PAYERS
}

/// Unique payers for representative selection and `stars` proxy.
pub(crate) fn unique_payers(quality: Option<&BazaarQuality>) -> u32 {
    quality.and_then(|q| q.l30_days_unique_payers).unwrap_or(0)
}

/// Lowercase host key for merchant grouping.
pub(crate) fn host_key_from_resource(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(|h| h.to_lowercase()))
}

/// Whether the resource URL path contains a `:param` template segment.
pub(crate) fn resource_path_has_param(url: &str) -> bool {
    url::Url::parse(url)
        .ok()
        .is_some_and(|parsed| parsed.path().contains(':'))
}

/// Normalize an x402 endpoint for dedupe (lowercase, trim trailing slash).
pub(crate) fn normalize_x402_endpoint(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    trimmed.to_lowercase()
}

/// Pick the best representative within a single-host group.
pub(crate) fn pick_representative<'a>(group: &[&'a BazaarResource]) -> &'a BazaarResource {
    debug_assert!(!group.is_empty());
    group
        .iter()
        .copied()
        .max_by(|a, b| {
            let payers_a = unique_payers(a.quality.as_ref());
            let payers_b = unique_payers(b.quality.as_ref());
            payers_a
                .cmp(&payers_b)
                .then_with(|| {
                    let a_param = resource_path_has_param(&a.resource);
                    let b_param = resource_path_has_param(&b.resource);
                    (!a_param).cmp(&!b_param)
                })
                .then_with(|| a.resource.cmp(&b.resource))
        })
        .expect("non-empty group")
}

/// Filter spam floor + testnet-only, group by host, cap at 100 hosts.
pub(crate) fn transform_discovery_items(items: Vec<BazaarResource>) -> Vec<BazaarResource> {
    let filtered: Vec<BazaarResource> = items
        .into_iter()
        .filter(|item| {
            item.quality.as_ref().is_some_and(meets_payers_floor)
                && resource_has_mainnet_chain(&item.accepts)
        })
        .collect();

    let mut by_host: HashMap<String, Vec<&BazaarResource>> = HashMap::new();
    for item in &filtered {
        if let Some(host) = host_key_from_resource(&item.resource) {
            by_host.entry(host).or_default().push(item);
        }
    }

    let mut representatives: Vec<BazaarResource> = by_host
        .values()
        .map(|group| pick_representative(group).clone())
        .collect();

    representatives.sort_by_key(|b| std::cmp::Reverse(unique_payers(b.quality.as_ref())));
    representatives.truncate(MAX_HOSTS_PER_RUN);

    // Preserve stable ordering for diagnostics after truncation.
    representatives.sort_by(|a, b| a.resource.cmp(&b.resource));
    representatives
}

/// Drop resources whose normalized endpoint already exists in `existing`.
pub(crate) fn dedupe_by_x402_endpoint(
    resources: Vec<BazaarResource>,
    existing: &HashSet<String>,
) -> Vec<BazaarResource> {
    let mut seen = HashSet::new();
    resources
        .into_iter()
        .filter(|item| {
            let key = normalize_x402_endpoint(&item.resource);
            !existing.contains(&key) && seen.insert(key)
        })
        .collect()
}

fn format_x402_price(accepts: &[BazaarAccept]) -> Option<String> {
    let accept = accepts.iter().find(|a| {
        parse_chain_id_from_network(&a.network)
            .and_then(map_mainnet_chain_id)
            .is_some()
    })?;
    let amount = accept.amount.as_deref()?.trim();
    if amount.is_empty() {
        return None;
    }
    match accept
        .asset
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty())
    {
        Some(asset) => Some(format!("{amount} ({asset})")),
        None => Some(amount.to_string()),
    }
}

fn homepage_from_resource(url: &str) -> Option<String> {
    url::Url::parse(url).ok().map(|parsed| {
        let mut origin = parsed.clone();
        origin.set_path("");
        origin.set_query(None);
        origin.set_fragment(None);
        origin.to_string()
    })
}

fn tool_name_from_resource(resource: &BazaarResource) -> String {
    if let Some(name) = resource
        .service_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return name.to_string();
    }
    host_key_from_resource(&resource.resource).unwrap_or_else(|| "x402-endpoint".to_string())
}

/// Map a grouped Bazaar resource to a [`RawTool`] (probe outcome optional).
pub(crate) fn resource_to_raw(
    resource: &BazaarResource,
    relevance_status_override: Option<String>,
) -> RawTool {
    let endpoint = resource.resource.trim().to_string();
    let homepage = homepage_from_resource(&endpoint);
    let chains = map_resource_chains(&resource.accepts);

    RawTool {
        name: tool_name_from_resource(resource),
        description: resource.description.clone(),
        tool_type: "x402".to_string(),
        repo_url: homepage.clone(),
        homepage,
        chains,
        stars: unique_payers(resource.quality.as_ref()) as i32,
        source: SOURCE_NAME.to_string(),
        source_url: Some(BAZAAR_DISCOVERY_URL.to_string()),
        pricing: "x402".to_string(),
        x402_price: format_x402_price(&resource.accepts),
        x402_endpoint: Some(endpoint),
        relevance_status_override,
        ..Default::default()
    }
}

fn relevance_status_for_probe(outcome: &ProbeOutcome) -> Option<String> {
    match outcome {
        ProbeOutcome::Verified { .. } => Some("accepted".to_string()),
        ProbeOutcome::NotPaymentRequired
        | ProbeOutcome::ParseFailed
        | ProbeOutcome::RequestFailed(_)
        | ProbeOutcome::SsrfBlocked(_) => Some("needs_review".to_string()),
    }
}

async fn probe_resource(resource: &BazaarResource) -> ProbeOutcome {
    x402_verify::probe_x402_endpoint(resource.resource.trim()).await
}

/// Map resources to raw tools with ingest-time probe (bounded parallelism).
pub(crate) async fn map_resources_with_probe(resources: &[BazaarResource]) -> Vec<RawTool> {
    use tokio::sync::Semaphore;

    let semaphore = Arc::new(Semaphore::new(BAZAAR_PROBE_CONCURRENCY));
    let mut handles = Vec::with_capacity(resources.len());
    for (idx, resource) in resources.iter().enumerate() {
        let resource = resource.clone();
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("bazaar probe semaphore");
        handles.push(tokio::spawn(async move {
            let _permit = permit;
            let outcome = probe_resource(&resource).await;
            (
                idx,
                resource_to_raw(&resource, relevance_status_for_probe(&outcome)),
            )
        }));
    }

    let mut indexed = Vec::with_capacity(handles.len());
    for handle in handles {
        indexed.push(handle.await.expect("bazaar probe task"));
    }
    indexed.sort_by_key(|(idx, _)| *idx);
    indexed.into_iter().map(|(_, raw)| raw).collect()
}

async fn fetch_discovery_page(
    client: &reqwest::Client,
    base_url: &str,
    offset: u32,
) -> Result<DiscoveryResponse> {
    let url = format!("{base_url}/v2/x402/discovery/resources");
    let response = client
        .get(&url)
        .query(&[
            ("limit", PAGE_LIMIT.to_string()),
            ("offset", offset.to_string()),
        ])
        .send()
        .await
        .context("CDP Bazaar discovery request failed")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("CDP Bazaar discovery returned HTTP {status}: {body}");
    }

    response
        .json()
        .await
        .context("parsing CDP Bazaar discovery JSON")
}

async fn fetch_all_discovery_items(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<Vec<BazaarResource>> {
    let mut all = Vec::new();
    for page in 0..MAX_PAGES {
        let offset = page * PAGE_LIMIT;
        let response = fetch_discovery_page(client, base_url, offset).await?;
        let count = response.items.len();
        all.extend(response.items);
        if count < PAGE_LIMIT as usize {
            break;
        }
    }
    Ok(all)
}

async fn load_existing_x402_endpoints(pool: &sqlx::PgPool) -> Result<HashSet<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT x402_endpoint FROM tools WHERE x402_endpoint IS NOT NULL AND trim(x402_endpoint) <> ''",
    )
    .fetch_all(pool)
    .await
    .context("loading existing x402_endpoint values")?;
    Ok(rows
        .into_iter()
        .map(|(url,)| normalize_x402_endpoint(&url))
        .collect())
}

/// Crawl CDP Bazaar discovery at a configurable API base (wiremock-friendly).
pub(crate) async fn crawl_bazaar_at_base(
    base_url: &str,
    existing_endpoints: &HashSet<String>,
) -> Result<Vec<RawTool>> {
    crawl_bazaar_at_base_impl(base_url, existing_endpoints, None).await
}

/// Wiremock/tests: same shipped fetch→transform→dedupe path without live seller probes.
#[cfg(test)]
pub(crate) async fn crawl_bazaar_at_base_with_stub_probe(
    base_url: &str,
    existing_endpoints: &HashSet<String>,
    stub: ProbeOutcome,
) -> Result<Vec<RawTool>> {
    crawl_bazaar_at_base_impl(base_url, existing_endpoints, Some(stub)).await
}

async fn crawl_bazaar_at_base_impl(
    base_url: &str,
    existing_endpoints: &HashSet<String>,
    stub_probe: Option<ProbeOutcome>,
) -> Result<Vec<RawTool>> {
    let client = crate::crawler::sources::http_client()?;
    let items = fetch_all_discovery_items(&client, base_url).await?;
    let grouped = transform_discovery_items(items);
    let deduped = dedupe_by_x402_endpoint(grouped, existing_endpoints);
    let raws = match stub_probe {
        Some(outcome) => deduped
            .iter()
            .map(|resource| resource_to_raw(resource, relevance_status_for_probe(&outcome)))
            .collect(),
        None => map_resources_with_probe(&deduped).await,
    };
    Ok(raws)
}

/// Production crawl using the CDP API and DB-backed endpoint dedupe.
pub async fn crawl_bazaar(existing_endpoints: &HashSet<String>) -> Result<Vec<RawTool>> {
    crawl_bazaar_at_base("https://api.cdp.coinbase.com/platform", existing_endpoints).await
}

pub async fn run_once(pool: &sqlx::PgPool) {
    let existing_endpoints = match load_existing_x402_endpoints(pool).await {
        Ok(endpoints) => endpoints,
        Err(e) => {
            tracing::error!(source = SOURCE_NAME, error = %e, "failed to load x402_endpoint set");
            crate::crawler::update_source_status(
                crate::crawler::UpsertTarget::Pool(pool),
                SOURCE_NAME,
                BAZAAR_DISCOVERY_URL,
                "error",
                0,
                Some(&e.to_string()),
            )
            .await;
            return;
        }
    };

    match crawl_bazaar(&existing_endpoints).await {
        Ok(raws) => {
            tracing::info!(source = SOURCE_NAME, count = raws.len(), "crawl completed");
            crate::crawler::persist_crawl_results_gated(
                pool,
                SOURCE_NAME,
                BAZAAR_DISCOVERY_URL,
                raws,
            )
            .await;
        }
        Err(e) => {
            tracing::error!(source = SOURCE_NAME, error = %e, "crawl failed");
            crate::crawler::update_source_status(
                crate::crawler::UpsertTarget::Pool(pool),
                SOURCE_NAME,
                BAZAAR_DISCOVERY_URL,
                "error",
                0,
                Some(&e.to_string()),
            )
            .await;
        }
    }
}

pub async fn crawl_for_pool(pool: &sqlx::PgPool) -> Result<Vec<RawTool>> {
    let existing_endpoints = load_existing_x402_endpoints(pool).await?;
    crawl_bazaar(&existing_endpoints).await
}

pub struct BazaarCrawler;

#[async_trait::async_trait]
impl SourceCrawler for BazaarCrawler {
    async fn crawl(&self) -> Result<Vec<RawTool>> {
        anyhow::bail!(
            "bazaar requires crawl_with_pool(pool) or run_once; pool-less crawl skips x402_endpoint dedupe"
        )
    }

    async fn crawl_with_pool(&self, pool: &sqlx::PgPool) -> Result<Vec<RawTool>> {
        crawl_for_pool(pool).await
    }

    fn source_name(&self) -> &str {
        SOURCE_NAME
    }

    fn interval(&self) -> &'static str {
        "0 20 */6 * * *"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_resource(
        resource: &str,
        service_name: &str,
        payers: u32,
        network: &str,
    ) -> BazaarResource {
        BazaarResource {
            resource: resource.to_string(),
            service_name: Some(service_name.to_string()),
            description: Some(format!("{service_name} x402 API")),
            accepts: vec![BazaarAccept {
                network: network.to_string(),
                amount: Some("1000".to_string()),
                asset: Some("USDC".to_string()),
            }],
            quality: Some(BazaarQuality {
                l30_days_unique_payers: Some(payers),
            }),
        }
    }

    #[test]
    fn bazaar_payers_floor_excludes_sub_floor_resources() {
        let items = vec![
            sample_resource(
                "https://merchant.example/api/low",
                "Low Payers",
                4,
                "eip155:8453",
            ),
            sample_resource(
                "https://merchant.example/api/ok",
                "Ok Payers",
                5,
                "eip155:8453",
            ),
        ];
        let out = transform_discovery_items(items);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].service_name.as_deref(), Some("Ok Payers"));
    }

    #[test]
    fn bazaar_chain_map_maps_mainnet_ids() {
        let chains = map_resource_chains(&[BazaarAccept {
            network: "eip155:8453".to_string(),
            amount: None,
            asset: None,
        }]);
        assert_eq!(chains, vec!["base".to_string()]);
        assert_eq!(map_mainnet_chain_id(1), Some("ethereum"));
        assert_eq!(map_mainnet_chain_id(137), Some("polygon"));
        assert_eq!(map_mainnet_chain_id(42161), Some("arbitrum"));
        assert_eq!(map_mainnet_chain_id(10), Some("optimism"));
        assert_eq!(map_mainnet_chain_id(43114), Some("avalanche"));
    }

    #[test]
    fn bazaar_testnet_only_resources_are_dropped() {
        let items = vec![sample_resource(
            "https://testnet-only.example/api",
            "Testnet Only",
            10,
            "eip155:84532",
        )];
        assert!(transform_discovery_items(items).is_empty());
    }

    #[test]
    fn bazaar_grouping_collapses_same_merchant_resources() {
        let items = vec![
            sample_resource(
                "https://merchant.example/api/search/:query",
                "Merchant Search",
                20,
                "eip155:8453",
            ),
            sample_resource(
                "https://merchant.example/api/list",
                "Merchant List",
                25,
                "eip155:8453",
            ),
            sample_resource("https://other.example/api", "Other Merchant", 8, "eip155:1"),
        ];
        let out = transform_discovery_items(items);
        assert_eq!(out.len(), 2);
        let merchant = out
            .iter()
            .find(|r| host_key_from_resource(&r.resource).as_deref() == Some("merchant.example"))
            .expect("merchant host present");
        assert_eq!(
            merchant.resource, "https://merchant.example/api/list",
            "higher payers and no :param path wins"
        );
    }

    #[test]
    fn bazaar_resource_to_raw_sets_x402_fields_and_referral_disabled_via_normalize() {
        let resource = sample_resource(
            "https://merchant.example/api/list",
            "Merchant List",
            12,
            "eip155:8453",
        );
        let raw = resource_to_raw(&resource, Some("accepted".to_string()));
        assert_eq!(raw.tool_type, "x402");
        assert_eq!(raw.pricing, "x402");
        assert_eq!(
            raw.x402_endpoint.as_deref(),
            Some("https://merchant.example/api/list")
        );
        assert_eq!(raw.relevance_status_override.as_deref(), Some("accepted"));

        let tools = crate::crawler::prepare_crawled_tools_gated(&[raw], "bazaar", false);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].pricing, "x402");
        assert_eq!(tools[0].tool_type, "x402");
        assert!(!tools[0].referral_enabled);
        assert_eq!(tools[0].relevance_status, "accepted");
        assert_eq!(tools[0].approval_status, "pending");
    }

    #[test]
    fn bazaar_dedupe_by_x402_endpoint_normalizes_case_and_trailing_slash() {
        let resources = vec![
            sample_resource("https://Merchant.Example/API/", "One", 10, "eip155:8453"),
            sample_resource("https://merchant.example/api", "Two", 11, "eip155:8453"),
        ];
        let out = dedupe_by_x402_endpoint(resources, &HashSet::new());
        assert_eq!(out.len(), 1);
    }

    fn discovery_page_json(items: serde_json::Value, offset: u32) -> String {
        serde_json::json!({
            "items": items,
            "pagination": {"limit": 100, "offset": offset, "total": 200}
        })
        .to_string()
    }

    fn discovery_item(
        resource: &str,
        service_name: &str,
        payers: u32,
        network: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "resource": resource,
            "serviceName": service_name,
            "description": format!("{service_name} x402 API"),
            "accepts": [{"network": network, "amount": "1000", "asset": "USDC"}],
            "quality": {"l30DaysUniquePayers": payers}
        })
    }

    #[test]
    fn bazaar_wiremock_transform_discovery_items_prints_observations() {
        let parsed: DiscoveryResponse = serde_json::from_str(&discovery_page_json(
            serde_json::json!([
                discovery_item(
                    "https://merchant.example/api/search/:q",
                    "Merchant Search",
                    12,
                    "eip155:8453",
                ),
                discovery_item(
                    "https://merchant.example/api/list",
                    "Merchant List",
                    15,
                    "eip155:8453",
                ),
                discovery_item("https://spam.example/api", "Spam", 2, "eip155:8453"),
                discovery_item("https://sepolia.example/api", "Sepolia", 20, "eip155:84532",),
            ]),
            0,
        ))
        .expect("parse wiremock CDP page JSON");

        let input_count = parsed.items.len();
        let sub_floor = parsed
            .items
            .iter()
            .filter(|i| !i.quality.as_ref().is_some_and(meets_payers_floor))
            .count();
        let testnet_only = parsed
            .items
            .iter()
            .filter(|i| !resource_has_mainnet_chain(&i.accepts))
            .count();

        let grouped = transform_discovery_items(parsed.items);
        let raws: Vec<RawTool> = grouped
            .iter()
            .map(|r| resource_to_raw(r, Some("accepted".to_string())))
            .collect();

        eprintln!("bazaar-transform: input_items={input_count}");
        eprintln!("bazaar-transform: sub_floor_excluded={sub_floor}");
        eprintln!("bazaar-transform: testnet_only_excluded={testnet_only}");
        eprintln!("bazaar-transform: grouped_hosts={}", grouped.len());
        eprintln!(
            "bazaar-transform: raw_tools={} pricing=x402 tool_type=x402 referral_enabled=false",
            raws.len()
        );
        for raw in &raws {
            eprintln!(
                "bazaar-transform: endpoint={:?} chains={:?} relevance={:?}",
                raw.x402_endpoint, raw.chains, raw.relevance_status_override
            );
        }

        assert_eq!(grouped.len(), 1, "merchant hosts collapse to one");
        assert_eq!(
            grouped[0].resource, "https://merchant.example/api/list",
            "highest payers without :param wins"
        );
        assert_eq!(map_resource_chains(&grouped[0].accepts), vec!["base"]);
        assert_eq!(raws.len(), 1);
        assert_eq!(raws[0].pricing, "x402");
        assert_eq!(raws[0].tool_type, "x402");
        let tools = crate::crawler::prepare_crawled_tools_gated(&raws, "bazaar", false);
        assert!(!tools[0].referral_enabled);
    }

    #[tokio::test]
    async fn bazaar_wiremock_crawl_bazaar_at_base_applies_floor_grouping_pagination_and_dedupe() {
        let server = MockServer::start().await;

        // Page 0 must return PAGE_LIMIT items so fetch_all_discovery_items requests offset=100.
        let mut page0_items: Vec<serde_json::Value> = (0..96)
            .map(|i| {
                discovery_item(
                    &format!("https://filler{i}.example/api"),
                    &format!("Filler {i}"),
                    1,
                    "eip155:8453",
                )
            })
            .collect();
        page0_items.extend([
            discovery_item(
                "https://merchant.example/api/search/:q",
                "Merchant Search",
                12,
                "eip155:8453",
            ),
            discovery_item(
                "https://merchant.example/api/list",
                "Merchant List",
                15,
                "eip155:8453",
            ),
            discovery_item("https://spam.example/api", "Spam", 2, "eip155:8453"),
            discovery_item("https://sepolia.example/api", "Sepolia", 20, "eip155:84532"),
        ]);
        assert_eq!(page0_items.len(), 100);
        let page0 = discovery_page_json(serde_json::Value::Array(page0_items), 0);

        // Page 1: second merchant + duplicate endpoint (dedupe within run).
        let page1 = discovery_page_json(
            serde_json::json!([
                discovery_item("https://other.example/api", "Other Merchant", 8, "eip155:1"),
                discovery_item(
                    "https://merchant.example/api/list/",
                    "Merchant Duplicate",
                    99,
                    "eip155:8453",
                ),
            ]),
            100,
        );

        Mock::given(method("GET"))
            .and(path("/v2/x402/discovery/resources"))
            .and(query_param("limit", "100"))
            .and(query_param("offset", "0"))
            .respond_with(ResponseTemplate::new(200).set_body_string(page0))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v2/x402/discovery/resources"))
            .and(query_param("limit", "100"))
            .and(query_param("offset", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_string(page1))
            .mount(&server)
            .await;

        let mut existing = HashSet::new();
        existing.insert(normalize_x402_endpoint(
            "https://existing.example/api/already-listed",
        ));

        let stub_probe = ProbeOutcome::Verified {
            amount: Some("1000".to_string()),
            asset: Some("USDC".to_string()),
        };
        let raws = crawl_bazaar_at_base_with_stub_probe(&server.uri(), &existing, stub_probe)
            .await
            .expect("crawl_bazaar_at_base through wiremock (stub probe, no seller HTTP)");

        eprintln!("bazaar-crawl: raw_tools={}", raws.len());
        for raw in &raws {
            eprintln!(
                "bazaar-crawl: name={} pricing={} tool_type={} endpoint={:?} chains={:?} relevance={:?}",
                raw.name,
                raw.pricing,
                raw.tool_type,
                raw.x402_endpoint,
                raw.chains,
                raw.relevance_status_override
            );
        }

        assert_eq!(raws.len(), 2, "merchant grouped + other merchant");
        assert!(
            raws.iter()
                .all(|r| r.pricing == "x402" && r.tool_type == "x402"),
            "all items map to x402 pricing/type"
        );
        assert!(
            raws.iter().all(|r| !r.relevance_status_override.is_none()),
            "probe-at-ingest sets relevance override on every row"
        );

        let merchant = raws
            .iter()
            .find(|r| {
                r.x402_endpoint.as_deref().is_some_and(|url| {
                    host_key_from_resource(url).as_deref() == Some("merchant.example")
                })
            })
            .expect("grouped merchant representative");
        assert_eq!(
            normalize_x402_endpoint(merchant.x402_endpoint.as_deref().unwrap()),
            "https://merchant.example/api/list"
        );
        assert!(!resource_path_has_param(
            merchant.x402_endpoint.as_deref().unwrap()
        ));
        assert_eq!(merchant.chains, vec!["base".to_string()]);

        let other = raws
            .iter()
            .find(|r| r.x402_endpoint.as_deref() == Some("https://other.example/api"))
            .expect("page-1 merchant kept");
        assert_eq!(other.chains, vec!["ethereum".to_string()]);

        let tools = crate::crawler::prepare_crawled_tools_gated(&raws, "bazaar", false);
        assert_eq!(tools.len(), 2);
        assert!(tools.iter().all(|t| !t.referral_enabled));
        assert_eq!(tools[0].approval_status, "pending");
    }

    #[tokio::test]
    async fn bazaar_crawl_without_pool_returns_error() {
        let crawler = BazaarCrawler;
        let err = crawler
            .crawl()
            .await
            .expect_err("pool-less crawl must fail");
        assert!(err.to_string().contains("crawl_with_pool"));
    }
}
