//! S0 gap_audit — intent → subgoal decomposition → catalog gap analysis.
//!
//! Decomposes a natural-language intent into subgoals, maps each to catalog
//! tools via free `search_tools`, and surfaces gaps (subgoals with no catalog
//! coverage). Per-call x402 premium (Axis-B).
//! Spec: docs/superpowers/specs/2026-07-07-s-group-strategy-memo.md §S.2

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::server::mcp_search::{mcp_search_tools, McpSearchSort};
use crate::server::tool_categories::PUBLIC_TOOL_CATEGORY_IDS;

const MAX_SUBGOALS: usize = 8;
const CANDIDATE_LIMIT: i64 = 5;

#[derive(Debug)]
pub enum GapAuditError {
    InvalidIntent,
    Database(String),
}

impl GapAuditError {
    pub fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            Self::InvalidIntent => StatusCode::BAD_REQUEST,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::InvalidIntent => "intent is required and must not be empty",
            Self::Database(_) => "failed to query catalog",
        }
    }
}

/// A decomposed subgoal from the intent.
#[derive(Debug, Clone, Serialize)]
pub struct Subgoal {
    pub label: String,
    pub function: Option<String>,
    pub keywords: String,
}

/// Result of mapping a subgoal to the catalog.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status")]
pub enum SubgoalResult {
    /// Catalog has tools for this subgoal.
    #[serde(rename = "covered")]
    Covered {
        candidates: Vec<String>,
        candidate_count: i64,
    },
    /// No tools found in the catalog for this subgoal.
    #[serde(rename = "gap")]
    Gap { note: String },
}

/// Full gap_audit response.
#[derive(Debug, Clone, Serialize)]
pub struct GapAuditResponse {
    pub intent: String,
    pub subgoals: Vec<GapAuditSubgoal>,
    pub gap_count: usize,
    pub covered_count: usize,
    pub disclaimer: &'static str,
    pub audited_at: DateTime<Utc>,
    pub cached: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GapAuditSubgoal {
    pub subgoal: Subgoal,
    pub result: SubgoalResult,
}

pub const GAP_AUDIT_DISCLAIMER: &str =
    "Catalog coverage analysis at time T. Gaps indicate the current OnchainAI catalog \
     does not list tools for that subgoal — manual research may be needed. \
     Tool citations are real catalog slugs. This is not financial advice; \
     verify all tools independently before use.";

/// Keyword → function mapping for subgoal decomposition.
fn keyword_to_function(keyword: &str) -> Option<&'static str> {
    let lower = keyword.to_lowercase();
    let rules: &[(&str, &str)] = &[
        ("bridge", "bridge"),
        ("cross-chain", "bridge"),
        ("cross chain", "bridge"),
        ("swap", "swap"),
        ("dex", "swap"),
        ("exchange", "swap"),
        ("wallet", "wallet"),
        ("custody", "wallet"),
        ("sign", "wallet"),
        ("payment", "payments"),
        ("pay", "payments"),
        ("x402", "payments"),
        ("usdc", "payments"),
        ("lend", "lending"),
        ("borrow", "lending"),
        ("loan", "lending"),
        ("stake", "staking"),
        ("staking", "staking"),
        ("yield", "staking"),
        ("restake", "staking"),
        ("trade", "trading"),
        ("trading", "trading"),
        ("perp", "trading"),
        ("futures", "trading"),
        ("nft", "nft"),
        ("mint", "nft"),
        ("data", "data"),
        ("price", "data"),
        ("analytics", "data"),
        ("oracle", "data"),
        ("rpc", "dev-tool"),
        ("sdk", "dev-tool"),
        ("contract", "dev-tool"),
        ("debug", "dev-tool"),
        ("identity", "identity"),
        ("kya", "identity"),
        ("attestation", "identity"),
        ("vote", "governance"),
        ("dao", "governance"),
        ("governance", "governance"),
        ("social", "social"),
        ("content", "social"),
        ("agent", "ai-agent"),
        ("autonomous", "ai-agent"),
        ("ai", "ai-agent"),
    ];
    for (pattern, function) in rules {
        if lower.contains(pattern) {
            return Some(function);
        }
    }
    None
}

/// Decompose an intent into subgoals using keyword matching.
/// This is a rule-based decomposition (no LLM) — extracts action verbs and
/// maps them to function categories.
pub fn decompose_intent(intent: &str) -> Vec<Subgoal> {
    let lower = intent.to_lowercase();
    let mut subgoals: Vec<Subgoal> = Vec::new();
    let mut seen_functions: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Split on common separators: "→", "->", "then", "and", ",", ";"
    let parts: Vec<&str> = lower
        .split(|c| c == '→' || c == ',' || c == ';' || c == '\n')
        .flat_map(|s| s.split("->"))
        .flat_map(|s| s.split(" then "))
        .flat_map(|s| s.split(" and "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    for part in parts {
        let function = keyword_to_function(part);
        let keywords = part.to_string();

        // Deduplicate by function — avoid 5 "bridge" subgoals from one intent.
        let func_key = function.unwrap_or(&keywords).to_string();
        if seen_functions.contains(&func_key) {
            continue;
        }
        seen_functions.insert(func_key);

        subgoals.push(Subgoal {
            label: part.to_string(),
            function: function.map(String::from),
            keywords,
        });

        if subgoals.len() >= MAX_SUBGOALS {
            break;
        }
    }

    // Fallback: if no subgoals were extracted, use the full intent as one subgoal.
    if subgoals.is_empty() {
        let function = keyword_to_function(&lower);
        subgoals.push(Subgoal {
            label: intent.trim().to_string(),
            function: function.map(String::from),
            keywords: lower,
        });
    }

    subgoals
}

/// Run gap_audit: decompose intent, search catalog per subgoal, surface gaps.
pub async fn run_gap_audit(pool: &PgPool, intent: &str) -> Result<GapAuditResponse, GapAuditError> {
    let trimmed = intent.trim();
    if trimmed.is_empty() {
        return Err(GapAuditError::InvalidIntent);
    }
    if trimmed.chars().count() > 500 {
        return Err(GapAuditError::InvalidIntent);
    }

    let subgoals = decompose_intent(trimmed);
    let mut results: Vec<GapAuditSubgoal> = Vec::new();
    let mut gap_count = 0usize;
    let mut covered_count = 0usize;

    for subgoal in subgoals {
        let search_query = &subgoal.keywords;
        let category = subgoal.function.as_deref().and_then(|f| {
            if PUBLIC_TOOL_CATEGORY_IDS.contains(&f) {
                Some(f.to_string())
            } else {
                None
            }
        });

        let page = mcp_search_tools(
            pool,
            search_query,
            category.map(String::from),
            None,
            McpSearchSort::Trust,
            CANDIDATE_LIMIT,
            0,
        )
        .await
        .map_err(|e| GapAuditError::Database(e.1))?;

        if page.tools.is_empty() {
            gap_count += 1;
            results.push(GapAuditSubgoal {
                subgoal,
                result: SubgoalResult::Gap {
                    note: "No tools found in the OnchainAI catalog for this subgoal — manual research needed".into(),
                },
            });
        } else {
            covered_count += 1;
            let candidates: Vec<String> = page.tools.iter().map(|t| t.slug.clone()).collect();
            let candidate_count = page.total_count;
            results.push(GapAuditSubgoal {
                subgoal,
                result: SubgoalResult::Covered {
                    candidates,
                    candidate_count,
                },
            });
        }
    }

    Ok(GapAuditResponse {
        intent: trimmed.to_string(),
        subgoals: results,
        gap_count,
        covered_count,
        disclaimer: GAP_AUDIT_DISCLAIMER,
        audited_at: Utc::now(),
        cached: None,
    })
}

/// Validate intent for the gap_audit tool.
pub fn validate_gap_audit_intent(intent: &str) -> Result<String, GapAuditError> {
    let trimmed = intent.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 500 {
        return Err(GapAuditError::InvalidIntent);
    }
    Ok(trimmed.to_string())
}

/// Simple in-memory cache for repeat intents (60s TTL).
use std::collections::HashMap;
use std::sync::Mutex;

static GAP_CACHE: Mutex<Option<HashMap<String, (GapAuditResponse, DateTime<Utc>)>>> =
    Mutex::new(None);

const CACHE_TTL_SECS: i64 = 60;
const CACHE_MAX_ENTRIES: usize = 100;

fn cache_entry_millis(at: &DateTime<Utc>) -> i64 {
    at.timestamp_millis()
}

fn trim_gap_cache(
    cache: &mut HashMap<String, (GapAuditResponse, DateTime<Utc>)>,
    now: DateTime<Utc>,
) {
    let now_ms = cache_entry_millis(&now);
    let ttl_ms = CACHE_TTL_SECS * 1000;
    cache.retain(|_, (_, at)| now_ms - cache_entry_millis(at) <= ttl_ms);
    while cache.len() > CACHE_MAX_ENTRIES {
        let Some(oldest_key) = cache
            .iter()
            .min_by_key(|(key, (_, at))| (cache_entry_millis(at), key.clone()))
            .map(|(key, _)| key.clone())
        else {
            break;
        };
        cache.remove(&oldest_key);
    }
}

pub fn gap_cache_key(intent: &str) -> String {
    format!("gap|{intent}")
}

pub fn gap_cache_get(key: &str, now: DateTime<Utc>) -> Option<GapAuditResponse> {
    let guard = GAP_CACHE.lock().ok()?;
    let cache = guard.as_ref()?;
    let (response, cached_at) = cache.get(key)?;
    if now.timestamp_millis() - cached_at.timestamp_millis() > CACHE_TTL_SECS * 1000 {
        return None;
    }
    let mut cached = response.clone();
    cached.cached = Some(*cached_at);
    Some(cached)
}

pub fn gap_cache_set(key: String, response: GapAuditResponse, now: DateTime<Utc>) {
    if let Ok(mut guard) = GAP_CACHE.lock() {
        if guard.is_none() {
            *guard = Some(HashMap::new());
        }
        if let Some(cache) = guard.as_mut() {
            trim_gap_cache(cache, now);
            cache.insert(key, (response, now));
            trim_gap_cache(cache, now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompose_splits_on_arrows_and_conjunctions() {
        let subgoals = decompose_intent("bridge BTC to Base then swap to USDC and stake");
        assert!(subgoals.len() >= 2);
        assert!(subgoals.iter().any(|s| s.function == Some("bridge".into())));
        assert!(subgoals.iter().any(|s| s.function == Some("swap".into())));
        assert!(subgoals
            .iter()
            .any(|s| s.function == Some("staking".into())));
    }

    #[test]
    fn decompose_deduplicates_by_function() {
        let subgoals = decompose_intent("bridge BTC and bridge ETH and bridge SOL");
        // All three are "bridge" function — should deduplicate to 1.
        assert_eq!(subgoals.len(), 1);
    }

    #[test]
    fn decompose_fallback_when_no_keywords() {
        let subgoals = decompose_intent("do something cool");
        assert_eq!(subgoals.len(), 1);
        assert!(subgoals[0].function.is_none());
    }

    #[test]
    fn decompose_caps_at_max_subgoals() {
        let intent = "bridge, swap, wallet, payments, lending, staking, trading, nft, data, dev-tool, identity";
        let subgoals = decompose_intent(intent);
        assert!(subgoals.len() <= MAX_SUBGOALS);
    }

    #[test]
    fn keyword_to_function_maps_common_actions() {
        assert_eq!(keyword_to_function("bridge USDC"), Some("bridge"));
        assert_eq!(keyword_to_function("swap tokens"), Some("swap"));
        assert_eq!(keyword_to_function("stake SOL"), Some("staking"));
        assert_eq!(keyword_to_function("unknown action"), None);
    }

    #[test]
    fn validate_rejects_empty() {
        assert!(matches!(
            validate_gap_audit_intent(""),
            Err(GapAuditError::InvalidIntent)
        ));
    }

    #[test]
    fn validate_rejects_over_500_chars() {
        let long = "a".repeat(501);
        assert!(matches!(
            validate_gap_audit_intent(&long),
            Err(GapAuditError::InvalidIntent)
        ));
    }

    #[test]
    fn gap_cache_roundtrip() {
        let key = "gap|roundtrip_isolated";
        let now = Utc::now();
        let response = GapAuditResponse {
            intent: "test".into(),
            subgoals: vec![],
            gap_count: 0,
            covered_count: 0,
            disclaimer: GAP_AUDIT_DISCLAIMER,
            audited_at: now,
            cached: None,
        };
        gap_cache_set(key.into(), response, now);
        assert!(gap_cache_get(key, now).is_some());
    }

    #[test]
    fn gap_cache_enforces_max_entries() {
        let now = Utc::now();
        let response = GapAuditResponse {
            intent: "test".into(),
            subgoals: vec![],
            gap_count: 0,
            covered_count: 0,
            disclaimer: GAP_AUDIT_DISCLAIMER,
            audited_at: now,
            cached: None,
        };
        for i in 0..105 {
            let ts = now + chrono::Duration::milliseconds(i);
            gap_cache_set(format!("gap|maxtest-{i}"), response.clone(), ts);
        }
        let probe_at = now + chrono::Duration::seconds(1);
        assert!(gap_cache_get("gap|maxtest-0", probe_at).is_none());
        assert!(gap_cache_get("gap|maxtest-4", probe_at).is_none());
        assert!(gap_cache_get("gap|maxtest-5", probe_at).is_some());
        assert!(gap_cache_get("gap|maxtest-104", probe_at).is_some());
        // Clean up: clear maxtest entries to avoid interfering with other cache tests.
        if let Ok(mut guard) = GAP_CACHE.lock() {
            if let Some(cache) = guard.as_mut() {
                cache.retain(|key, _| !key.starts_with("gap|maxtest-"));
            }
        }
    }

    #[test]
    fn gap_cache_expires_after_60s() {
        let key = "gap|expire_unique";
        let now = Utc::now();
        let response = GapAuditResponse {
            intent: "test".into(),
            subgoals: vec![],
            gap_count: 0,
            covered_count: 0,
            disclaimer: GAP_AUDIT_DISCLAIMER,
            audited_at: now,
            cached: None,
        };
        gap_cache_set(key.into(), response, now);
        let future = now + chrono::Duration::seconds(CACHE_TTL_SECS + 1);
        assert!(gap_cache_get(key, future).is_none());
    }
}
