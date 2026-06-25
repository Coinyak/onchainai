//! Tools list page — sidebar, sort bar, search, tool cards.

use crate::components::{sidebar::Sidebar, tool_card::ToolCard, top_nav::TopNav};
use crate::server::functions::{count_tools, get_categories, list_tools};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

#[component]
pub fn ToolsListPage() -> impl IntoView {
    let query = use_query_map();
    let function = Memo::new(move |_| query.with(|q| q.get("function").map(|s| s.to_string())));
    let sort = Memo::new(move |_| {
        query
            .with(|q| q.get("sort").map(|s| s.to_string()))
            .unwrap_or_else(|| "hot".into())
    });
    let search_q = Memo::new(move |_| query.with(|q| q.get("q").map(|s| s.to_string())));

    let categories =
        Resource::new(|| (), |_| async move { get_categories().await });
    let total = Resource::new(
        move || function.get(),
        |f| async move { count_tools(f).await },
    );
    let tools = Resource::new(
        move || (sort.get(), function.get(), search_q.get()),
        |(sort, function, q)| async move {
            list_tools(sort, 0, 20, function, None, q).await
        },
    );

    view! {
        <TopNav/>
        <div class="tools-layout">
            <Suspense fallback=|| view! { <aside class="tools-sidebar"><p>"..."</p></aside> }>
                {move || {
                    categories.get().map(|res| match res {
                        Ok(cats) => view! {
                            <Sidebar categories=cats active_function=function.get()/>
                        }
                        .into_any(),
                        Err(_) => view! { <aside class="tools-sidebar"/> }.into_any(),
                    })
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
                        {move || {
                            function
                                .get()
                                .map(|f| view! { <input type="hidden" name="function" value=f/> })
                        }}
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
                                Ok(n) => view! { <span class="tool-count">{n}" tools"</span> }
                                    .into_any(),
                                Err(_) => view! { <span class="tool-count">"— tools"</span> }
                                    .into_any(),
                            })
                        }}
                    </Suspense>
                </div>
                <Suspense fallback=|| view! { <p class="text-[#6B6B6B]">"Loading..."</p> }>
                    {move || {
                        tools.get().map(|res| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <p class="empty-state">"No tools match your filters."</p> }
                                    .into_any()
                            }
                            Ok(list) => view! {
                                <div class="tool-list">
                                    {list.into_iter().map(|t| view! { <ToolCard tool=t/> }).collect_view()}
                                </div>
                            }
                            .into_any(),
                            Err(_) => view! { <p class="empty-state">"Failed to load tools."</p> }
                                .into_any(),
                        })
                    }}
                </Suspense>
            </div>
        </div>
    }
}