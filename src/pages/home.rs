//! Home page — hero, search, promo cards, category grid, full sidebar + HOT list.

use crate::components::category_grid::CategoryGrid;
use crate::components::promo_cards::PromoCards;
use crate::components::search_bar::SearchBar;
use crate::components::tools_browser::{BrowserBase, ToolsBrowser};
use crate::components::top_nav::TopNav;
use crate::config::MCP_ENDPOINT_CMD;
use crate::models::{Category, SiteSettings};
use crate::server::functions::{get_categories, get_site_settings};
use leptos::prelude::*;

async fn load_home_header() -> (SiteSettings, Vec<(Category, i64)>) {
    // Independent queries — run concurrently to save a DB round-trip (the DB is
    // remote; latency dominates).
    let (settings, categories) = futures::join!(get_site_settings(), get_categories());
    let settings = settings.unwrap_or_else(|_| SiteSettings {
        id: 1,
        site_name: "OnchainAI".into(),
        slogan: "Crypto tools, unified.".into(),
        description: "Discover, install, and share crypto MCP, CLI, SDK, API, x402, RWA, and AI agent tools — all in one place. Search as a human, or let your agent search via MCP.".into(),
        mcp_endpoint: MCP_ENDPOINT_CMD.into(),
        search_keywords: vec![],
        allow_free_registration: true,
        require_tool_approval: true,
        allow_x402_registration: false,
        updated_at: chrono::Utc::now(),
    });
    let categories = categories.unwrap_or_default();
    (settings, categories)
}

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <TopNav/>
        <Await future=load_home_header() let:data blocking=true>
            <div class="home-page max-w-[1200px] mx-auto px-4 md:px-6 py-8 md:py-12">
                <section class="hero mb-8">
                    <h1 class="text-[28px] md:text-[36px] font-bold tracking-tight leading-tight mb-3">
                        {data.0.slogan.clone()}
                    </h1>
                    <p class="text-[#6B6B6B] text-[14px] md:text-[16px] leading-relaxed mb-6 max-w-[720px]">
                        {data.0.description.clone()}
                    </p>
                    <SearchBar/>
                </section>
                <section class="mb-10">
                    <PromoCards mcp_endpoint=data.0.mcp_endpoint.clone()/>
                </section>
                <CategoryGrid categories=data.1.clone() base=BrowserBase::Home/>
            </div>
            <section class="home-tools-section border-t border-[#E5E5E5]">
                <ToolsBrowser base=BrowserBase::Home show_toolbar_search=false/>
            </section>
        </Await>
    }
}
