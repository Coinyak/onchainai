//! Full-width search bar — submits to `/tools` via GET.

use leptos::prelude::*;

#[component]
pub fn SearchBar() -> impl IntoView {
    view! {
        <form action="/tools" method="get" class="w-full">
            <input
                type="search"
                name="q"
                placeholder="Search crypto MCP, CLI, SDK, API tools..."
                class="search-input w-full h-12 px-4 text-[14px] rounded-lg border border-[#E5E5E5] bg-white text-[#1A1A1A] outline-none focus:border-[#E76F00] focus:ring-2 focus:ring-[#E76F00]/20"
                autocomplete="off"
            />
        </form>
    }
}