//! Category page — tools filtered by category id.

use crate::components::{tool_card::ToolCard, top_nav::TopNav};
use crate::server::functions::{get_categories, list_tools, ToolFilters};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn CategoryPage() -> impl IntoView {
    let params = use_params_map();
    let cat_id = Memo::new(move |_| params.with(|p| p.get("id").unwrap_or_default()));

    let categories =
        Resource::new(|| (), |_| async move { get_categories().await });
    let tools = Resource::new(
        move || cat_id.get(),
        |id| async move {
            if id.is_empty() {
                Err(ServerFnError::new("missing category"))
            } else {
                list_tools(
                    "hot".into(),
                    0,
                    50,
                    ToolFilters {
                        function: Some(id),
                        ..Default::default()
                    },
                    None,
                )
                .await
            }
        },
    );

    view! {
        <TopNav/>
        <div class="max-w-[960px] mx-auto px-4 py-8">
            <Suspense fallback=|| view! { <h1>"Loading..."</h1> }>
                {move || {
                    let id = cat_id.get();
                    categories.get().map(|res| match res {
                        Ok(cats) => {
                            let label = cats
                                .iter()
                                .find(|(c, _)| c.id == id)
                                .map(|(c, _)| c.label.clone())
                                .unwrap_or_else(|| "Category".into());
                            if cats.iter().any(|(c, _)| c.id == id) {
                                view! { <h1 class="text-[28px] font-bold mb-4">{label}</h1> }.into_any()
                            } else {
                                view! {
                                    <h1 class="text-[28px] font-bold">"404"</h1>
                                    <p class="text-[#6B6B6B]">"Category not found."</p>
                                }
                                .into_any()
                            }
                        }
                        Err(_) => view! { <h1>"Category"</h1> }.into_any(),
                    })
                }}
            </Suspense>
            <Suspense fallback=|| view! { <p>"Loading tools..."</p> }>
                {move || {
                    tools.get().map(|res| match res {
                        Ok(list) => view! {
                            <div class="tool-list">
                                {list.into_iter().map(|t| view! { <ToolCard tool=t/> }).collect_view()}
                            </div>
                        }
                        .into_any(),
                        Err(_) => view! { <p class="text-[#6B6B6B]">"Tools unavailable."</p> }
                            .into_any(),
                    })
                }}
            </Suspense>
        </div>
    }
}