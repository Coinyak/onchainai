//! Public discovery helpers: intent parsing, finder URLs, compare limits, and empty recovery.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchIntent {
    pub query_terms: String,
    pub function: Option<String>,
    pub chain: Option<String>,
    pub tool_type: Option<String>,
    pub install_risk: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FinderSafety {
    LowRiskOnly,
    VerifiedPreferred,
    #[default]
    ExcludeCritical,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolFinderAnswers {
    pub function: Option<String>,
    pub chain: Option<String>,
    pub tool_type: Option<String>,
    pub safety: FinderSafety,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EmptyRecoverySummary {
    pub chain: Vec<String>,
    pub tool_type: Vec<String>,
    pub install_risk: Vec<String>,
    pub search: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmptyStateSuggestion {
    pub label: &'static str,
    pub href: String,
}

fn chain_id(token: &str) -> Option<&'static str> {
    match token {
        "base" => Some("base"),
        "ethereum" | "eth" => Some("ethereum"),
        "solana" | "sol" => Some("solana"),
        "bitcoin" | "btc" => Some("bitcoin"),
        "arbitrum" | "arb" => Some("arbitrum"),
        "optimism" | "op" => Some("optimism"),
        "polygon" | "matic" => Some("polygon"),
        _ => None,
    }
}

fn type_id(token: &str) -> Option<&'static str> {
    match token {
        "mcp" => Some("mcp"),
        "cli" => Some("cli"),
        "sdk" => Some("sdk"),
        "api" => Some("api"),
        "x402" => Some("x402"),
        _ => None,
    }
}

fn function_id(token: &str) -> Option<&'static str> {
    match token {
        "bridge" => Some("bridge"),
        "swap" | "trading" | "trade" => Some("trading-swap"),
        "payment" | "payments" => Some("payments-x402"),
        "agent" | "agents" => Some("ai-agent"),
        "data" | "indexing" | "indexer" => Some("data-indexing"),
        _ => None,
    }
}

fn encode(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

fn push_param(parts: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|v| !v.trim().is_empty()) {
        parts.push(format!("{key}={}", encode(value)));
    }
}

pub fn parse_search_intent(query: &str) -> SearchIntent {
    let mut intent = SearchIntent::default();
    let mut query_terms: Vec<String> = Vec::new();
    let words: Vec<String> = query
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-')
                .to_ascii_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect();

    let mut skip_next = false;
    for (idx, token) in words.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        let next = words.get(idx + 1).map(String::as_str);
        if token == "low" && next == Some("risk") {
            intent.install_risk = Some("low".into());
            skip_next = true;
            continue;
        }
        if token == "medium" && next == Some("risk") {
            intent.install_risk = Some("medium".into());
            skip_next = true;
            continue;
        }
        if token == "high" && next == Some("risk") {
            intent.install_risk = Some("high".into());
            skip_next = true;
            continue;
        }
        if let Some(chain) = chain_id(token) {
            intent.chain.get_or_insert_with(|| chain.to_string());
            continue;
        }
        if let Some(tool_type) = type_id(token) {
            intent
                .tool_type
                .get_or_insert_with(|| tool_type.to_string());
            if tool_type == "x402" {
                continue;
            }
        }
        if let Some(function) = function_id(token) {
            intent.function.get_or_insert_with(|| function.to_string());
        }
        query_terms.push(token.clone());
    }

    intent.query_terms = query_terms.join(" ");
    intent
}

pub fn search_intent_href(base_path: &str, intent: &SearchIntent) -> String {
    let mut parts = Vec::new();
    push_param(&mut parts, "function", intent.function.as_deref());
    push_param(&mut parts, "chain", intent.chain.as_deref());
    push_param(&mut parts, "type", intent.tool_type.as_deref());
    push_param(&mut parts, "install_risk", intent.install_risk.as_deref());
    push_param(&mut parts, "q", Some(intent.query_terms.as_str()));

    if parts.is_empty() {
        base_path.to_string()
    } else {
        format!("{base_path}?{}", parts.join("&"))
    }
}

pub fn tool_finder_href(answers: &ToolFinderAnswers) -> String {
    let mut parts = Vec::new();
    push_param(&mut parts, "function", answers.function.as_deref());
    push_param(&mut parts, "chain", answers.chain.as_deref());
    push_param(&mut parts, "type", answers.tool_type.as_deref());
    match answers.safety {
        FinderSafety::LowRiskOnly => push_param(&mut parts, "install_risk", Some("low")),
        FinderSafety::VerifiedPreferred => {
            push_param(&mut parts, "status", Some("verified,official"))
        }
        FinderSafety::ExcludeCritical => {}
    }

    if parts.is_empty() {
        "/tools".into()
    } else {
        format!("/tools?{}", parts.join("&"))
    }
}

pub fn normalize_compare_slugs(raw: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    raw.split(',')
        .filter_map(|part| urlencoding::decode(part.trim()).ok())
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| !part.is_empty())
        .filter(|part| seen.insert(part.clone()))
        .take(3)
        .collect()
}

pub fn compare_href(slugs: &[String]) -> String {
    if slugs.is_empty() {
        "/compare".into()
    } else {
        format!("/compare?tools={}", encode(&slugs.join(",")))
    }
}

pub fn empty_state_suggestions(
    base_path: &str,
    summary: &EmptyRecoverySummary,
) -> Vec<EmptyStateSuggestion> {
    let mut suggestions = Vec::new();
    if !summary.chain.is_empty() {
        suggestions.push(EmptyStateSuggestion {
            label: "Remove chain filter",
            href: base_path.to_string(),
        });
    }
    if !summary.tool_type.is_empty() {
        suggestions.push(EmptyStateSuggestion {
            label: "Show all types",
            href: base_path.to_string(),
        });
    }
    if summary.install_risk.iter().any(|risk| risk == "low") {
        suggestions.push(EmptyStateSuggestion {
            label: "Include medium risk",
            href: format!("{base_path}?install_risk=low%2Cmedium"),
        });
    }
    if let Some(search) = summary.search.as_deref().filter(|q| !q.trim().is_empty()) {
        suggestions.push(EmptyStateSuggestion {
            label: "Search all tools for this keyword",
            href: format!("{base_path}?q={}", encode(search)),
        });
    }
    if suggestions.is_empty() {
        suggestions.push(EmptyStateSuggestion {
            label: "Browse all tools",
            href: base_path.to_string(),
        });
    }
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_intent_maps_high_confidence_tokens() {
        let intent = parse_search_intent("base wallet mcp");
        assert_eq!(intent.chain.as_deref(), Some("base"));
        assert_eq!(intent.tool_type.as_deref(), Some("mcp"));
        assert_eq!(intent.query_terms, "wallet mcp");

        let href = search_intent_href("/tools", &intent);
        assert_eq!(href, "/tools?chain=base&type=mcp&q=wallet%20mcp");
    }

    #[test]
    fn search_intent_maps_risk_and_x402() {
        let intent = parse_search_intent("low risk x402");
        assert_eq!(intent.tool_type.as_deref(), Some("x402"));
        assert_eq!(intent.install_risk.as_deref(), Some("low"));
        assert_eq!(
            search_intent_href("/tools", &intent),
            "/tools?type=x402&install_risk=low"
        );
    }

    #[test]
    fn finder_answers_generate_existing_filter_urls() {
        let answers = ToolFinderAnswers {
            function: Some("bridge".into()),
            chain: Some("base".into()),
            tool_type: Some("mcp".into()),
            safety: FinderSafety::LowRiskOnly,
        };
        assert_eq!(
            tool_finder_href(&answers),
            "/tools?function=bridge&chain=base&type=mcp&install_risk=low"
        );
    }

    #[test]
    fn compare_slugs_are_deduped_and_limited() {
        assert_eq!(
            normalize_compare_slugs("zapper,bob,zapper,third,fourth"),
            vec!["zapper", "bob", "third"]
        );
    }

    #[test]
    fn empty_state_never_suggests_stricter_filters() {
        let summary = EmptyRecoverySummary {
            chain: vec!["base".into()],
            tool_type: vec!["mcp".into()],
            install_risk: vec!["low".into()],
            search: Some("wallet".into()),
        };
        let suggestions = empty_state_suggestions("/tools", &summary);
        let labels: Vec<_> = suggestions.iter().map(|s| s.label).collect();
        assert!(labels.contains(&"Remove chain filter"));
        assert!(labels.contains(&"Show all types"));
        assert!(labels.contains(&"Include medium risk"));
        assert!(labels.contains(&"Search all tools for this keyword"));
        assert!(suggestions
            .iter()
            .all(|s| !s.href.contains("install_risk=critical")));
    }
}
