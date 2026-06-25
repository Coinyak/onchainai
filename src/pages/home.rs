//! Home page — hero, search, promo cards, category grid, HOT tool list.

use crate::components::{
    category_grid::CategoryGrid, promo_cards::PromoCards, search_bar::SearchBar,
    tool_card::ToolCard, top_nav::TopNav,
};
use crate::models::{Category, SiteSettings, Tool};
use crate::server::functions::{get_categories, get_recent_tools, get_site_settings};
use leptos::prelude::*;

async fn load_home_data() -> (SiteSettings, Vec<(Category, i64)>, Vec<Tool>) {
    let settings = get_site_settings().await.unwrap_or_else(|_| SiteSettings {
        id: 1,
        site_name: "OnchainAI".into(),
        slogan: "Crypto tools, unified.".into(),
        description: "Discover, install, and share crypto MCP, CLI, SDK, API, x402, RWA, and AI agent tools — all in one place.".into(),
        mcp_endpoint: "npx mcp-remote onchainai.xyz/mcp".into(),
        search_keywords: vec![],
        allow_free_registration: true,
        require_tool_approval: true,
        allow_x402_registration: false,
        updated_at: chrono::Utc::now(),
    });
    let categories = get_categories().await.unwrap_or_default();
    let tools = get_recent_tools(12).await.unwrap_or_default();
    (settings, categories, tools)
}

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <TopNav/>
        <Await future=load_home_data() let:data blocking=true>
            <div class="home-page max-w-[1200px] mx-auto px-4 md:px-6 py-8 md:py-12">
                <section class="hero mb-8">
                    <h1 class="text-[28px] md:text-[28px] font-bold tracking-tight leading-tight mb-3">
                        {data.0.slogan.clone()}
                    </h1>
                    <p class="text-[#6B6B6B] text-[14px] md:text-[14px] leading-relaxed mb-6 max-w-[640px]">
                        {data.0.description.clone()}
                    </p>
                    <SearchBar/>
                </section>
                <section class="mb-10">
                    <PromoCards mcp_endpoint=data.0.mcp_endpoint.clone()/>
                </section>
                <CategoryGrid categories=data.1.clone()/>
                <section>
                    <h2 class="text-[20px] font-semibold mb-4">"Popular tools"</h2>
                    {if data.2.is_empty() {
                        view! { <p class="text-[#6B6B6B]">"No tools yet. Check back soon."</p> }.into_any()
                    } else {
                        view! {
                            <div class="tool-list">
                                {data.2.clone().into_iter().map(|t| view! { <ToolCard tool=t/> }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </section>
            </div>
        </Await>
    }
}