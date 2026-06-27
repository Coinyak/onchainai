//! Registration promo cards on the home page.

use crate::components::copy_button::CopyButton;
use crate::config::mcp_remote_url_from_command;
use leptos::prelude::*;

#[component]
pub fn PromoCards(mcp_endpoint: String) -> impl IntoView {
    let mcp_url = mcp_remote_url_from_command(&mcp_endpoint);
    let mcp_json = format!(
        r#"{{
  "mcpServers": {{
    "onchainai": {{
      "command": "npx",
      "args": ["mcp-remote", "{mcp_url}"]
    }}
  }}
}}"#
    );
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
                <div class="flex items-center gap-2 mb-3 min-w-0">
                    <code class="font-mono text-[13px] bg-[#F5F5F0] border border-[#E5E5E5] rounded-md px-3 py-2 flex-1 min-w-0 overflow-x-auto break-all">
                        <span class="text-[#999999]">"$ "</span>{mcp_endpoint.clone()}
                    </code>
                    <CopyButton text=mcp_endpoint.clone()/>
                </div>
                <div class="flex items-start gap-2 min-w-0">
                    <pre class="font-mono text-[12px] bg-[#F5F5F0] border border-[#E5E5E5] rounded-md px-3 py-2 flex-1 min-w-0 overflow-x-auto whitespace-pre-wrap break-all">
                        {mcp_json.clone()}
                    </pre>
                    <CopyButton text=mcp_json/>
                </div>
            </div>
        </div>
    }
}
