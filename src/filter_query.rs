//! URL query parsing and multi-select filter helpers (comma-separated values).

use crate::server::functions::ToolFilters;

/// Scalar query params — not comma-split (e.g. search text may contain commas).
const SCALAR_KEYS: &[&str] = &["q", "sort", "selected"];

fn decode_param(v: &str) -> String {
    urlencoding::decode(v)
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| v.to_string())
}

/// Parse `?function=bridge,swap` into a deduped value list.
pub fn parse_multi(raw: Option<&str>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .filter(|p| seen.insert((*p).to_string()))
            .map(str::to_string)
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

/// Encode values for a query param (`bridge,swap`).
pub fn encode_multi(values: &[String]) -> Option<String> {
    if values.is_empty() {
        None
    } else {
        Some(values.join(","))
    }
}

/// Toggle one value in a multi-select param; returns updated path+query.
pub fn toggle_multi(
    base_path: impl AsRef<str>,
    query_base: impl AsRef<str>,
    key: &str,
    value: &str,
    active: &[String],
) -> String {
    let base_path = base_path.as_ref();
    let query_base = query_base.as_ref();
    let query = query_base
        .strip_prefix(base_path)
        .unwrap_or(query_base)
        .trim_start_matches('?');

    let mut map: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for part in query.split('&').filter(|s| !s.is_empty()) {
        if let Some((k, v)) = part.split_once('=') {
            if k == key {
                continue;
            }
            let vals = if SCALAR_KEYS.contains(&k) {
                vec![decode_param(v)]
            } else {
                parse_multi(Some(v))
            };
            map.insert(k.to_string(), vals);
        }
    }

    let mut next = active.to_vec();
    if let Some(pos) = next.iter().position(|x| x == value) {
        next.remove(pos);
    } else {
        next.push(value.to_string());
        next.sort();
    }

    if !next.is_empty() {
        map.insert(key.to_string(), next);
    }

    if map.is_empty() {
        return base_path.to_string();
    }

    let parts: Vec<String> = map
        .into_iter()
        .filter_map(|(k, vals)| {
            if SCALAR_KEYS.contains(&k.as_str()) {
                vals.first()
                    .filter(|v| !v.is_empty())
                    .map(|v| format!("{k}={}", urlencoding::encode(v)))
            } else {
                encode_multi(&vals).map(|v| format!("{k}={}", urlencoding::encode(&v)))
            }
        })
        .collect();
    format!("{base_path}?{}", parts.join("&"))
}

/// Remove one multi-select axis from the current query; keeps `q`, `sort`, and other filters.
pub fn clear_axis(base_path: impl AsRef<str>, query_base: impl AsRef<str>, key: &str) -> String {
    let base_path = base_path.as_ref();
    let query_base = query_base.as_ref();
    let query = query_base
        .strip_prefix(base_path)
        .unwrap_or(query_base)
        .trim_start_matches('?');

    let mut map: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for part in query.split('&').filter(|s| !s.is_empty()) {
        if let Some((k, v)) = part.split_once('=') {
            if k == key {
                continue;
            }
            let vals = if SCALAR_KEYS.contains(&k) {
                vec![decode_param(v)]
            } else {
                parse_multi(Some(v))
            };
            map.insert(k.to_string(), vals);
        }
    }

    if map.is_empty() {
        return base_path.to_string();
    }

    let parts: Vec<String> = map
        .into_iter()
        .filter_map(|(k, vals)| {
            if SCALAR_KEYS.contains(&k.as_str()) {
                vals.first()
                    .filter(|v| !v.is_empty())
                    .map(|v| format!("{k}={}", urlencoding::encode(v)))
            } else {
                encode_multi(&vals).map(|v| format!("{k}={}", urlencoding::encode(&v)))
            }
        })
        .collect();
    format!("{base_path}?{}", parts.join("&"))
}

#[allow(clippy::too_many_arguments)]
pub fn build_tool_filters(
    function: Option<String>,
    asset_class: Option<String>,
    actor: Option<String>,
    tool_type: Option<String>,
    status: Option<String>,
    pricing: Option<String>,
    install_risk: Option<String>,
    chain: Option<String>,
) -> ToolFilters {
    ToolFilters {
        function: parse_multi(function.as_deref()),
        asset_class: parse_multi(asset_class.as_deref()),
        actor: parse_multi(actor.as_deref()),
        tool_type: parse_multi(tool_type.as_deref()),
        status: parse_multi(status.as_deref()),
        pricing: parse_multi(pricing.as_deref()),
        install_risk: parse_multi(install_risk.as_deref()),
        chain: parse_multi(chain.as_deref()),
    }
}

/// Active browser filters for empty-state copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveFiltersSummary {
    pub function: Vec<String>,
    pub asset_class: Vec<String>,
    pub actor: Vec<String>,
    pub tool_type: Vec<String>,
    pub status: Vec<String>,
    pub pricing: Vec<String>,
    pub install_risk: Vec<String>,
    pub chain: Vec<String>,
    pub search: Option<String>,
    pub sort: String,
}

impl Default for ActiveFiltersSummary {
    fn default() -> Self {
        Self {
            function: Vec::new(),
            asset_class: Vec::new(),
            actor: Vec::new(),
            tool_type: Vec::new(),
            status: Vec::new(),
            pricing: Vec::new(),
            install_risk: Vec::new(),
            chain: Vec::new(),
            search: None,
            sort: "hot".into(),
        }
    }
}

impl ActiveFiltersSummary {
    #[allow(clippy::too_many_arguments)]
    pub fn from_query(
        function: Option<String>,
        asset_class: Option<String>,
        actor: Option<String>,
        tool_type: Option<String>,
        status: Option<String>,
        pricing: Option<String>,
        install_risk: Option<String>,
        chain: Option<String>,
        search: Option<String>,
        sort: Option<String>,
    ) -> Self {
        Self {
            function: parse_multi(function.as_deref()),
            asset_class: parse_multi(asset_class.as_deref()),
            actor: parse_multi(actor.as_deref()),
            tool_type: parse_multi(tool_type.as_deref()),
            status: parse_multi(status.as_deref()),
            pricing: parse_multi(pricing.as_deref()),
            install_risk: parse_multi(install_risk.as_deref()),
            chain: parse_multi(chain.as_deref()),
            search: search.filter(|s| !s.trim().is_empty()),
            sort: sort.unwrap_or_else(|| "hot".into()),
        }
    }

    pub fn has_active_filters(&self) -> bool {
        !self.function.is_empty()
            || !self.asset_class.is_empty()
            || !self.actor.is_empty()
            || !self.tool_type.is_empty()
            || !self.status.is_empty()
            || !self.pricing.is_empty()
            || !self.install_risk.is_empty()
            || !self.chain.is_empty()
            || self.search.is_some()
            || self.sort != "hot"
    }
}

fn humanize_filter_id(id: &str) -> String {
    id.split(&['-', '_'][..])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn label_with_map(id: &str, labels: &std::collections::HashMap<String, String>) -> String {
    labels
        .get(id)
        .cloned()
        .unwrap_or_else(|| humanize_filter_id(id))
}

/// Plain-language filter lines for empty states.
pub fn describe_active_filters(
    summary: &ActiveFiltersSummary,
    function_labels: &std::collections::HashMap<String, String>,
) -> Vec<String> {
    let mut lines = Vec::new();
    if !summary.function.is_empty() {
        let labels: Vec<String> = summary
            .function
            .iter()
            .map(|id| label_with_map(id, function_labels))
            .collect();
        lines.push(format!("Function: {}", labels.join(", ")));
    }
    if !summary.asset_class.is_empty() {
        let labels: Vec<String> = summary
            .asset_class
            .iter()
            .map(|id| humanize_filter_id(id))
            .collect();
        lines.push(format!("Asset class: {}", labels.join(", ")));
    }
    if !summary.actor.is_empty() {
        let labels: Vec<String> = summary
            .actor
            .iter()
            .map(|id| humanize_filter_id(id))
            .collect();
        lines.push(format!("Actor: {}", labels.join(", ")));
    }
    if !summary.tool_type.is_empty() {
        let labels: Vec<String> = summary
            .tool_type
            .iter()
            .map(|id| id.to_uppercase())
            .collect();
        lines.push(format!("Type: {}", labels.join(", ")));
    }
    if !summary.status.is_empty() {
        let labels: Vec<String> = summary
            .status
            .iter()
            .map(|id| humanize_filter_id(id))
            .collect();
        lines.push(format!("Status: {}", labels.join(", ")));
    }
    if !summary.pricing.is_empty() {
        let labels: Vec<String> = summary
            .pricing
            .iter()
            .map(|id| humanize_filter_id(id))
            .collect();
        lines.push(format!("Pricing: {}", labels.join(", ")));
    }
    if !summary.install_risk.is_empty() {
        let labels: Vec<String> = summary
            .install_risk
            .iter()
            .map(|id| humanize_filter_id(id))
            .collect();
        lines.push(format!("Install risk: {}", labels.join(", ")));
    }
    if !summary.chain.is_empty() {
        let labels: Vec<String> = summary.chain.iter().map(|id| id.to_uppercase()).collect();
        lines.push(format!("Chain: {}", labels.join(", ")));
    }
    if let Some(q) = &summary.search {
        lines.push(format!("Search: \"{q}\""));
    }
    if summary.sort != "hot" {
        let sort_label = match summary.sort.as_str() {
            "new" => "New",
            "comments" => "Comments",
            other => other,
        };
        lines.push(format!("Sort: {sort_label}"));
    }
    lines
}

/// Returns true when a same-app route change should close the mobile filter overlay.
pub fn should_collapse_mobile_sidebar_on_route_change(
    prev_route: &str,
    next_route: &str,
    mobile_viewport: bool,
) -> bool {
    mobile_viewport && !prev_route.is_empty() && prev_route != next_route
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_multi_splits_commas() {
        assert_eq!(
            parse_multi(Some("bridge,swap,lending")),
            vec!["bridge", "swap", "lending"]
        );
    }

    #[test]
    fn toggle_multi_adds_and_removes() {
        let base = "/tools?sort=hot";
        let added = toggle_multi("/tools", base, "function", "swap", &[]);
        assert!(added.contains("function=swap"));

        let removed = toggle_multi("/tools", &added, "function", "swap", &["swap".into()]);
        assert!(!removed.contains("function=swap"));
    }

    #[test]
    fn toggle_multi_keeps_other_params() {
        let url = toggle_multi(
            "/tools",
            "/tools?function=bridge&sort=new",
            "function",
            "swap",
            &["bridge".into()],
        );
        assert!(
            url.contains("function=bridge%2Cswap")
                || url.contains("function=swap%2Cbridge")
                || url.contains("function=bridge,swap")
                || url.contains("function=swap,bridge")
        );
        assert!(url.contains("sort=new"));
    }

    #[test]
    fn parse_multi_dedupes() {
        assert_eq!(
            parse_multi(Some("bridge,bridge,swap")),
            vec!["bridge", "swap"]
        );
    }

    #[test]
    fn toggle_multi_preserves_q_with_commas() {
        let url = toggle_multi(
            "/tools",
            "/tools?q=foo%2Cbar&function=bridge",
            "function",
            "swap",
            &["bridge".into()],
        );
        assert!(
            url.contains("q=foo%2Cbar") || url.contains("q=foo,bar"),
            "q not preserved: {url}"
        );
        assert!(url.contains("swap"), "swap not in url: {url}");
        assert!(url.contains("bridge"), "bridge not in url: {url}");
    }

    #[test]
    fn clear_axis_removes_only_target() {
        let href = clear_axis(
            "/tools",
            "/tools?function=bridge&sort=new&q=test",
            "function",
        );
        assert!(!href.contains("function="));
        assert!(href.contains("sort=new"));
        assert!(href.contains("q=test"));
    }

    #[test]
    fn should_collapse_mobile_sidebar_on_route_change_only_for_mobile_nav() {
        assert!(!should_collapse_mobile_sidebar_on_route_change(
            "", "/tools", true
        ));
        assert!(!should_collapse_mobile_sidebar_on_route_change(
            "/tools", "/tools", true
        ));
        assert!(should_collapse_mobile_sidebar_on_route_change(
            "/tools",
            "/tools?function=bridge",
            true
        ));
        assert!(!should_collapse_mobile_sidebar_on_route_change(
            "/tools",
            "/tools?function=bridge",
            false
        ));
    }

    #[test]
    fn active_filters_summary_detects_filters() {
        let summary = ActiveFiltersSummary::from_query(
            Some("bridge".into()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(summary.has_active_filters());

        let empty = ActiveFiltersSummary::default();
        assert!(!empty.has_active_filters());
    }

    #[test]
    fn describe_active_filters_uses_labels() {
        let summary = ActiveFiltersSummary::from_query(
            Some("bridge,swap".into()),
            Some("crypto".into()),
            None,
            Some("mcp".into()),
            None,
            Some("x402".into()),
            Some("low".into()),
            Some("eth".into()),
            Some("wallet".into()),
            Some("new".into()),
        );
        let mut labels = std::collections::HashMap::new();
        labels.insert("bridge".into(), "Bridge".into());
        labels.insert("swap".into(), "Swap".into());
        let lines = describe_active_filters(&summary, &labels);
        assert!(lines.iter().any(|l| l.contains("Function: Bridge, Swap")));
        assert!(lines.iter().any(|l| l.contains("Asset class: Crypto")));
        assert!(lines.iter().any(|l| l.contains("Type: MCP")));
        assert!(lines.iter().any(|l| l.contains("Pricing: X402")));
        assert!(lines.iter().any(|l| l.contains("Install risk: Low")));
        assert!(lines.iter().any(|l| l.contains("Chain: ETH")));
        assert!(lines.iter().any(|l| l.contains("Search: \"wallet\"")));
        assert!(lines.iter().any(|l| l.contains("Sort: New")));
    }
}
