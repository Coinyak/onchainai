//! Filter sidebar — multi-select, section collapse, full 40px rail + localStorage.
//! Mobile (<768px): fullscreen filter overlay via [`FilterOverlayCtx`].

use crate::client_storage::{
    read_sidebar_collapsed, read_sidebar_sections, write_sidebar_collapsed, write_sidebar_sections,
};
#[cfg(target_arch = "wasm32")]
use crate::client_storage::read_bool;
use crate::components::tools_browser::BrowserBase;
use crate::filter_query::{clear_axis, parse_multi, toggle_multi};
use crate::models::Category;
use leptos::prelude::*;
use leptos_router::components::A;
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
const SIDEBAR_COLLAPSED_KEY: &str = "onchain-ai-sidebar-collapsed";
#[cfg(target_arch = "wasm32")]
const MOBILE_BREAKPOINT_PX: f64 = 768.0;

/// Shared open state for the mobile fullscreen filter overlay.
/// Provided by [`Sidebar`]; a parent can pass `overlay_open` to share with sibling controls.
#[derive(Clone, Copy)]
pub struct FilterOverlayCtx(pub RwSignal<bool>);

fn initial_sidebar_collapsed() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if is_browser() {
            if let Ok(w) = window().inner_width() {
                if w < MOBILE_BREAKPOINT_PX {
                    return read_bool(SIDEBAR_COLLAPSED_KEY, true);
                }
            }
        }
    }
    read_sidebar_collapsed()
}

fn sidebar_aside_class(collapsed: bool, mobile_open: bool) -> String {
    let mut classes = vec!["tools-sidebar", "tools-sidebar-desktop-only"];
    if collapsed {
        classes.push("tools-sidebar-collapsed");
    }
    if mobile_open {
        classes.push("sidebar-mobile-open");
    }
    classes.join(" ")
}

#[cfg(target_arch = "wasm32")]
fn is_mobile_viewport() -> bool {
    is_browser() && window().inner_width().map(|w| w < MOBILE_BREAKPOINT_PX).unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn is_mobile_viewport() -> bool {
    false
}

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

fn default_section_state(function_open: bool) -> HashMap<String, bool> {
    [
        ("function", function_open),
        ("asset_class", false),
        ("actor", false),
        ("type", false),
        ("status", false),
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

/// Function-filter `<A href>` — same logic as the Sidebar function-section `.map` closure.
pub fn sidebar_function_link(
    base_path: &str,
    query_base: &str,
    cat_id: &str,
    fn_active: &[String],
) -> (String, bool) {
    let href = toggle_multi(base_path, query_base, "function", cat_id, fn_active);
    let is_active = fn_active.iter().any(|v| v == cat_id);
    (href, is_active)
}

#[component]
fn CollapsibleSection(
    section_id: &'static str,
    title: &'static str,
    open_map: RwSignal<HashMap<String, bool>>,
    sidebar_collapsed: RwSignal<bool>,
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
                        write_sidebar_sections(m);
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
    active_function: Option<String>,
    active_asset_class: Option<String>,
    active_actor: Option<String>,
    active_type: Option<String>,
    active_status: Option<String>,
    active_chain: Option<String>,
    chain_options: Vec<(String, i64)>,
    #[prop(default = true)] default_function_open: bool,
    /// When set by a parent (e.g. `tools_browser`), shares overlay state with sibling controls.
    #[prop(optional)]
    overlay_open: Option<RwSignal<bool>>,
) -> impl IntoView {
    let base_path = base.path().to_string();
    let fn_active = parse_multi(active_function.as_deref());
    let ac_active = parse_multi(active_asset_class.as_deref());
    let actor_active = parse_multi(active_actor.as_deref());
    let type_active = parse_multi(active_type.as_deref());
    let status_active = parse_multi(active_status.as_deref());
    let chain_active = parse_multi(active_chain.as_deref());

    let mobile_open = overlay_open.unwrap_or_else(|| RwSignal::new(false));
    provide_context(FilterOverlayCtx(mobile_open));

    let sidebar_collapsed = RwSignal::new(initial_sidebar_collapsed());
    let open_map = RwSignal::new(read_sidebar_sections(default_section_state(
        default_function_open,
    )));

    Effect::new(move |_| {
        write_sidebar_collapsed(sidebar_collapsed.get());
    });

    let close_mobile_overlay = move |_| mobile_open.set(false);

    let aside_class = move || {
        sidebar_aside_class(sidebar_collapsed.get(), mobile_open.get())
    };
    let clear_href = base_path.clone();
    let fn_all_href = clear_axis(&base_path, &query_base, "function");
    let base_for_fn = base_path.clone();
    let query_for_fn = query_base.clone();
    let base_for_ac = base_path.clone();
    let query_for_ac = query_base.clone();
    let base_for_actor = base_path.clone();
    let query_for_actor = query_base.clone();
    let base_for_type = base_path.clone();
    let query_for_type = query_base.clone();
    let base_for_status = base_path.clone();
    let query_for_status = query_base.clone();
    let base_for_chain = base_path.clone();
    let query_for_chain = query_base.clone();

    view! {
        <Show when=move || mobile_open.get()>
            <div
                class="sidebar-mobile-backdrop"
                aria-hidden="true"
                on:click=close_mobile_overlay
            />
        </Show>
        <aside class=aside_class>
            <div class="sidebar-header">
                <button
                    type="button"
                    class="sidebar-rail-toggle"
                    aria-label="Toggle filters sidebar"
                    prop:aria-expanded=move || !sidebar_collapsed.get() || mobile_open.get()
                    on:click=move |_| {
                        if is_mobile_viewport() {
                            mobile_open.set(true);
                        } else {
                            sidebar_collapsed.update(|c| *c = !*c);
                        }
                    }
                >
                    "☰"
                </button>
                <button
                    type="button"
                    class="sidebar-mobile-close"
                    aria-label="Close filters"
                    on:click=close_mobile_overlay
                >
                    "✕"
                </button>
                <span class="sidebar-heading sidebar-title-text">"Filters"</span>
                <A
                    href=clear_href.clone()
                    attr:class="sidebar-clear sidebar-title-text"
                    on:click=close_mobile_overlay
                >
                    "Clear"
                </A>
            </div>

            <div class="sidebar-rail-icons">
                {[
                    ("function", "Fn", "Function"),
                    ("asset_class", "Ac", "Asset Class"),
                    ("actor", "Hu", "Actor"),
                    ("type", "Ty", "Type"),
                    ("status", "St", "Status"),
                    ("chain", "Ch", "Chain"),
                ].into_iter().map(|(id, short, label)| {
                    let section_id = id.to_string();
                    view! {
                        <button
                            type="button"
                            class="sidebar-rail-icon"
                            title=label
                            aria-label=label
                            on:click=move |_| {
                                sidebar_collapsed.set(false);
                                open_map.update(|m| {
                                    m.insert(section_id.clone(), true);
                                    write_sidebar_sections(m);
                                });
                            }
                        >
                            {short}
                        </button>
                    }
                }).collect_view()}
            </div>

            <div class="sidebar-body">
                <CollapsibleSection section_id="function" title="Function" open_map=open_map sidebar_collapsed=sidebar_collapsed>
                    <ul class="sidebar-list">
                        <li>
                            <A
                                href=fn_all_href.clone()
                                attr:class=if fn_active.is_empty() { "sidebar-link active" } else { "sidebar-link" }
                                on:click=close_mobile_overlay
                            >
                                "All"
                            </A>
                        </li>
                        {categories.into_iter().map(|(cat, count)| {
                            let (href, is_active) =
                                sidebar_function_link(&base_for_fn, &query_for_fn, &cat.id, &fn_active);
                            view! {
                                <li>
                                    <A href=href attr:class=link_class(is_active) on:click=close_mobile_overlay>
                                        <span class="sidebar-title-text">{cat.label}</span>
                                        <span class="sidebar-count">{count}</span>
                                    </A>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </CollapsibleSection>

                <CollapsibleSection section_id="asset_class" title="Asset Class" open_map=open_map sidebar_collapsed=sidebar_collapsed>
                    <ul class="sidebar-list">
                        {ASSET_CLASSES.iter().map(|opt| {
                            let href = toggle_multi(&base_for_ac, &query_for_ac, "asset_class", opt.id, &ac_active);
                            let is_active = ac_active.iter().any(|v| v == opt.id);
                            view! {
                                <li><A href=href attr:class=link_class(is_active) on:click=close_mobile_overlay><span class="sidebar-title-text">{opt.label}</span></A></li>
                            }
                        }).collect_view()}
                    </ul>
                </CollapsibleSection>

                <CollapsibleSection section_id="actor" title="Actor" open_map=open_map sidebar_collapsed=sidebar_collapsed>
                    <ul class="sidebar-list">
                        {ACTORS.iter().map(|opt| {
                            let href = toggle_multi(&base_for_actor, &query_for_actor, "actor", opt.id, &actor_active);
                            let is_active = actor_active.iter().any(|v| v == opt.id);
                            view! {
                                <li><A href=href attr:class=link_class(is_active) on:click=close_mobile_overlay><span class="sidebar-title-text">{opt.label}</span></A></li>
                            }
                        }).collect_view()}
                    </ul>
                </CollapsibleSection>

                <CollapsibleSection section_id="type" title="Type" open_map=open_map sidebar_collapsed=sidebar_collapsed>
                    <ul class="sidebar-list">
                        {TYPES.iter().map(|opt| {
                            let href = toggle_multi(&base_for_type, &query_for_type, "type", opt.id, &type_active);
                            let is_active = type_active.iter().any(|v| v == opt.id);
                            view! {
                                <li><A href=href attr:class=link_class(is_active) on:click=close_mobile_overlay><span class="sidebar-title-text">{opt.label}</span></A></li>
                            }
                        }).collect_view()}
                    </ul>
                </CollapsibleSection>

                <CollapsibleSection section_id="status" title="Status" open_map=open_map sidebar_collapsed=sidebar_collapsed>
                    <ul class="sidebar-list">
                        {STATUSES.iter().map(|opt| {
                            let href = toggle_multi(&base_for_status, &query_for_status, "status", opt.id, &status_active);
                            let is_active = status_active.iter().any(|v| v == opt.id);
                            view! {
                                <li><A href=href attr:class=link_class(is_active) on:click=close_mobile_overlay><span class="sidebar-title-text">{opt.label}</span></A></li>
                            }
                        }).collect_view()}
                    </ul>
                </CollapsibleSection>

                <CollapsibleSection section_id="chain" title="Chain" open_map=open_map sidebar_collapsed=sidebar_collapsed>
                    {if chain_options.is_empty() {
                        view! {
                            <p class="sidebar-empty">"No chains yet"</p>
                        }.into_any()
                    } else {
                        view! {
                            <ul class="sidebar-list">
                                {chain_options.into_iter().take(9).map(|(chain, count)| {
                                    let href = toggle_multi(&base_for_chain, &query_for_chain, "chain", &chain, &chain_active);
                                    let is_active = chain_active.iter().any(|v| v == &chain);
                                    view! {
                                        <li>
                                            <A href=href attr:class=link_class(is_active) on:click=close_mobile_overlay>
                                                <span class="sidebar-title-text">{chain.clone()}</span>
                                                <span class="sidebar-count">{count}</span>
                                            </A>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any()
                    }}
                </CollapsibleSection>
            </div>
        </aside>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::tools_browser::{build_query_base, BrowserBase};

    #[test]
    fn function_all_clears_only_function_param() {
        let query_base = build_query_base(
            BrowserBase::Tools,
            Some("bridge".into()),
            None,
            None,
            None,
            None,
            None,
            "new".into(),
            Some("test query".into()),
            None,
        );
        let href = clear_axis("/tools", &query_base, "function");
        assert!(!href.contains("function="));
        assert!(href.contains("sort=new"));
        assert!(href.contains("q="));
    }

    #[test]
    fn sidebar_aside_class_includes_mobile_overlay_classes() {
        assert_eq!(
            sidebar_aside_class(false, false),
            "tools-sidebar tools-sidebar-desktop-only"
        );
        assert_eq!(
            sidebar_aside_class(true, false),
            "tools-sidebar tools-sidebar-desktop-only tools-sidebar-collapsed"
        );
        assert_eq!(
            sidebar_aside_class(false, true),
            "tools-sidebar tools-sidebar-desktop-only sidebar-mobile-open"
        );
        assert_eq!(
            sidebar_aside_class(true, true),
            "tools-sidebar tools-sidebar-desktop-only tools-sidebar-collapsed sidebar-mobile-open"
        );
    }

    #[test]
    fn initial_sidebar_collapsed_defaults_on_ssr() {
        assert!(!initial_sidebar_collapsed());
    }

    #[test]
    fn sidebar_function_link_produces_multi_select_href() {
        let query_base = build_query_base(
            BrowserBase::Tools,
            Some("bridge".into()),
            None,
            None,
            None,
            None,
            None,
            "new".into(),
            None,
            None,
        );
        let fn_active = parse_multi(Some("bridge"));
        let (_, bridge_active) = sidebar_function_link("/tools", &query_base, "bridge", &fn_active);
        assert!(bridge_active);

        let (href, swap_active) = sidebar_function_link("/tools", &query_base, "swap", &fn_active);
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
}
