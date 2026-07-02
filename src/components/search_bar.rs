//! Search inputs — debounced URL sync via leptos_router.

use crate::components::tools_browser::{build_query_base, BrowserBase, BrowserQueryParams};
use crate::discovery::{parse_search_intent, search_intent_href};

use leptos::leptos_dom::helpers::TimeoutHandle;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};

const DEBOUNCE_MS: u64 = 200;

/// Cancel any pending debounce timer, then schedule `set_timeout_with_handle`.
fn schedule_debounced_set(
    timer: StoredValue<Option<TimeoutHandle>>,
    debounced: RwSignal<String>,
    val: String,
) {
    if let Some(handle) = timer.get_value() {
        handle.clear();
    }
    if let Ok(handle) = leptos::prelude::set_timeout_with_handle(
        move || debounced.set(val),
        std::time::Duration::from_millis(DEBOUNCE_MS),
    ) {
        timer.set_value(Some(handle));
    }
}

/// Hero search — debounced navigate to `/tools?q=...`.
#[component]
pub fn SearchBar() -> impl IntoView {
    let navigate = use_navigate();
    let input = RwSignal::new(String::new());
    let debounced = RwSignal::new(String::new());
    let timer = StoredValue::new(None::<TimeoutHandle>);

    Effect::new(move |_| {
        let val = input.get();
        schedule_debounced_set(timer, debounced, val);
    });

    on_cleanup(move || clear_timer(timer));

    Effect::new(move |_| {
        let q = debounced.get();
        if q.trim().is_empty() {
            return;
        }
        let url = search_intent_href("/tools", &parse_search_intent(q.trim()));
        navigate(&url, Default::default());
    });

    view! {
        <div class="w-full">
            <input
                type="search"
                placeholder="Search: asset tracking, trading, DeFi, chain name..."
                class="search-input w-full h-12 px-4 text-[14px] rounded-lg border border-[#E5E5E5] bg-white text-[#1A1A1A] outline-none focus:border-[#E76F00]"
                autocomplete="off"
                prop:value=move || input.get()
                on:input=move |ev| input.set(event_target_value(&ev))
            />
        </div>
    }
}

fn toolbar_query_params(
    base: &BrowserBase,
    query: &leptos_router::params::ParamsMap,
    q: String,
) -> BrowserQueryParams {
    let intent = parse_search_intent(&q);
    let search_q = if intent.query_terms.is_empty() {
        String::new()
    } else {
        intent.query_terms.clone()
    };
    BrowserQueryParams {
        function: match base {
            BrowserBase::Category(_) => {
                base.function_from_query(query.get("function").map(|s| s.to_string()))
            }
            _ => query
                .get("function")
                .map(|s| s.to_string())
                .or_else(|| intent.function.clone()),
        },
        asset_class: query.get("asset_class").map(|s| s.to_string()),
        actor: query.get("actor").map(|s| s.to_string()),
        tool_type: query
            .get("type")
            .map(|s| s.to_string())
            .or_else(|| intent.tool_type.clone()),
        status: query.get("status").map(|s| s.to_string()),
        pricing: query.get("pricing").map(|s| s.to_string()),
        install_risk: query
            .get("install_risk")
            .map(|s| s.to_string())
            .or_else(|| intent.install_risk.clone()),
        chain: query
            .get("chain")
            .map(|s| s.to_string())
            .or_else(|| intent.chain.clone()),
        sort: query
            .get("sort")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "hot".into()),
        search_q: Some(search_q).filter(|s| !s.is_empty()),
        selected: None,
        intent: None,
        compare_tools: query.get("compare_tools").map(|s| s.to_string()),
        page: 1,
    }
}

fn clear_timer(timer: StoredValue<Option<TimeoutHandle>>) {
    if let Some(handle) = timer.get_value() {
        handle.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos_router::params::ParamsMap;

    #[test]
    fn toolbar_search_preserves_compare_tools_but_clears_add_mode() {
        let mut query = ParamsMap::new();
        query.insert("compare_tools", "aave,uniswap".into());
        query.insert("selected", "old-tool".into());
        query.insert("intent", "add-mcp".into());
        query.insert("type", "mcp".into());

        let params = toolbar_query_params(&BrowserBase::Tools, &query, "wallet".into());

        assert_eq!(params.compare_tools.as_deref(), Some("aave,uniswap"));
        assert_eq!(params.selected, None);
        assert_eq!(params.intent, None);
        assert_eq!(params.tool_type.as_deref(), Some("mcp"));
        assert_eq!(params.search_q.as_deref(), Some("wallet"));
    }
}

/// Toolbar search — debounced, preserves active filters on same base path.
#[component]
pub fn ToolbarSearch(base: BrowserBase, initial_q: String) -> impl IntoView {
    let query = use_query_map();
    let navigate = use_navigate();
    let input = RwSignal::new(initial_q.clone());
    let debounced = RwSignal::new(initial_q);
    let timer = StoredValue::new(None::<TimeoutHandle>);
    let syncing_from_url = StoredValue::new(false);

    // Sync from URL (back/forward, filter links) — cancel pending debounce timers.
    Effect::new(move |_| {
        let q = query.with(|qm| qm.get("q").unwrap_or_default().to_string());
        if input.get_untracked() == q {
            return;
        }
        clear_timer(timer);
        syncing_from_url.set_value(true);
        input.set(q.clone());
        debounced.set(q);
        syncing_from_url.set_value(false);
    });

    // Debounce user typing only (not URL-driven sync).
    Effect::new(move |_| {
        if syncing_from_url.get_value() {
            return;
        }
        let val = input.get();
        schedule_debounced_set(timer, debounced, val);
    });

    on_cleanup(move || clear_timer(timer));

    Effect::new(move |_| {
        let q = debounced.get();
        let url =
            query.with(|qm| build_query_base(&base, &toolbar_query_params(&base, qm, q.clone())));
        let current_q = query.with_untracked(|qm| qm.get("q").unwrap_or_default().to_string());
        if q == current_q {
            return;
        }
        navigate(&url, Default::default());
    });

    view! {
        <div class="toolbar-search">
            <input
                type="search"
                placeholder="Search tools..."
                prop:value=move || input.get()
                on:input=move |ev| input.set(event_target_value(&ev))
            />
        </div>
    }
}
