//! Category page — tools filtered by category id.

use crate::components::{tool_card::ToolCard, top_nav::TopNav};
use crate::models::{Category, Tool};
use crate::server::functions::{get_categories, get_tool_comment_counts, list_tools, ToolFilters};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use std::collections::HashMap;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct CategoryPageData {
    categories: Vec<(Category, i64)>,
    tools: Vec<Tool>,
    comment_counts: HashMap<String, i64>,
    cat_id: String,
}

async fn load_category_page(cat_id: String) -> CategoryPageData {
    let categories = get_categories().await.unwrap_or_default();
    let tools = if cat_id.is_empty() {
        Vec::new()
    } else {
        list_tools(
            "hot".into(),
            0,
            50,
            ToolFilters {
                function: vec![cat_id.clone()],
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap_or_default()
    };
    let slugs = tools.iter().map(|t| t.slug.clone()).collect();
    let comment_counts: HashMap<String, i64> = get_tool_comment_counts(slugs)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();
    CategoryPageData {
        categories,
        tools,
        comment_counts,
        cat_id,
    }
}

#[component]
pub fn CategoryPage() -> impl IntoView {
    let params = use_params_map();
    let cat_id = Memo::new(move |_| params.with(|p| p.get("id").unwrap_or_default()));

    let page = Resource::new_blocking(
        move || cat_id.get(),
        |id| async move { load_category_page(id).await },
    );

    view! {
        <TopNav/>
        <div class="max-w-[960px] mx-auto px-4 py-8">
            {move || {
                page.get().map(|data| {
                    let label = data
                        .categories
                        .iter()
                        .find(|(c, _)| c.id == data.cat_id)
                        .map(|(c, _)| c.label.clone())
                        .unwrap_or_else(|| "Category".into());
                    let found = data.categories.iter().any(|(c, _)| c.id == data.cat_id);
                    view! {
                        {if found {
                            view! { <h1 class="text-[28px] font-bold mb-4">{label}</h1> }.into_any()
                        } else if !data.cat_id.is_empty() {
                            view! {
                                <h1 class="text-[28px] font-bold">"404"</h1>
                                <p class="text-[#6B6B6B]">"Category not found."</p>
                            }
                            .into_any()
                        } else {
                            view! { <h1 class="text-[28px] font-bold mb-4">"Category"</h1> }.into_any()
                        }}
                        {if found {
                            let comment_counts = data.comment_counts.clone();
                            view! {
                                <div class="tool-list">
                                    {data.tools.clone().into_iter().map(|t| {
                                        let count = comment_counts.get(&t.slug).copied().unwrap_or(0);
                                        view! { <ToolCard tool=t comment_count=count/> }
                                    }).collect_view()}
                                </div>
                            }
                            .into_any()
                        } else {
                            ().into_any()
                        }}
                    }
                })
            }}
        </div>
    }
}
