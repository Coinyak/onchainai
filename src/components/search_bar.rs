//! Search inputs — debounced URL sync via leptos_router.

use crate::components::tools_browser::{build_query_base, BrowserBase};

use leptos::leptos_dom::helpers::TimeoutHandle;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};

const DEBOUNCE_MS: u64 = 350;

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

    on_cleanup(move || {
        if let Some(handle) = timer.get_value() {
            handle.clear();
        }
    });

    Effect::new(move |_| {
        let q = debounced.get();
        if q.trim().is_empty() {
            return;
        }
        let url = format!("/tools?q={}", urlencoding::encode(q.trim()));
        navigate(&url, Default::default());
    });

    view! {
        <div class="w-full">
            <input
                type="search"
                placeholder="Search crypto MCP, CLI, SDK, API tools..."
                class="search-input w-full h-12 px-4 text-[14px] rounded-lg border border-[#E5E5E5] bg-white text-[#1A1A1A] outline-none focus:border-[#E76F00] focus:ring-2 focus:ring-[#E76F00]/20"
                autocomplete="off"
                prop:value=move || input.get()
                on:input=move |ev| input.set(event_target_value(&ev))
            />
        </div>
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
        if let Some(handle) = timer.get_value() {
            handle.clear();
        }
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

    on_cleanup(move || {
        if let Some(handle) = timer.get_value() {
            handle.clear();
        }
    });

    Effect::new(move |_| {
        let q = debounced.get();
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
                Some(q.clone()).filter(|s| !s.is_empty()),
                qm.get("selected").map(|s| s.to_string()),
            )
        });
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