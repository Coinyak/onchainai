//! Empty state with Submit CTA — UI_UX_DESIGN §6.

use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn EmptyState(
    #[prop(default = "No tools match your filters.")]
    message: &'static str,
) -> impl IntoView {
    view! {
        <div class="empty-state-panel">
            <p class="empty-state-message">{message}</p>
            <A
                href="/about#submit"
                attr:class="inline-flex items-center justify-center h-9 px-4 rounded-lg bg-[#E76F00] text-white text-[14px] font-medium no-underline hover:opacity-90"
            >
                "Submit a tool"
            </A>
        </div>
    }
}