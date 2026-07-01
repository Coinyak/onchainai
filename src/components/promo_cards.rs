//! Registration promo cards on the home page.

use crate::components::copy_button::CopyButton;
use crate::components::highlighted_command::HighlightedCommand;
use leptos::prelude::*;

#[component]
pub fn PromoCards(mcp_endpoint: String) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 min-w-0">
            <div class="promo-card border border-[#E5E5E5] rounded-lg p-6 bg-white">
                <h3 class="text-[16px] font-semibold mb-2">"Suggest a Tool"</h3>
                <p class="text-[14px] text-[#6B6B6B] mb-4 leading-relaxed">
                    "Know a crypto MCP, CLI, SDK, API, or x402 tool we should review? Send it for operator review before it appears publicly."
                </p>
                <a
                    href="/submit"
                    class="inline-flex items-center justify-center h-10 px-4 rounded-lg bg-[#E76F00] text-white text-[14px] font-medium no-underline hover:bg-[#D96400]"
                >
                    "Suggest →"
                </a>
            </div>
            <div class="promo-card border border-[#E5E5E5] rounded-lg p-6 bg-white min-w-0">
                <h3 class="text-[16px] font-semibold mb-2">"Connect via MCP"</h3>
                <p class="text-[14px] text-[#6B6B6B] mb-3 leading-relaxed">
                    "Ask your AI: Find crypto tools on onchain-ai."
                </p>
                <div class="flex items-center gap-2 min-w-0">
                    <HighlightedCommand text=mcp_endpoint.clone()/>
                    <CopyButton text=mcp_endpoint/>
                </div>
            </div>
        </div>
    }
}
