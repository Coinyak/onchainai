//! Registration promo cards on the home page.

use leptos::prelude::*;

#[component]
pub fn PromoCards(mcp_endpoint: String) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="promo-card border border-[#E5E5E5] rounded-lg p-5 bg-[#FAFAFA]">
                <h3 class="text-[16px] font-semibold mb-2">"Submit a Tool"</h3>
                <p class="text-[14px] text-[#6B6B6B] mb-4 leading-relaxed">
                    "List your crypto MCP, CLI, or SDK so humans and agents can discover it."
                </p>
                <a
                    href="/about"
                    class="inline-flex items-center justify-center h-10 px-4 rounded-lg bg-[#E76F00] text-white text-[14px] font-medium no-underline hover:opacity-90"
                >
                    "Learn how"
                </a>
            </div>
            <div class="promo-card border border-[#E5E5E5] rounded-lg p-5 bg-[#FAFAFA]">
                <h3 class="text-[16px] font-semibold mb-2">"Connect via MCP"</h3>
                <p class="text-[14px] text-[#6B6B6B] mb-3 leading-relaxed">
                    "Let agents search this directory from your IDE."
                </p>
                <div class="flex items-center gap-2">
                    <code class="font-mono text-[13px] bg-white border border-[#E5E5E5] rounded-md px-3 py-2 flex-1 overflow-x-auto">
                        {mcp_endpoint.clone()}
                    </code>
                    <button
                        type="button"
                        class="copy-btn shrink-0 h-10 px-3 rounded-lg border border-[#E5E5E5] bg-white text-[13px] font-medium text-[#1A1A1A] hover:bg-[#FAFAFA]"
                        data-copy=mcp_endpoint.clone()
                    >
                        "Copy"
                    </button>
                </div>
            </div>
        </div>
    }
}