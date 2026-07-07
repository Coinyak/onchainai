//! Product A — "검증된 결정 API": returns a single verified live x402 tool for a task.
//!
//! Reuses free `search_tools` (candidate extraction) + K2 `run_k2_on_demand_probe`
//! (liveness + price honesty) + trust tier ranking. Per-call x402 premium (Axis-B).
//! Spec: docs/superpowers/specs/2026-07-07-product-a-verified-api.md

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Tool;
use crate::server::queries::APPROVED_TOOLS_BY_SLUGS_SQL;
use crate::server::x402_verify::{price_matches_advertised, run_k2_on_demand_probe, ProbeOutcome};

const MAX_CANDIDATES: usize = 3;
const CACHE_TTL_SECS: i64 = 60;

#[derive(Debug)]
pub enum ProductAError {
    InvalidIntent,
    NoCandidates,
    Database(sqlx::Error),
}

impl ProductAError {
    pub fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            Self::InvalidIntent => StatusCode::BAD_REQUEST,
            Self::NoCandidates => StatusCode::NOT_FOUND,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::InvalidIntent => "intent is required and must not be empty",
            Self::NoCandidates => "no x402 candidates found for the given intent",
            Self::Database(_) => "failed to query candidates",
        }
    }
}

/// Trust tier ordering: verified > official > community.
fn trust_tier_rank(status: &str) -> i32 {
    match status {
        "verified" => 0,
        "official" => 1,
        _ => 2,
    }
}

/// Parse x402 price to a comparable f64 (lower = cheaper).
fn parse_price_usd(price: Option<&str>) -> f64 {
    let Some(price) = price else { return f64::MAX };
    let digits: String = price
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits.parse::<f64>().unwrap_or(f64::MAX)
}

/// A ranked x402 candidate with its probe outcome.
#[derive(Debug, Clone, Serialize)]
pub struct VerifiedCandidate {
    pub slug: String,
    pub name: String,
    pub tool_id: Uuid,
    pub trust_tier: String,
    pub x402_price: Option<String>,
    pub x402_endpoint: Option<String>,
    pub live: bool,
    pub price_match: bool,
    pub actual_price: Option<String>,
    pub probed_at: DateTime<Utc>,
    pub install_command: Option<String>,
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
}

/// Rejection reason for a candidate that did not pass verification.
#[derive(Debug, Clone, Serialize)]
pub struct RejectedCandidate {
    pub slug: String,
    pub name: String,
    pub trust_tier: String,
    pub reason: RejectionReason,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    Dead,
    PriceMismatch,
    Stale,
    NotRelevant,
    NotProbed,
}

/// Full Product A response.
#[derive(Debug, Clone, Serialize)]
pub struct ProductAResponse {
    pub verified_tool: Option<VerifiedCandidate>,
    pub rejected: Vec<RejectedCandidate>,
    pub disclaimer: &'static str,
    pub probed_at: DateTime<Utc>,
    pub cached: Option<DateTime<Utc>>,
}

pub const PRODUCT_A_DISCLAIMER: &str =
    "Verified at time T for endpoint liveness and advertised x402 fee match only. \
     Execution price, slippage, safety, and task correctness are NOT verified. \
     Trust tier reflects curation, not a safety guarantee.";

/// Run Product A: extract candidates, probe top N, return the best verified tool.
///
/// `candidate_slugs` are pre-extracted by the caller (from free search_tools).
/// This function handles ranking, probing, and response assembly.
pub async fn recommend_verified_tool(
    pool: &PgPool,
    candidate_slugs: &[String],
) -> Result<ProductAResponse, ProductAError> {
    if candidate_slugs.is_empty() {
        return Err(ProductAError::NoCandidates);
    }

    // Fetch candidate tools from the catalog (all must be real slugs — hallucination guard).
    let tools = sqlx::query_as::<_, Tool>(APPROVED_TOOLS_BY_SLUGS_SQL)
        .bind(candidate_slugs)
        .fetch_all(pool)
        .await
        .map_err(ProductAError::Database)?;

    if tools.is_empty() {
        return Err(ProductAError::NoCandidates);
    }

    // Filter to x402 tools with endpoints, then rank by (trust tier, x402 fee).
    let mut x402_candidates: Vec<&Tool> = tools
        .iter()
        .filter(|t| {
            (t.pricing == "x402" || t.x402_endpoint.is_some())
                && t.x402_endpoint
                    .as_deref()
                    .is_some_and(|e| !e.trim().is_empty())
        })
        .collect();

    x402_candidates.sort_by(|a, b| {
        trust_tier_rank(&a.status)
            .cmp(&trust_tier_rank(&b.status))
            .then_with(|| {
                parse_price_usd(a.x402_price.as_deref())
                    .partial_cmp(&parse_price_usd(b.x402_price.as_deref()))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let top_n: Vec<&Tool> = x402_candidates.into_iter().take(MAX_CANDIDATES).collect();

    if top_n.is_empty() {
        // Candidates existed but none had x402 endpoints.
        let rejected: Vec<RejectedCandidate> = tools
            .iter()
            .map(|t| RejectedCandidate {
                slug: t.slug.clone(),
                name: t.name.clone(),
                trust_tier: t.status.clone(),
                reason: RejectionReason::NotRelevant,
                detail: "tool is not an x402 endpoint listing".into(),
            })
            .collect();
        return Ok(ProductAResponse {
            verified_tool: None,
            rejected,
            disclaimer: PRODUCT_A_DISCLAIMER,
            probed_at: Utc::now(),
            cached: None,
        });
    }

    // Probe top N in parallel.
    let mut probe_handles = Vec::new();
    for tool in &top_n {
        let endpoint = tool
            .x402_endpoint
            .as_deref()
            .unwrap_or_default()
            .to_string();
        let price = tool.x402_price.clone();
        let tool_id = tool.id;
        let pool_clone = pool.clone();
        probe_handles.push(tokio::spawn(async move {
            run_k2_on_demand_probe(&pool_clone, tool_id, &endpoint, price.as_deref())
                .await
                .map(|run| (tool_id, run))
        }));
    }

    let mut probe_results: Vec<(Uuid, crate::server::x402_verify::ToolProbeRun)> = Vec::new();
    for handle in probe_handles {
        match handle.await {
            Ok(Ok((tool_id, run))) => probe_results.push((tool_id, run)),
            Ok(Err(e)) => {
                tracing::error!("Product A probe failed: {e}");
            }
            Err(e) => {
                tracing::error!("Product A probe task panicked: {e}");
            }
        }
    }

    // Build verified + rejected lists.
    let mut verified: Option<VerifiedCandidate> = None;
    let mut rejected: Vec<RejectedCandidate> = Vec::new();
    let now = Utc::now();

    for tool in &top_n {
        let Some((_, run)) = probe_results.iter().find(|(tid, _)| *tid == tool.id) else {
            // Probe failed entirely — treat as dead.
            rejected.push(RejectedCandidate {
                slug: tool.slug.clone(),
                name: tool.name.clone(),
                trust_tier: tool.status.clone(),
                reason: RejectionReason::Dead,
                detail: "probe failed — no response".into(),
            });
            continue;
        };

        let live = run.history_status == "live";
        let price_match = match (&run.outcome, tool.x402_price.as_deref()) {
            (ProbeOutcome::Verified { amount, .. }, Some(advertised)) => amount
                .as_deref()
                .is_some_and(|a| price_matches_advertised(a, advertised)),
            _ => false,
        };

        if live && price_match && verified.is_none() {
            verified = Some(VerifiedCandidate {
                slug: tool.slug.clone(),
                name: tool.name.clone(),
                tool_id: tool.id,
                trust_tier: tool.status.clone(),
                x402_price: tool.x402_price.clone(),
                x402_endpoint: tool.x402_endpoint.clone(),
                live: true,
                price_match: true,
                actual_price: run.actual_price.clone(),
                probed_at: run.probed_at,
                install_command: tool.install_command.clone(),
                repo_url: tool.repo_url.clone(),
                homepage: tool.homepage.clone(),
            });
        } else {
            let reason = if !live {
                RejectionReason::Dead
            } else if !price_match {
                RejectionReason::PriceMismatch
            } else {
                // Live + price_match but a higher-ranked tool was already selected.
                RejectionReason::Stale
            };
            let detail = if !live {
                format!("endpoint returned {} (not live)", run.history_status)
            } else if !price_match {
                format!(
                    "advertised {} but probe returned {}",
                    tool.x402_price.as_deref().unwrap_or("?"),
                    run.actual_price.as_deref().unwrap_or("none")
                )
            } else {
                "lower trust tier or higher fee than the selected tool".into()
            };
            rejected.push(RejectedCandidate {
                slug: tool.slug.clone(),
                name: tool.name.clone(),
                trust_tier: tool.status.clone(),
                reason,
                detail,
            });
        }
    }

    // Classify non-probed tools: x402 tools beyond MAX_CANDIDATES cap get NotProbed,
    // non-x402 tools get NotRelevant.
    let probed_ids: std::collections::HashSet<Uuid> = top_n.iter().map(|t| t.id).collect();
    for tool in &tools {
        if probed_ids.contains(&tool.id) {
            continue;
        }
        let is_x402 = (tool.pricing == "x402" || tool.x402_endpoint.is_some())
            && tool
                .x402_endpoint
                .as_deref()
                .is_some_and(|e| !e.trim().is_empty());
        if is_x402 {
            rejected.push(RejectedCandidate {
                slug: tool.slug.clone(),
                name: tool.name.clone(),
                trust_tier: tool.status.clone(),
                reason: RejectionReason::NotProbed,
                detail: format!("not probed — beyond MAX_CANDIDATES cap of {MAX_CANDIDATES}"),
            });
        } else {
            rejected.push(RejectedCandidate {
                slug: tool.slug.clone(),
                name: tool.name.clone(),
                trust_tier: tool.status.clone(),
                reason: RejectionReason::NotRelevant,
                detail: "not an x402 endpoint listing or no endpoint configured".into(),
            });
        }
    }

    Ok(ProductAResponse {
        verified_tool: verified,
        rejected,
        disclaimer: PRODUCT_A_DISCLAIMER,
        probed_at: now,
        cached: None,
    })
}

/// Validate the intent string.
pub fn validate_intent(intent: &str) -> Result<String, ProductAError> {
    let trimmed = intent.trim();
    if trimmed.is_empty() {
        return Err(ProductAError::InvalidIntent);
    }
    if trimmed.chars().count() > 500 {
        return Err(ProductAError::InvalidIntent);
    }
    Ok(trimmed.to_string())
}

/// Simple in-memory cache for repeat intents (60s TTL).
/// Keyed by intent+chain+function; stores the probed_at timestamp to indicate staleness.
use std::collections::HashMap;
use std::sync::Mutex;

static INTENT_CACHE: Mutex<Option<HashMap<String, (ProductAResponse, DateTime<Utc>)>>> =
    Mutex::new(None);

pub fn cache_key(intent: &str, chain: Option<&str>, function: Option<&str>) -> String {
    format!(
        "{intent}|chain={}|function={}",
        chain.unwrap_or(""),
        function.unwrap_or("")
    )
}

pub fn cache_get(key: &str, now: DateTime<Utc>) -> Option<ProductAResponse> {
    let guard = INTENT_CACHE.lock().ok()?;
    let cache = guard.as_ref()?;
    let (response, cached_at) = cache.get(key)?;
    if now.timestamp() - cached_at.timestamp() > CACHE_TTL_SECS {
        return None;
    }
    let mut cached = response.clone();
    cached.cached = Some(*cached_at);
    Some(cached)
}

pub fn cache_set(key: String, response: ProductAResponse, now: DateTime<Utc>) {
    if let Ok(mut guard) = INTENT_CACHE.lock() {
        if guard.is_none() {
            *guard = Some(HashMap::new());
        }
        if let Some(cache) = guard.as_mut() {
            // Evict stale entries to prevent unbounded growth.
            if cache.len() > 100 {
                cache.retain(|_, (_, at)| now.timestamp() - at.timestamp() <= CACHE_TTL_SECS);
            }
            cache.insert(key, (response, now));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_tier_rank_orders_verified_first() {
        assert_eq!(trust_tier_rank("verified"), 0);
        assert_eq!(trust_tier_rank("official"), 1);
        assert_eq!(trust_tier_rank("community"), 2);
    }

    #[test]
    fn parse_price_usd_extracts_numeric() {
        assert!((parse_price_usd(Some("$0.001/call")) - 0.001).abs() < f64::EPSILON);
        assert!((parse_price_usd(Some("$0.01")) - 0.01).abs() < f64::EPSILON);
        assert_eq!(parse_price_usd(None), f64::MAX);
    }

    #[test]
    fn validate_intent_rejects_empty() {
        assert!(matches!(
            validate_intent(""),
            Err(ProductAError::InvalidIntent)
        ));
        assert!(matches!(
            validate_intent("   "),
            Err(ProductAError::InvalidIntent)
        ));
    }

    #[test]
    fn validate_intent_rejects_over_500_chars() {
        let long = "a".repeat(501);
        assert!(matches!(
            validate_intent(&long),
            Err(ProductAError::InvalidIntent)
        ));
    }

    #[test]
    fn validate_intent_accepts_normal() {
        assert_eq!(
            validate_intent("bridge USDC to Base").unwrap(),
            "bridge USDC to Base"
        );
    }

    #[test]
    fn cache_key_includes_chain_and_function() {
        let key = cache_key("bridge", Some("base"), Some("payments"));
        assert!(key.contains("chain=base"));
        assert!(key.contains("function=payments"));
    }

    #[test]
    fn cache_set_and_get_roundtrip() {
        let key = "test_intent|chain=|function=";
        let now = Utc::now();
        let response = ProductAResponse {
            verified_tool: None,
            rejected: vec![],
            disclaimer: PRODUCT_A_DISCLAIMER,
            probed_at: now,
            cached: None,
        };
        cache_set(key.into(), response.clone(), now);
        let got = cache_get(key, now);
        assert!(got.is_some());
        assert!(got.unwrap().cached.is_some());
    }

    #[test]
    fn cache_expires_after_60s() {
        let key = "test_expire|chain=|function=";
        let now = Utc::now();
        let response = ProductAResponse {
            verified_tool: None,
            rejected: vec![],
            disclaimer: PRODUCT_A_DISCLAIMER,
            probed_at: now,
            cached: None,
        };
        cache_set(key.into(), response, now);
        let future = now + chrono::Duration::seconds(CACHE_TTL_SECS + 1);
        assert!(cache_get(key, future).is_none());
    }
}
