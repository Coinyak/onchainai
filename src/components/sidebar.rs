//! Filter sidebar — six axes with URL query sync + collapsible sections.

use crate::components::tools_browser::BrowserBase;
use crate::models::Category;
use leptos::prelude::*;
use leptos_router::components::A;
use std::collections::HashMap;

struct FilterOption {
    id: &'static str,
    label: &'static str,
}

const ASSET_CLASSES: &[FilterOption] = &[
    FilterOption { id: "crypto", label: "Crypto" },
    FilterOption { id: "stablecoins", label: "Stablecoins" },
    FilterOption { id: "derivatives", label: "Derivatives" },
    FilterOption { id: "rwa", label: "RWA" },
];

const ACTORS: &[FilterOption] = &[
    FilterOption { id: "human", label: "Human" },
    FilterOption { id: "ai-agent", label: "AI Agent" },
];

const TYPES: &[FilterOption] = &[
    FilterOption { id: "mcp", label: "MCP" },
    FilterOption { id: "cli", label: "CLI" },
    FilterOption { id: "sdk", label: "SDK" },
    FilterOption { id: "api", label: "API" },
    FilterOption { id: "x402", label: "x402" },
    FilterOption { id: "skill", label: "Skill" },
];

const STATUSES: &[FilterOption] = &[
    FilterOption { id: "community", label: "Community" },
    FilterOption { id: "verified", label: "Verified" },
    FilterOption { id: "official", label: "Official" },
];

fn default_section_state() -> HashMap<String, bool> {
    [
        ("function", true),
        ("asset_class", true),
        ("actor", true),
        ("type", true),
        ("status", true),
        ("chain", true),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

fn link_class(active: bool) -> &'static str {
    if active {
        "sidebar-link active"
    } else {
        "sidebar-link"
    }
}

fn toggle_param(base_path: &str, base: &str, key: &str, value: &str, current: Option<&str>) -> String {
    let query = if base.starts_with(base_path) {
        base.strip_prefix(base_path).unwrap_or("").trim_start_matches('?')
    } else {
        base.trim_start_matches('?')
    };
    let mut pairs: Vec<(String, String)> = Vec::new();
    for part in query.split('&').filter(|s| !s.is_empty()) {
        if let Some((k, v)) = part.split_once('=') {
            if k != key {
                pairs.push((k.to_string(), v.to_string()));
            }
        }
    }
    let is_active = current == Some(value);
    if !is_active {
        pairs.push((key.to_string(), value.to_string()));
    }
    if pairs.is_empty() {
        base_path.to_string()
    } else {
        format!(
            "{}?{}",
            base_path,
            pairs
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&")
        )
    }
}

#[component]
fn CollapsibleSection(
    section_id: &'static str,
    title: &'static str,
    open_map: RwSignal<HashMap<String, bool>>,
    children: Children,
) -> impl IntoView {
    let is_open = move || open_map.get().get(section_id).copied().unwrap_or(true);
    view! {
        <section class="sidebar-section">
            <button
                type="button"
                class="sidebar-title sidebar-toggle"
                aria-expanded=is_open
                on:click=move |_| {
                    open_map.update(|m| {
                        let cur = m.get(section_id).copied().unwrap_or(true);
                        m.insert(section_id.to_string(), !cur);
                    });
                }
            >
                <span>{title}</span>
                <span class="sidebar-chevron" aria-hidden="true">
                    {move || if is_open() { "▾" } else { "▸" }}
                </span>
            </button>
            <div class=move || if is_open() { "sidebar-panel open" } else { "sidebar-panel collapsed" }>
                {children()}
            </div>
        </section>
    }
}

#[component]
fn FilterSection(
    section_id: &'static str,
    title: &'static str,
    param: &'static str,
    options: &'static [FilterOption],
    base_path: String,
    query_base: String,
    active_value: Option<String>,
    open_map: RwSignal<HashMap<String, bool>>,
) -> impl IntoView {
    view! {
        <CollapsibleSection section_id=section_id title=title open_map=open_map>
            <ul class="sidebar-list">
                {options
                    .iter()
                    .map(|opt| {
                        let href = toggle_param(
                            &base_path,
                            &query_base,
                            param,
                            opt.id,
                            active_value.as_deref(),
                        );
                        let is_active = active_value.as_deref() == Some(opt.id);
                        view! {
                            <li>
                                <A href=href attr:class=link_class(is_active)>
                                    {opt.label}
                                    {if is_active {
                                        view! { <span class="sidebar-dot" aria-hidden="true"></span> }.into_any()
                                    } else {
                                        ().into_any()
                                    }}
                                </A>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </CollapsibleSection>
    }
}

#[component]
pub fn Sidebar(
    base: BrowserBase,
    categories: Vec<(Category, i64)>,
    query_base: String,
    active_function: Option<String>,
    active_asset_class: Option<String>,
    active_actor: Option<String>,
    active_type: Option<String>,
    active_status: Option<String>,
    active_chain: Option<String>,
    chain_options: Vec<(String, i64)>,
) -> impl IntoView {
    let base_path = base.path().to_string();
    let function_query = query_base.clone();
    let function_base = base_path.clone();
    let chain_query = query_base.clone();
    let chain_base = base_path.clone();
    let open_map = RwSignal::new(default_section_state());

    let all_class = if active_function.is_none() {
        "sidebar-link active"
    } else {
        "sidebar-link"
    };

    view! {
        <aside class="tools-sidebar">
            <div class="sidebar-header">
                <span class="sidebar-heading">"Filters"</span>
                <A href=base_path.clone() attr:class="sidebar-clear">"Clear"</A>
            </div>

            <CollapsibleSection section_id="function" title="Function" open_map=open_map>
                <ul class="sidebar-list">
                    <li>
                        <A
                            href=toggle_param(&function_base, &function_query, "function", "", active_function.as_deref())
                            attr:class=all_class
                        >
                            "All"
                        </A>
                    </li>
                    {categories
                        .into_iter()
                        .map(|(cat, count)| {
                            let href = toggle_param(
                                &function_base,
                                &function_query,
                                "function",
                                &cat.id,
                                active_function.as_deref(),
                            );
                            let is_active = active_function.as_deref() == Some(cat.id.as_str());
                            view! {
                                <li>
                                    <A href=href attr:class=link_class(is_active)>
                                        {cat.label}
                                        <span class="sidebar-count">{count}</span>
                                    </A>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            </CollapsibleSection>

            <FilterSection
                section_id="asset_class"
                title="Asset Class"
                param="asset_class"
                options=ASSET_CLASSES
                base_path=base_path.clone()
                query_base=query_base.clone()
                active_value=active_asset_class
                open_map=open_map
            />
            <FilterSection
                section_id="actor"
                title="Actor"
                param="actor"
                options=ACTORS
                base_path=base_path.clone()
                query_base=query_base.clone()
                active_value=active_actor
                open_map=open_map
            />
            <FilterSection
                section_id="type"
                title="Type"
                param="type"
                options=TYPES
                base_path=base_path.clone()
                query_base=query_base.clone()
                active_value=active_type
                open_map=open_map
            />
            <FilterSection
                section_id="status"
                title="Status"
                param="status"
                options=STATUSES
                base_path=base_path.clone()
                query_base=query_base.clone()
                active_value=active_status
                open_map=open_map
            />

            <CollapsibleSection section_id="chain" title="Chain" open_map=open_map>
                <ul class="sidebar-list">
                    {chain_options
                        .into_iter()
                        .take(9)
                        .map(|(chain, count)| {
                            let href = toggle_param(
                                &chain_base,
                                &chain_query,
                                "chain",
                                &chain,
                                active_chain.as_deref(),
                            );
                            let is_active = active_chain.as_deref() == Some(chain.as_str());
                            view! {
                                <li>
                                    <A href=href attr:class=link_class(is_active)>
                                        {chain.clone()}
                                        <span class="sidebar-count">{count}</span>
                                    </A>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            </CollapsibleSection>
        </aside>
    }
}