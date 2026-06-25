//! URL query parsing and multi-select filter helpers (comma-separated values).

use crate::server::functions::ToolFilters;

/// Parse `?function=bridge,swap` into a deduped value list.
pub fn parse_multi(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
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
            if k != key {
                map.insert(k.to_string(), parse_multi(Some(v)));
            }
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
        .filter_map(|(k, vals)| encode_multi(&vals).map(|v| format!("{k}={v}")))
        .collect();
    format!("{base_path}?{}", parts.join("&"))
}

pub fn build_tool_filters(
    function: Option<String>,
    asset_class: Option<String>,
    actor: Option<String>,
    tool_type: Option<String>,
    status: Option<String>,
    chain: Option<String>,
) -> ToolFilters {
    ToolFilters {
        function: parse_multi(function.as_deref()),
        asset_class: parse_multi(asset_class.as_deref()),
        actor: parse_multi(actor.as_deref()),
        tool_type: parse_multi(tool_type.as_deref()),
        status: parse_multi(status.as_deref()),
        chain: parse_multi(chain.as_deref()),
    }
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
        let url = toggle_multi("/tools", "/tools?function=bridge&sort=new", "function", "swap", &[
            "bridge".into(),
        ]);
        assert!(url.contains("function=bridge,swap") || url.contains("function=swap,bridge"));
        assert!(url.contains("sort=new"));
    }
}