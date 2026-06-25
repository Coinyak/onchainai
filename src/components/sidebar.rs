//! Filter sidebar for tools list — six axes with URL query sync.

use crate::models::Category;
use leptos::prelude::*;
use leptos_router::components::A;

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

fn link_class(active: bool) -> &'static str {
    if active {
        "sidebar-link active"
    } else {
        "sidebar-link"
    }
}

fn toggle_param(base: &str, key: &str, value: &str, current: Option<&str>) -> String {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for part in base.trim_start_matches('?').split('&').filter(|s| !s.is_empty()) {
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
        "/tools".to_string()
    } else {
        format!("/tools?{}", pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&"))
    }
}

#[component]
fn FilterSection(
    title: &'static str,
    param: &'static str,
    options: &'static [FilterOption],
    query_base: String,
    active_value: Option<String>,
) -> impl IntoView {
    view! {
        <section class="sidebar-section">
            <h3 class="sidebar-title">{title}</h3>
            <ul class="sidebar-list">
                {options
                    .iter()
                    .map(|opt| {
                        let href = toggle_param(&query_base, param, opt.id, active_value.as_deref());
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
        </section>
    }
}

#[component]
pub fn Sidebar(
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
    let all_class = if active_function.is_none() {
        "sidebar-link active"
    } else {
        "sidebar-link"
    };
    let clear_href = "/tools".to_string();

    view! {
        <aside class="tools-sidebar">
            <div class="sidebar-header">
                <span class="sidebar-heading">"Filters"</span>
                <A href=clear_href attr:class="sidebar-clear">"Clear"</A>
            </div>

            <section class="sidebar-section">
                <h3 class="sidebar-title">"Function"</h3>
                <ul class="sidebar-list">
                    <li>
                        <A href=toggle_param(&query_base, "function", "", active_function.as_deref())
                            attr:class=all_class>
                            "All"
                        </A>
                    </li>
                    {categories
                        .into_iter()
                        .map(|(cat, count)| {
                            let href = toggle_param(
                                &query_base,
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
            </section>

            <FilterSection
                title="Asset Class"
                param="asset_class"
                options=ASSET_CLASSES
                query_base=query_base.clone()
                active_value=active_asset_class
            />
            <FilterSection
                title="Actor"
                param="actor"
                options=ACTORS
                query_base=query_base.clone()
                active_value=active_actor
            />
            <FilterSection
                title="Type"
                param="type"
                options=TYPES
                query_base=query_base.clone()
                active_value=active_type
            />
            <FilterSection
                title="Status"
                param="status"
                options=STATUSES
                query_base=query_base.clone()
                active_value=active_status
            />

            <section class="sidebar-section">
                <h3 class="sidebar-title">"Chain"</h3>
                <ul class="sidebar-list">
                    {chain_options
                        .into_iter()
                        .take(9)
                        .map(|(chain, count)| {
                            let href = toggle_param(
                                &query_base,
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
            </section>
        </aside>
    }
}