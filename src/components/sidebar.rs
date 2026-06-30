//! Filter sidebar — multi-select, section collapse, full 40px rail + localStorage (harness-round-11).

#[cfg(feature = "hydrate")]
use crate::client_storage::{
    read_sidebar_collapsed_with_default, read_sidebar_sections,
    sidebar_default_collapsed_for_viewport,
};
use crate::client_storage::{write_sidebar_collapsed, write_sidebar_sections};
use crate::components::tools_browser::BrowserBase;
use crate::filter_query::{clear_axis, parse_multi, toggle_multi};
use crate::models::Category;
use leptos::prelude::*;
use std::collections::HashMap;

struct FilterOption {
    id: &'static str,
    label: &'static str,
}

const ASSET_CLASSES: &[FilterOption] = &[
    FilterOption {
        id: "crypto",
        label: "Crypto",
    },
    FilterOption {
        id: "stablecoins",
        label: "Stablecoins",
    },
    FilterOption {
        id: "derivatives",
        label: "Derivatives",
    },
    FilterOption {
        id: "rwa",
        label: "RWA",
    },
];

const ACTORS: &[FilterOption] = &[
    FilterOption {
        id: "human",
        label: "Human",
    },
    FilterOption {
        id: "ai-agent",
        label: "AI Agent",
    },
];

const TYPES: &[FilterOption] = &[
    FilterOption {
        id: "mcp",
        label: "MCP",
    },
    FilterOption {
        id: "cli",
        label: "CLI",
    },
    FilterOption {
        id: "sdk",
        label: "SDK",
    },
    FilterOption {
        id: "api",
        label: "API",
    },
    FilterOption {
        id: "x402",
        label: "x402",
    },
    FilterOption {
        id: "skill",
        label: "Skill",
    },
];

const STATUSES: &[FilterOption] = &[
    FilterOption {
        id: "community",
        label: "Community",
    },
    FilterOption {
        id: "verified",
        label: "Verified",
    },
    FilterOption {
        id: "official",
        label: "Official",
    },
];

const PRICING: &[FilterOption] = &[
    FilterOption {
        id: "free",
        label: "Free",
    },
    FilterOption {
        id: "x402",
        label: "x402",
    },
    FilterOption {
        id: "paid",
        label: "Paid",
    },
    FilterOption {
        id: "freemium",
        label: "Freemium",
    },
];

fn default_section_state(function_open: bool) -> HashMap<String, bool> {
    [
        ("function", function_open),
        ("asset_class", false),
        ("actor", false),
        ("type", false),
        ("status", false),
        ("pricing", false),
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

fn persist_sidebar_collapsed(collapsed: bool) {
    write_sidebar_collapsed(collapsed);
}

fn set_sidebar_collapsed(sidebar_collapsed: RwSignal<bool>, collapsed: bool) {
    sidebar_collapsed.set(collapsed);
    persist_sidebar_collapsed(collapsed);
}

fn collapse_mobile_sidebar(sidebar_collapsed: RwSignal<bool>) {
    #[cfg(feature = "hydrate")]
    {
        if sidebar_default_collapsed_for_viewport() {
            set_sidebar_collapsed(sidebar_collapsed, true);
        }
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = sidebar_collapsed;
    }
}

/// Function-filter `<A href>` — same logic as the Sidebar function-section `.map` closure.
pub fn sidebar_function_link(
    base: &crate::components::tools_browser::BrowserBase,
    query_base: &str,
    cat_id: &str,
    fn_active: &[String],
) -> (String, bool) {
    use crate::components::tools_browser::{category_href, BrowserBase};
    let (href, is_active) = match base {
        BrowserBase::Category(current) => {
            let active = current == cat_id || fn_active.iter().any(|v| v == cat_id);
            (category_href(cat_id, query_base), active)
        }
        _ => {
            let base_path = base.path();
            let href = toggle_multi(&base_path, query_base, "function", cat_id, fn_active);
            let is_active = fn_active.iter().any(|v| v == cat_id);
            (href, is_active)
        }
    };
    (href, is_active)
}

#[component]
fn CollapsibleSection(
    section_id: &'static str,
    title: &'static str,
    open_map: RwSignal<HashMap<String, bool>>,
    sidebar_collapsed: RwSignal<bool>,
    sidebar_storage_loaded: RwSignal<bool>,
    children: Children,
) -> impl IntoView {
    let is_open = move || open_map.get().get(section_id).copied().unwrap_or(true);
    view! {
        <section class="sidebar-section">
            <button
                type="button"
                class="sidebar-title sidebar-toggle"
                prop:aria-expanded=is_open
                on:click=move |_| {
                    open_map.update(|m| {
                        let cur = m.get(section_id).copied().unwrap_or(true);
                        m.insert(section_id.to_string(), !cur);
                        if sidebar_storage_loaded.get() {
                            write_sidebar_sections(m);
                        }
                    });
                }
            >
                <span class="sidebar-title-text">{title}</span>
                <span class="sidebar-chevron" aria-hidden="true">
                    {move || if is_open() { "▾" } else { "▸" }}
                </span>
            </button>
            <div class=move || {
                if sidebar_collapsed.get() || !is_open() {
                    "sidebar-panel collapsed"
                } else {
                    "sidebar-panel open"
                }
            }>
                {children()}
            </div>
        </section>
    }
}

#[component]
pub fn Sidebar(
    base: BrowserBase,
    categories: Vec<(Category, i64)>,
    query_base: String,
    filter_revision: Memo<String>,
    active_function: Option<String>,
    active_asset_class: Option<String>,
    active_actor: Option<String>,
    active_type: Option<String>,
    active_status: Option<String>,
    active_pricing: Option<String>,
    #[prop(default = true)] default_function_open: bool,
) -> impl IntoView {
    let base_path = base.path();
    let fn_active = parse_multi(active_function.as_deref());
    let ac_active = parse_multi(active_asset_class.as_deref());
    let actor_active = parse_multi(active_actor.as_deref());
    let type_active = parse_multi(active_type.as_deref());
    let status_active = parse_multi(active_status.as_deref());
    let pricing_active = parse_multi(active_pricing.as_deref());

    // SSR: collapsed strip on mobile via CSS until storage loads; desktop expanded rail.
    let default_sections = default_section_state(default_function_open);
    let sidebar_collapsed = RwSignal::new(true);
    let open_map = RwSignal::new(default_sections.clone());
    let sidebar_storage_loaded = RwSignal::new(false);

    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            let sections_default = default_sections.clone();
            wasm_bindgen_futures::spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(0).await;
                if sidebar_default_collapsed_for_viewport() {
                    set_sidebar_collapsed(sidebar_collapsed, true);
                } else {
                    sidebar_collapsed.set(read_sidebar_collapsed_with_default(false));
                }
                open_map.set(read_sidebar_sections(sections_default));
                sidebar_storage_loaded.set(true);
            });
        });
    }

    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            let collapsed = sidebar_collapsed.get();
            if let Some(body) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.body())
            {
                if sidebar_default_collapsed_for_viewport() && !collapsed {
                    let _ = body.class_list().add_1("sidebar-scroll-locked");
                } else {
                    let _ = body.class_list().remove_1("sidebar-scroll-locked");
                }
            }
        });
    }

    let backdrop_ref = NodeRef::<leptos::html::Button>::new();
    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            if !sidebar_collapsed.get() && sidebar_default_collapsed_for_viewport() {
                if let Some(btn) = backdrop_ref.get() {
                    let _ = btn.focus();
                }
            }
        });

        let revision_snapshot = StoredValue::new(String::new());
        Effect::new(move |_| {
            if !sidebar_storage_loaded.get() {
                return;
            }
            let revision = filter_revision.get();
            let prev = revision_snapshot.get_value();
            if prev.is_empty() {
                revision_snapshot.set_value(revision);
                return;
            }
            if crate::filter_query::should_collapse_mobile_sidebar_on_route_change(
                &prev,
                &revision,
                sidebar_default_collapsed_for_viewport(),
            ) {
                set_sidebar_collapsed(sidebar_collapsed, true);
            }
            revision_snapshot.set_value(revision);
        });
    }

    let aside_class = move || {
        if sidebar_collapsed.get() {
            "tools-sidebar tools-sidebar-collapsed"
        } else {
            "tools-sidebar"
        }
    };
    let clear_href = match &base {
        BrowserBase::Category(_) => "/tools".to_string(),
        _ => base_path.clone(),
    };
    let fn_all_href = match &base {
        BrowserBase::Category(_) => "/tools".to_string(),
        _ => clear_axis(&base_path, &query_base, "function"),
    };
    let base_for_fn = base.clone();
    let query_for_fn = query_base.clone();
    let base_for_ac = base_path.clone();
    let query_for_ac = query_base.clone();
    let base_for_actor = base_path.clone();
    let query_for_actor = query_base.clone();
    let base_for_type = base_path.clone();
    let query_for_type = query_base.clone();
    let base_for_status = base_path.clone();
    let query_for_status = query_base.clone();
    let base_for_pricing = base_path.clone();
    let query_for_pricing = query_base.clone();

    let show_mobile_backdrop = move || {
        if !sidebar_storage_loaded.get() || sidebar_collapsed.get() {
            return false;
        }
        #[cfg(feature = "hydrate")]
        {
            sidebar_default_collapsed_for_viewport()
        }
        #[cfg(not(feature = "hydrate"))]
        {
            false
        }
    };

    view! {
        <div class="tools-sidebar-shell">
            <Show when=show_mobile_backdrop>
                <button
                    type="button"
                    node_ref=backdrop_ref
                    class="sidebar-mobile-backdrop"
                    aria-label="Close filters"
                    tabindex="-1"
                    on:click=move |_| set_sidebar_collapsed(sidebar_collapsed, true)
                    on:keydown=move |ev| {
                        if ev.key() == "Escape" {
                            ev.stop_propagation();
                            set_sidebar_collapsed(sidebar_collapsed, true);
                        }
                    }
                />
            </Show>
            <aside
                class=aside_class
                data-sidebar-ready=""
                data-filter-revision=move || filter_revision.get()
                data-sidebar-storage-loaded=move || sidebar_storage_loaded.get().then_some("")
                aria-busy=move || (!sidebar_storage_loaded.get()).then_some("true")
                on:keydown=move |ev| {
                    if ev.key() == "Escape" && !sidebar_collapsed.get() {
                        ev.stop_propagation();
                        set_sidebar_collapsed(sidebar_collapsed, true);
                    }
                }
            >
            <div class="sidebar-controls">
                <button
                    type="button"
                    class="sidebar-rail-toggle"
                    aria-label="Toggle filters sidebar"
                    prop:aria-expanded=move || !sidebar_collapsed.get()
                    on:click=move |_| {
                        let was_collapsed = sidebar_collapsed.get_untracked();
                        let next_collapsed = !was_collapsed;
                        set_sidebar_collapsed(sidebar_collapsed, next_collapsed);
                        if was_collapsed {
                            open_map.update(|m| {
                                m.insert("function".to_string(), true);
                                if sidebar_storage_loaded.get_untracked() {
                                    write_sidebar_sections(m);
                                }
                            });
                        }
                    }
                >
                    <svg class="sidebar-rail-toggle-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
                        <line x1="4" y1="7" x2="20" y2="7" />
                        <line x1="4" y1="12" x2="20" y2="12" />
                        <line x1="4" y1="17" x2="20" y2="17" />
                    </svg>
                </button>
                <a
                    href=clear_href.clone()
                    class="sidebar-clear sidebar-title-text"
                    on:click=move |_| collapse_mobile_sidebar(sidebar_collapsed)
                >
                    "Clear"
                </a>
            </div>

            <div class="sidebar-rail-icons">
                {[
                    ("function", "Fn", "Function"),
                    ("asset_class", "Ac", "Asset Class"),
                    ("actor", "Hu", "Actor"),
                    ("type", "Ty", "Type"),
                    ("status", "St", "Status"),
                    ("pricing", "Pr", "Pricing"),
                ].into_iter().map(|(id, short, label)| {
                    let section_id = id.to_string();
                    view! {
                        <button
                            type="button"
                            class="sidebar-rail-icon"
                            title=label
                            aria-label=label
                            on:click=move |_| {
                                set_sidebar_collapsed(sidebar_collapsed, false);
                                open_map.update(|m| {
                                    m.insert(section_id.clone(), true);
                                    if sidebar_storage_loaded.get() {
                                        write_sidebar_sections(m);
                                    }
                                });
                            }
                        >
                            {short}
                        </button>
                    }
                }).collect_view()}
            </div>

            <div class="sidebar-body">
                <CollapsibleSection section_id="function" title="Function" open_map=open_map sidebar_collapsed=sidebar_collapsed sidebar_storage_loaded=sidebar_storage_loaded>
                    <ul class="sidebar-list">
                        <li>
                            <a
                                href=fn_all_href.clone()
                                class=if fn_active.is_empty() { "sidebar-link active" } else { "sidebar-link" }
                                on:click=move |_| collapse_mobile_sidebar(sidebar_collapsed)
                            >
                                "All"
                            </a>
                        </li>
                        {categories.into_iter().map(|(cat, count)| {
                            let (href, is_active) =
                                sidebar_function_link(&base_for_fn, &query_for_fn, &cat.id, &fn_active);
                            view! {
                                <li>
                                    <a
                                        href=href
                                        class=link_class(is_active)
                                        on:click=move |_| collapse_mobile_sidebar(sidebar_collapsed)
                                    >
                                        <span class="sidebar-title-text">{cat.label}</span>
                                        <span class="sidebar-count">{count}</span>
                                    </a>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </CollapsibleSection>

                <CollapsibleSection section_id="asset_class" title="Asset Class" open_map=open_map sidebar_collapsed=sidebar_collapsed sidebar_storage_loaded=sidebar_storage_loaded>
                    <ul class="sidebar-list">
                        {ASSET_CLASSES.iter().map(|opt| {
                            let href = toggle_multi(&base_for_ac, &query_for_ac, "asset_class", opt.id, &ac_active);
                            let is_active = ac_active.iter().any(|v| v == opt.id);
                            view! {
                                <li>
                                    <a
                                        href=href
                                        class=link_class(is_active)
                                        on:click=move |_| collapse_mobile_sidebar(sidebar_collapsed)
                                    >
                                        <span class="sidebar-title-text">{opt.label}</span>
                                    </a>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </CollapsibleSection>

                <CollapsibleSection section_id="actor" title="Actor" open_map=open_map sidebar_collapsed=sidebar_collapsed sidebar_storage_loaded=sidebar_storage_loaded>
                    <ul class="sidebar-list">
                        {ACTORS.iter().map(|opt| {
                            let href = toggle_multi(&base_for_actor, &query_for_actor, "actor", opt.id, &actor_active);
                            let is_active = actor_active.iter().any(|v| v == opt.id);
                            view! {
                                <li>
                                    <a
                                        href=href
                                        class=link_class(is_active)
                                        on:click=move |_| collapse_mobile_sidebar(sidebar_collapsed)
                                    >
                                        <span class="sidebar-title-text">{opt.label}</span>
                                    </a>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </CollapsibleSection>

                <CollapsibleSection section_id="type" title="Type" open_map=open_map sidebar_collapsed=sidebar_collapsed sidebar_storage_loaded=sidebar_storage_loaded>
                    <ul class="sidebar-list">
                        {TYPES.iter().map(|opt| {
                            let href = toggle_multi(&base_for_type, &query_for_type, "type", opt.id, &type_active);
                            let is_active = type_active.iter().any(|v| v == opt.id);
                            view! {
                                <li>
                                    <a
                                        href=href
                                        class=link_class(is_active)
                                        on:click=move |_| collapse_mobile_sidebar(sidebar_collapsed)
                                    >
                                        <span class="sidebar-title-text">{opt.label}</span>
                                    </a>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </CollapsibleSection>

                <CollapsibleSection section_id="status" title="Status" open_map=open_map sidebar_collapsed=sidebar_collapsed sidebar_storage_loaded=sidebar_storage_loaded>
                    <ul class="sidebar-list">
                        {STATUSES.iter().map(|opt| {
                            let href = toggle_multi(&base_for_status, &query_for_status, "status", opt.id, &status_active);
                            let is_active = status_active.iter().any(|v| v == opt.id);
                            view! {
                                <li>
                                    <a
                                        href=href
                                        class=link_class(is_active)
                                        on:click=move |_| collapse_mobile_sidebar(sidebar_collapsed)
                                    >
                                        <span class="sidebar-title-text">{opt.label}</span>
                                    </a>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </CollapsibleSection>

                <CollapsibleSection section_id="pricing" title="Pricing" open_map=open_map sidebar_collapsed=sidebar_collapsed sidebar_storage_loaded=sidebar_storage_loaded>
                    <ul class="sidebar-list">
                        {PRICING.iter().map(|opt| {
                            let href = toggle_multi(&base_for_pricing, &query_for_pricing, "pricing", opt.id, &pricing_active);
                            let is_active = pricing_active.iter().any(|v| v == opt.id);
                            view! {
                                <li>
                                    <a
                                        href=href
                                        class=link_class(is_active)
                                        on:click=move |_| collapse_mobile_sidebar(sidebar_collapsed)
                                    >
                                        <span class="sidebar-title-text">{opt.label}</span>
                                    </a>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </CollapsibleSection>
            </div>
        </aside>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::tools_browser::{build_query_base, BrowserBase, BrowserQueryParams};

    fn query_params() -> BrowserQueryParams {
        BrowserQueryParams {
            sort: "hot".into(),
            page: 1,
            ..BrowserQueryParams::default()
        }
    }

    #[test]
    fn function_all_clears_only_function_param() {
        let query_base = build_query_base(
            &BrowserBase::Tools,
            &BrowserQueryParams {
                function: Some("bridge".into()),
                sort: "new".into(),
                search_q: Some("test query".into()),
                ..query_params()
            },
        );
        let href = clear_axis("/tools", &query_base, "function");
        assert!(!href.contains("function="));
        assert!(href.contains("sort=new"));
        assert!(href.contains("q="));
    }

    #[test]
    fn sidebar_function_link_produces_multi_select_href() {
        let query_base = build_query_base(
            &BrowserBase::Tools,
            &BrowserQueryParams {
                function: Some("bridge".into()),
                sort: "new".into(),
                ..query_params()
            },
        );
        let fn_active = parse_multi(Some("bridge"));
        let tools_base = BrowserBase::Tools;
        let (_, bridge_active) =
            sidebar_function_link(&tools_base, &query_base, "bridge", &fn_active);
        assert!(bridge_active);

        let (href, swap_active) =
            sidebar_function_link(&tools_base, &query_base, "swap", &fn_active);
        assert!(!swap_active);
        assert!(
            href.contains("function=bridge%2Cswap")
                || href.contains("function=swap%2Cbridge")
                || href.contains("function=bridge,swap")
                || href.contains("function=swap,bridge"),
            "Sidebar <A href> must encode comma-separated function param, got: {href}"
        );
        assert_eq!(
            href.matches("sort=").count(),
            1,
            "sort must not duplicate: {href}"
        );
        assert!(href.contains("sort=new"));
    }

    #[test]
    fn sidebar_function_link_on_category_navigates_to_other_category() {
        let query_base = "/categories/bridge?chain=ethereum&sort=new".to_string();
        let cat_base = BrowserBase::Category("bridge".into());
        let fn_active = parse_multi(Some("bridge"));
        let (href, active) = sidebar_function_link(&cat_base, &query_base, "swap", &fn_active);
        assert!(!active);
        assert_eq!(href, "/categories/swap?chain=ethereum&sort=new");
    }

    #[test]
    fn sidebar_data_attrs_render_without_leptos_attr_prefix() {
        let html = leptos::prelude::Owner::new().with(|| {
            view! { <aside data-sidebar-ready="" class="tools-sidebar">"x"</aside> }.to_html()
        });
        assert!(
            html.contains(r#"data-sidebar-ready=""#) || html.contains("data-sidebar-ready"),
            "expected data-sidebar-ready in SSR HTML, got: {html}"
        );
        assert!(
            !html.contains("attr:data-sidebar-ready"),
            "Leptos must not leak attr: prefix into HTML: {html}"
        );
    }

    #[test]
    fn sidebar_pricing_href_includes_pricing_param() {
        let query_base = build_query_base(
            &BrowserBase::Tools,
            &BrowserQueryParams {
                sort: "new".into(),
                ..query_params()
            },
        );
        let href = toggle_multi("/tools", &query_base, "pricing", "x402", &[]);
        assert!(
            href.contains("pricing=x402"),
            "pricing filter href must include pricing=x402, got: {href}"
        );
    }

    #[test]
    fn sidebar_function_link_bridge_href_includes_function_param() {
        let query_base = build_query_base(
            &BrowserBase::Tools,
            &BrowserQueryParams {
                sort: "new".into(),
                ..query_params()
            },
        );
        let (href, active) = sidebar_function_link(&BrowserBase::Tools, &query_base, "bridge", &[]);
        assert!(!active);
        assert!(
            href.contains("function=bridge"),
            "bridge filter href must include function=bridge, got: {href}"
        );
        let lower = href.to_lowercase();
        assert!(
            !lower.contains("error deserializing") && !lower.contains("missing field"),
            "href must not contain error strings: {href}"
        );
    }
}
