//! Registration promo cards on the home page.

use crate::components::copy_button::CopyButton;
use leptos::prelude::*;

#[component]
pub fn PromoCards(mcp_endpoint: String) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
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
            <div class="promo-card border border-[#E5E5E5] rounded-lg p-6 bg-white">
                <h3 class="text-[16px] font-semibold mb-2">"Connect via MCP"</h3>
                <p class="text-[14px] text-[#6B6B6B] mb-3 leading-relaxed">
                    "Connect your agent to OnchainAI MCP and search tools instantly."
                </p>
                <div class="flex items-center gap-2">
                    <code class="font-mono text-[13px] bg-[#F5F5F0] border border-[#E5E5E5] rounded-md px-3 py-2 flex-1 overflow-x-auto">
                        <span class="text-[#999999]">"$ "</span>{mcp_endpoint.clone()}
                    </code>
                    <CopyButton text=mcp_endpoint.clone()/>
                </div>
            </div>
        </div>
    }
}
