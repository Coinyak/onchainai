//! Desktop preview panel — 400px right slide-in; ESC + backdrop click to close.

use crate::components::tool_detail_content::ToolDetailContent;
use crate::models::Tool;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn PreviewPanel(tool: Tool, close_href: String, full_page_href: String) -> impl IntoView {
    let close = close_href.clone();
    view! {
        <A href=close_href.clone() attr:class="preview-backdrop" attr:aria-label="Close preview">
            <span class="sr-only">"Close"</span>
        </A>
        <aside
            class="preview-panel"
            role="dialog"
            aria-label="Tool preview"
            tabindex="-1"
            on:keydown=move |ev| {
                if ev.key() == "Escape" {
                    let win = window();
                    let _ = win.location().set_href(&close);
                }
            }
        >
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
