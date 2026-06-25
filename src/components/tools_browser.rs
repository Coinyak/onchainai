//! Shared tools browser — sidebar filters, sort bar, HOT list, preview overlays.
//! Used by Home (`/`) and ToolsList (`/tools`) per UI_UX_DESIGN §2.

use crate::components::{
    bottom_sheet::BottomSheet, empty_state::EmptyState, error_state::ErrorState,
    preview_panel::PreviewPanel, search_bar::ToolbarSearch, sidebar::Sidebar,
    skeleton::ToolListSkeleton, tool_card::ToolCard,
};
use crate::filter_query::build_tool_filters;
use crate::models::{Category, Tool};
use crate::server::functions::{
    count_tools, get_categories, get_chain_counts, get_tool_by_slug, list_tools, ToolFilters,
};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BrowserBase {
    Home,
    Tools,
}

impl BrowserBase {
    pub fn path(self) -> &'static str {
        match self {
            BrowserBase::Home => "/",
            BrowserBase::Tools => "/tools",
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_query_base(
    base: BrowserBase,
    function: Option<String>,
    asset_class: Option<String>,
    actor: Option<String>,
    tool_type: Option<String>,
    status: Option<String>,
    chain: Option<String>,
    sort: String,
    search_q: Option<String>,
    selected: Option<String>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = function {
        parts.push(format!("function={v}"));
    }
    if let Some(v) = asset_class {
        parts.push(format!("asset_class={v}"));
    }
    if let Some(v) = actor {
        parts.push(format!("actor={v}"));
    }
    if let Some(v) = tool_type {
        parts.push(format!("type={v}"));
    }
    if let Some(v) = status {
        parts.push(format!("status={v}"));
    }
    if let Some(v) = chain {
        parts.push(format!("chain={v}"));
    }
    if sort != "hot" {
        parts.push(format!("sort={sort}"));
    }
    if let Some(v) = search_q.filter(|s| !s.is_empty()) {
        parts.push(format!("q={v}"));
    }
    if let Some(v) = selected {
        parts.push(format!("selected={v}"));
    }
    if parts.is_empty() {
        base.path().to_string()
    } else {
        format!("{}?{}", base.path(), parts.join("&"))
    }
}

pub fn with_selected(base_path: BrowserBase, base: &str, slug: &str) -> String {
    let root = base_path.path();
    if base == root || base.is_empty() {
        format!("{root}?selected={slug}")
    } else if base.contains('?') {
        format!("{base}&selected={slug}")
    } else {
        format!("{base}?selected={slug}")
    }
}

pub fn without_selected(base_path: BrowserBase, base: &str) -> String {
    let root = base_path.path();
    let trimmed = base.trim_start_matches('?');
    let query = if base.starts_with(root) {
        base.strip_prefix(root).unwrap_or("").trim_start_matches('?')
    } else {
        trimmed
    };
    let parts: Vec<&str> = query
        .split('&')
        .filter(|p| !p.is_empty() && !p.starts_with("selected="))
        .collect();
    if parts.is_empty() {
        root.to_string()
    } else {
        format!("{root}?{}", parts.join("&"))
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct BrowserData {
    categories: Vec<(Category, i64)>,
    chains: Vec<(String, i64)>,
    total: i64,
    tools: Vec<Tool>,
    preview_tool: Option<Tool>,
}

async fn load_browser_data(
    sort: String,
    filters: ToolFilters,
    search_q: Option<String>,
    selected: Option<String>,
) -> Result<BrowserData, ServerFnError> {
    let categories = get_categories().await?;
    let chains = get_chain_counts(12).await?;
    let total = count_tools(filters.clone()).await?;
    let tools = list_tools(sort, 0, 50, filters, search_q).await?;
    let preview_tool = match selected.filter(|s| !s.is_empty()) {
        Some(s) => get_tool_by_slug(s).await.ok(),
        None => None,
    };
    Ok(BrowserData {
        categories,
        chains,
        total,
        tools,
        preview_tool,
    })
}

#[component]
pub fn ToolsBrowser(
    base: BrowserBase,
    #[prop(optional)] show_toolbar_search: bool,
) -> impl IntoView {
    let query = use_query_map();
    let function = Memo::new(move |_| query.with(|q| q.get("function").map(|s| s.to_string())));
    let asset_class = Memo::new(move |_| query.with(|q| q.get("asset_class").map(|s| s.to_string())));
    let actor = Memo::new(move |_| query.with(|q| q.get("actor").map(|s| s.to_string())));
    let tool_type = Memo::new(move |_| query.with(|q| q.get("type").map(|s| s.to_string())));
    let status = Memo::new(move |_| query.with(|q| q.get("status").map(|s| s.to_string())));
    let chain = Memo::new(move |_| query.with(|q| q.get("chain").map(|s| s.to_string())));
    let sort = Memo::new(move |_| {
        query
            .with(|q| q.get("sort").map(|s| s.to_string()))
            .unwrap_or_else(|| "hot".into())
    });
    let search_q = Memo::new(move |_| query.with(|q| q.get("q").map(|s| s.to_string())));
    let selected = Memo::new(move |_| query.with(|q| q.get("selected").map(|s| s.to_string())));

    let query_base = Memo::new(move |_| {
        build_query_base(
            base,
            function.get(),
            asset_class.get(),
            actor.get(),
            tool_type.get(),
            status.get(),
            chain.get(),
            sort.get(),
            search_q.get(),
            selected.get(),
        )
    });

    let filters = Memo::new(move |_| {
        build_tool_filters(
            function.get(),
            asset_class.get(),
            actor.get(),
            tool_type.get(),
            status.get(),
            chain.get(),
        )
    });

    let retry_tick = RwSignal::new(0u32);
    let page_deps = Memo::new(move |_| {
        (
            sort.get(),
            filters.get(),
            search_q.get(),
            selected.get(),
            retry_tick.get(),
        )
    });

    let page = Resource::new(
        move || page_deps.get(),
        |(sort, filters, search_q, selected, _)| async move {
            load_browser_data(sort, filters, search_q, selected).await
        },
    );

    let sort_hot = Memo::new(move |_| {
        let q = query_base.get();
        if q.contains('?') {
            format!("{q}&sort=hot").replace("&sort=hot&sort=hot", "&sort=hot")
        } else {
            format!("{q}?sort=hot")
        }
    });
    let sort_new = Memo::new(move |_| {
        let b = query_base.get();
        if b.contains('?') {
            format!("{b}&sort=new")
        } else {
            format!("{b}?sort=new")
        }
    });
    let sort_comments = Memo::new(move |_| {
        let b = query_base.get();
        if b.contains('?') {
            format!("{b}&sort=comments")
        } else {
            format!("{b}?sort=comments")
        }
    });

    view! {
        <div class="tools-layout" data-tools-browser="">
            <Suspense fallback=|| view! { <ToolListSkeleton count=6/> }>
                {move || match page.get() {
                    Some(Ok(data)) => {
                        let qb = query_base.get();
                        view! {
                            <Sidebar
                                base=base
                                categories=data.categories.clone()
                                query_base=qb.clone()
                                active_function=function.get()
                                active_asset_class=asset_class.get()
                                active_actor=actor.get()
                                active_type=tool_type.get()
                                active_status=status.get()
                                active_chain=chain.get()
                                chain_options=data.chains.clone()
                            />
                            <div class="tools-main">
                                <div class="tools-toolbar sticky-toolbar">
                                    {if show_toolbar_search {
                                        view! { <ToolbarSearch base=base/> }.into_any()
                                    } else {
                                        ().into_any()
                                    }}
                                    <div class="toolbar-sort">
                                        <a href=move || sort_hot.get() class="sort-link">"HOT"</a>
                                        <a href=move || sort_new.get() class="sort-link">"New"</a>
                                        <a href=move || sort_comments.get() class="sort-link">"Comments"</a>
                                    </div>
                                    <span class="tool-count">{data.total}" tools"</span>
                                </div>
                                {if data.tools.is_empty() {
                                    view! { <EmptyState/> }.into_any()
                                } else {
                                    view! {
                                        <div class="tool-list">
                                            {data.tools.clone().into_iter().map(|t| {
                                                let slug = t.slug.clone();
                                                let preview = with_selected(base, &qb, &slug);
                                                view! { <ToolCard tool=t preview_href=preview/> }
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                }}
                            </div>

                            {data.preview_tool.clone().map(|tool| {
                                let close = without_selected(base, &qb);
                                let full = format!("/tools/{}", tool.slug);
                                view! {
                                    <div class="preview-desktop">
                                        <PreviewPanel tool=tool.clone() close_href=close.clone() full_page_href=full.clone()/>
                                    </div>
                                    <div class="preview-mobile">
                                        <BottomSheet tool=tool close_href=close full_page_href=full/>
                                    </div>
                                }
                            })}
                        }.into_any()
                    }
                    Some(Err(e)) => view! {
                        <div class="tools-main tools-main-full">
                            <ErrorState
                                message=e.to_string()
                                on_retry=move || retry_tick.update(|n| *n = n.wrapping_add(1))
                            />
                        </div>
                    }.into_any(),
                    None => view! {
                        <div class="tools-main tools-main-full">
                            <ToolListSkeleton count=6/>
                        </div>
                    }.into_any(),
                }}
            </Suspense>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_query_includes_multi_filters_and_selected() {
        let q = build_query_base(
            BrowserBase::Home,
            Some("bridge,swap".into()),
            None,
            None,
            Some("mcp".into()),
            None,
            None,
            "hot".into(),
            None,
            Some("zapper".into()),
        );
        assert!(q.starts_with("/?"));
        assert!(q.contains("function=bridge,swap"));
        assert!(q.contains("type=mcp"));
        assert!(q.contains("selected=zapper"));
    }

    #[test]
    fn tools_path_clear_is_root() {
        assert_eq!(without_selected(BrowserBase::Tools, "/tools"), "/tools");
        assert_eq!(
            without_selected(BrowserBase::Tools, "/tools?function=swap&selected=foo"),
            "/tools?function=swap"
        );
    }
}