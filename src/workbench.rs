//! Pure helpers for the operator review workbench (SSR + WASM).

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct WorkbenchSummaryCard {
    pub label: String,
    pub count: i64,
    pub queue: Option<String>,
}

pub fn derive_selected_slug(selected: Option<&str>, slugs: &[String]) -> Option<String> {
    if let Some(s) = selected.map(str::trim).filter(|s| !s.is_empty()) {
        if slugs.iter().any(|slug| slug == s) {
            return Some(s.to_string());
        }
    }
    slugs.first().cloned()
}

pub fn build_summary_cards(
    discovered: i64,
    claim_pending: i64,
    verified_ready: i64,
    featured_queue: i64,
) -> Vec<WorkbenchSummaryCard> {
    vec![
        WorkbenchSummaryCard {
            label: "Discovered".into(),
            count: discovered,
            queue: Some("new_candidate".into()),
        },
        WorkbenchSummaryCard {
            label: "Claim Pending".into(),
            count: claim_pending,
            queue: None,
        },
        WorkbenchSummaryCard {
            label: "Verified Ready".into(),
            count: verified_ready,
            queue: None,
        },
        WorkbenchSummaryCard {
            label: "Featured Queue".into(),
            count: featured_queue,
            queue: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_tool_prefers_first_queue_item_when_query_missing() {
        let ids = vec!["bob-gateway-cli".to_string(), "zapper-mcp".to_string()];
        let selected = derive_selected_slug(None, &ids);
        assert_eq!(selected.as_deref(), Some("bob-gateway-cli"));
    }

    #[test]
    fn summary_cards_include_claim_pending_bucket() {
        let cards = build_summary_cards(24, 5, 12, 4);
        assert!(cards.iter().any(|card| card.label == "Claim Pending"));
    }
}
