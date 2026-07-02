//! Home page — hero, search, promo cards, sidebar + HOT list.

use crate::components::featured_carousel::FeaturedCarousel;
use crate::components::promo_cards::PromoCards;
use crate::components::search_bar::SearchBar;
use crate::components::tool_finder::ToolFinderPanel;
use crate::components::tools_browser::{BrowserBase, ToolsBrowser};
use crate::config::MCP_ENDPOINT_CMD;
use crate::models::SiteSettings;
use crate::server::functions::{get_featured_cards, get_site_settings, FeaturedCardView};
use leptos::prelude::*;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct HomeHeaderData {
    settings: SiteSettings,
    featured: Vec<FeaturedCardView>,
}

async fn load_home_header() -> HomeHeaderData {
    let (settings, featured) = futures::join!(get_site_settings(), get_featured_cards());
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
        default_referral_bps: None,
        default_referral_payout_address: None,
        x402_builder_code: None,
        hero_title: None,
        hero_subtitle: None,
        about_content: None,
        footer_links: vec![],
        updated_at: chrono::Utc::now(),
    });
    let featured = featured.unwrap_or_default();
    HomeHeaderData { settings, featured }
}

#[component]
fn HomeHeroContent(
    slogan: String,
    description: String,
    mcp_endpoint: String,
    featured: Vec<FeaturedCardView>,
) -> impl IntoView {
    view! {
        <div class="home-page px-4 md:px-6 py-8 md:py-10">
            <section class="hero mb-8">
                <h1 class="text-[28px] md:text-[36px] font-bold tracking-tight leading-tight mb-3">
                    {slogan}
                </h1>
                <p class="text-[#6B6B6B] text-[14px] md:text-[16px] leading-relaxed mb-6 max-w-[720px]">
                    {description}
                </p>
                <SearchBar/>
                <ToolFinderPanel/>
            </section>
            <FeaturedCarousel cards=featured/>
            <section class="mb-6">
                <PromoCards mcp_endpoint=mcp_endpoint/>
            </section>
        </div>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    let header = Resource::new_blocking(|| (), |_| async move { load_home_header().await });

    view! {
        <ToolsBrowser base=BrowserBase::Home show_toolbar_search=false>
            {move || header.get().map(|data| view! {
                <HomeHeroContent
                    slogan=data.settings.slogan.clone()
                    description=data.settings.description.clone()
                    mcp_endpoint=data.settings.mcp_endpoint.clone()
                    featured=data.featured.clone()
                />
            })}
        </ToolsBrowser>
    }
}
