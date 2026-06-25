//! Mobile bottom sheet — 60% slide-up overlay for tool quick view.

use crate::components::tool_detail_content::ToolDetailContent;
use crate::models::Tool;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn BottomSheet(
    tool: Tool,
    close_href: String,
    full_page_href: String,
) -> impl IntoView {
    view! {
        <A href=close_href.clone() attr:class="bottom-sheet-backdrop" attr:aria-label="Close preview">
            <span class="sr-only">"Close"</span>
        </A>
        <div class="bottom-sheet" role="dialog" aria-label="Tool preview">
            <div class="bottom-sheet-handle" aria-hidden="true"></div>
            <div class="bottom-sheet-body">
                <ToolDetailContent tool=tool compact=true full_page_href=full_page_href/>
            </div>
        </div>
    }
}