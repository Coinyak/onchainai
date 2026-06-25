//! Home page — hero, search, promo cards, category grid, HOT tool list.

use crate::components::{
    category_grid::CategoryGrid, promo_cards::PromoCards, search_bar::SearchBar,
    tool_card::ToolCard, top_nav::TopNav,
};
use crate::models::{Category, SiteSettings, Tool};
use crate::server::functions::{get_categories, get_recent_tools, get_site_settings};
use leptos::prelude::*;

#[component]
pub fn HomePage() -> impl IntoView {
    let settings = Resource::new(|| (), |_| async move { get_site_settings().await });
    let categories =
        Resource::new(|| (), |_| async move { get_categories().await });
    let tools = Resource::new(|| (), |_| async move { get_recent_tools(12).await });

    view! {
        <TopNav/>
        <div class="home-page max-w-[1200px] mx-auto px-4 md:px-6 py-8 md:py-12">
            <Suspense fallback=|| view! { <p class="text-[#6B6B6B]">"Loading..."</p> }>
                {move || {
                    settings.get().map(|res: Result<SiteSettings, ServerFnError>| match res {
                        Ok(s) => view! {
                            <section class="hero mb-8">
                                <h1 class="text-[28px] md:text-[28px] font-bold tracking-tight leading-tight mb-3">
                                    {s.slogan.clone()}
                                </h1>
                                <p class="text-[#6B6B6B] text-[14px] md:text-[14px] leading-relaxed mb-6 max-w-[640px]">
                                    {s.description.clone()}
                                </p>
                                <SearchBar/>
                            </section>
                            <section class="mb-10">
                                <PromoCards mcp_endpoint=s.mcp_endpoint.clone()/>
                            </section>
                        }.into_any(),
                        Err(e) => view! {
                            <section class="hero mb-8">
                                <h1 class="text-[28px] font-bold tracking-tight leading-tight mb-3">
                                    "Crypto tools, unified."
                                </h1>
                                <p class="text-[#6B6B6B] text-[14px] leading-relaxed mb-6">
                                    {format!("Failed to load settings: {e}")}
                                </p>
                                <SearchBar/>
                            </section>
                            <PromoCards mcp_endpoint="npx mcp-remote onchainai.xyz/mcp".to_string()/>
                        }.into_any(),
                    })
                }}
            </Suspense>

            <Suspense fallback=|| view! { <p class="text-[#6B6B6B]">"Loading categories..."</p> }>
                {move || {
                    categories
                        .get()
                        .map(|res: Result<Vec<(Category, i64)>, ServerFnError>| match res {
                            Ok(cats) => view! { <CategoryGrid categories=cats/> }.into_any(),
                            Err(_) => view! { <p class="text-[#6B6B6B]">"Categories unavailable."</p> }
                                .into_any(),
                        })
                }}
            </Suspense>

            <section>
                <h2 class="text-[20px] font-semibold mb-4">"Popular tools"</h2>
                <Suspense fallback=|| view! { <p class="text-[#6B6B6B]">"Loading tools..."</p> }>
                    {move || {
                        tools.get().map(|res: Result<Vec<Tool>, ServerFnError>| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <p class="text-[#6B6B6B]">"No tools yet. Check back soon."</p> }
                                    .into_any()
                            }
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
            </section>
        </div>
    }
}