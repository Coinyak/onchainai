//! Desktop preview panel — 400px right slide-in; ESC + backdrop click to close.

use crate::components::tool_detail_content::ToolDetailContent;
use crate::models::Tool;
use leptos::prelude::*;

#[component]
pub fn PreviewPanel(tool: Tool, close_href: String, full_page_href: String) -> impl IntoView {
    let close_backdrop = close_href.clone();
    let close_button = close_href.clone();
    view! {
        <a href=close_backdrop class="preview-backdrop" aria-label="Close preview">
            <span class="sr-only">"Close"</span>
        </a>
        <aside
            class="preview-panel"
            role="dialog"
            aria-label="Tool preview"
            tabindex="-1"
            on:keydown={
                let close = close_href.clone();
                move |ev| {
                    if ev.key() == "Escape" {
                        #[cfg(feature = "hydrate")]
                        if let Some(win) = web_sys::window() {
                            let _ = win.location().set_href(&close);
                        }
                    }
                }
            }
        >
            <div class="preview-panel-header">
                <a href=close_button class="preview-close" aria-label="Close preview">
                    "×"
                </a>
            </div>
            <div class="preview-panel-body">
                <ToolDetailContent tool=tool compact=true full_page_href=full_page_href/>
            </div>
        </aside>
    }
}
