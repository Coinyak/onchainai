//! Search inputs — debounced URL sync via leptos_router.

use crate::components::tools_browser::{build_query_base, BrowserBase};

use leptos::leptos_dom::helpers::debounce;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};

const DEBOUNCE_MS: u64 = 350;

/// Hero search — debounced navigate to `/tools?q=...`.
#[component]
pub fn SearchBar() -> impl IntoView {
    let navigate = use_navigate();
    let input = RwSignal::new(String::new());

    let mut debounced_navigate = debounce(
        std::time::Duration::from_millis(DEBOUNCE_MS),
        move |val: String| {
            if val.trim().is_empty() {
                return;
            }
            let url = format!("/tools?q={}", urlencoding::encode(val.trim()));
            navigate(&url, Default::default());
        },
    );

    view! {
        <div class="w-full">
            <input
                type="search"
                placeholder="Search crypto MCP, CLI, SDK, API tools..."
                class="search-input w-full h-12 px-4 text-[14px] rounded-lg border border-[#E5E5E5] bg-white text-[#1A1A1A] outline-none focus:border-[#E76F00] focus:ring-2 focus:ring-[#E76F00]/20"
                autocomplete="off"
                prop:value=move || input.get()
                on:input=move |ev| {
                    let val = event_target_value(&ev);
                    input.set(val.clone());
                    debounced_navigate(val);
                }
            />
        </div>
    }
}

/// Toolbar search — debounced, preserves active filters on same base path.
#[component]
pub fn ToolbarSearch(base: BrowserBase) -> impl IntoView {
    let query = use_query_map();
    let navigate = use_navigate();
    let input = RwSignal::new(String::new());

    Effect::new(move |_| {
        let q = query.with(|qm| qm.get("q").unwrap_or_default().to_string());
        input.set(q);
    });

    let mut debounced_navigate = debounce(
        std::time::Duration::from_millis(DEBOUNCE_MS),
        move |val: String| {
            let url = query.with(|qm| {
                build_query_base(
                    base,
                    qm.get("function").map(|s| s.to_string()),
                    qm.get("asset_class").map(|s| s.to_string()),
                    qm.get("actor").map(|s| s.to_string()),
                    qm.get("type").map(|s| s.to_string()),
                    qm.get("status").map(|s| s.to_string()),
                    qm.get("chain").map(|s| s.to_string()),
                    qm.get("sort").map(|s| s.to_string()).unwrap_or_else(|| "hot".into()),
                    Some(val.clone()).filter(|s| !s.is_empty()),
                    qm.get("selected").map(|s| s.to_string()),
                )
            });
            let current_q = query.with_untracked(|qm| qm.get("q").unwrap_or_default().to_string());
            if val == current_q {
                return;
            }
            navigate(&url, Default::default());
        },
    );

    view! {
        <div class="toolbar-search">
            <input
                type="search"
                placeholder="Search tools..."
                prop:value=move || input.get()
                on:input=move |ev| {
                    let val = event_target_value(&ev);
                    input.set(val.clone());
                    debounced_navigate(val);
                }
            />
        </div>
    }
}