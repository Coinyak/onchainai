//! Desktop preview panel — 400px right slide-in for tool quick view.

use crate::components::tool_detail_content::ToolDetailContent;
use crate::models::Tool;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn PreviewPanel(
    tool: Tool,
    close_href: String,
    full_page_href: String,
) -> impl IntoView {
    view! {
        <div class="preview-backdrop" aria-hidden="true"></div>
        <aside class="preview-panel" role="dialog" aria-label="Tool preview">
            <div class="preview-panel-header">
                <A href=close_href attr:class="preview-close" attr:aria-label="Close preview">
                    "×"
                </A>
            </div>
            <div class="preview-panel-body">
                <ToolDetailContent tool=tool compact=true full_page_href=full_page_href/>
            </div>
        </aside>
    }
}