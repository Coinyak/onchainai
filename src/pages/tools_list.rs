//! Tools list page — sidebar, sort bar, search, tool cards, preview overlays.

use crate::components::{
    bottom_sheet::BottomSheet, preview_panel::PreviewPanel, sidebar::Sidebar,
    tool_card::ToolCard, top_nav::TopNav,
};
use crate::server::functions::{count_tools, get_categories, get_chain_counts, get_tool_by_slug, list_tools, ToolFilters};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

#[allow(clippy::too_many_arguments)]
fn build_query_base(
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
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
}

fn with_selected(base: &str, slug: &str) -> String {
    if base.is_empty() {
        format!("/tools?selected={slug}")
    } else if base.starts_with('?') {
        format!("/tools{base}&selected={slug}")
    } else {
        format!("/tools?{base}&selected={slug}")
    }
}

fn without_selected(base: &str) -> String {
    let trimmed = base.trim_start_matches('?');
    let parts: Vec<&str> = trimmed
        .split('&')
        .filter(|p| !p.is_empty() && !p.starts_with("selected="))
        .collect();
    if parts.is_empty() {
        "/tools".to_string()
    } else {
        format!("/tools?{}", parts.join("&"))
    }
}

#[component]
pub fn ToolsListPage() -> impl IntoView {
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

    let filters = Memo::new(move |_| ToolFilters {
        function: function.get(),
        asset_class: asset_class.get(),
        actor: actor.get(),
        tool_type: tool_type.get(),
        status: status.get(),
        chain: chain.get(),
    });

    let categories = Resource::new(|| (), |_| async move { get_categories().await });
    let chains = Resource::new(|| (), |_| async move { get_chain_counts(12).await });
    let total = Resource::new(
        move || filters.get(),
        |f| async move { count_tools(f).await },
    );
    let tools = Resource::new(
        move || (sort.get(), filters.get(), search_q.get()),
        |(sort, filters, q)| async move { list_tools(sort, 0, 20, filters, q).await },
    );
    let preview_tool = Resource::new(
        move || selected.get(),
        |slug| async move {
            match slug {
                Some(s) if !s.is_empty() => get_tool_by_slug(s).await,
                _ => Err(ServerFnError::new("no selection")),
            }
        },
    );

    view! {
        <TopNav/>
        <div class="tools-layout">
            <Suspense fallback=|| view! { <aside class="tools-sidebar"><p>"..."</p></aside> }>
                {move || {
                    let (cats_res, chains_res) = (categories.get(), chains.get());
                    match (cats_res, chains_res) {
                        (Some(Ok(cats)), Some(Ok(chain_opts))) => view! {
                            <Sidebar
                                categories=cats
                                query_base=query_base.get()
                                active_function=function.get()
                                active_asset_class=asset_class.get()
                                active_actor=actor.get()
                                active_type=tool_type.get()
                                active_status=status.get()
                                active_chain=chain.get()
                                chain_options=chain_opts
                            />
                        }
                        .into_any(),
                        _ => view! { <aside class="tools-sidebar"/> }.into_any(),
                    }
                }}
            </Suspense>
            <div class="tools-main">
                <div class="tools-toolbar sticky-toolbar">
                    <form action="/tools" method="get" class="toolbar-search">
                        <input
                            type="search"
                            name="q"
                            placeholder="Search tools..."
                            prop:value=move || search_q.get().unwrap_or_default()
                        />
                        {move || function.get().map(|f| view! { <input type="hidden" name="function" value=f/> })}
                        {move || asset_class.get().map(|v| view! { <input type="hidden" name="asset_class" value=v/> })}
                        {move || actor.get().map(|v| view! { <input type="hidden" name="actor" value=v/> })}
                        {move || tool_type.get().map(|v| view! { <input type="hidden" name="type" value=v/> })}
                        {move || status.get().map(|v| view! { <input type="hidden" name="status" value=v/> })}
                        {move || chain.get().map(|v| view! { <input type="hidden" name="chain" value=v/> })}
                        <input type="hidden" name="sort" prop:value=move || sort.get()/>
                    </form>
                    <div class="toolbar-sort">
                        <a href="/tools?sort=hot" class="sort-link">"HOT"</a>
                        <a href="/tools?sort=new" class="sort-link">"New"</a>
                        <a href="/tools?sort=comments" class="sort-link">"Comments"</a>
                    </div>
                    <Suspense fallback=|| view! { <span class="tool-count">"..."</span> }>
                        {move || {
                            total.get().map(|res| match res {
                                Ok(n) => view! { <span class="tool-count">{n}" tools"</span> }.into_any(),
                                Err(_) => view! { <span class="tool-count">"— tools"</span> }.into_any(),
                            })
                        }}
                    </Suspense>
                </div>
                <Suspense fallback=|| view! { <p class="text-[#6B6B6B]">"Loading..."</p> }>
                    {move || {
                        tools.get().map(|res| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <p class="empty-state">"No tools match your filters."</p> }.into_any()
                            }
                            Ok(list) => {
                                let base = query_base.get();
                                view! {
                                    <div class="tool-list">
                                        {list
                                            .into_iter()
                                            .map(|t| {
                                                let slug = t.slug.clone();
                                                let preview = with_selected(&base, &slug);
                                                view! { <ToolCard tool=t preview_href=preview/> }
                                            })
                                            .collect_view()}
                                    </div>
                                }
                                .into_any()
                            }
                            Err(_) => view! { <p class="empty-state">"Failed to load tools."</p> }.into_any(),
                        })
                    }}
                </Suspense>
            </div>
        </div>

        {move || {
            selected.get().and_then(|_| preview_tool.get()).map(|res| match res {
                Ok(tool) => {
                    let base = query_base.get();
                    let close = without_selected(&base);
                    let full = format!("/tools/{}", tool.slug);
                    view! {
                        <div class="preview-desktop">
                            <PreviewPanel tool=tool.clone() close_href=close.clone() full_page_href=full.clone()/>
                        </div>
                        <div class="preview-mobile">
                            <BottomSheet tool=tool close_href=close full_page_href=full/>
                        </div>
                    }
                    .into_any()
                }
                Err(_) => ().into_any(),
            })
        }}
    }
}