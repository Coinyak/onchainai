//! Sticky top navigation.

use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn TopNav() -> impl IntoView {
    view! {
        <header class="sticky top-0 z-50 bg-white border-b border-[#E5E5E5]">
            <div class="max-w-[1200px] mx-auto px-4 md:px-6 h-14 flex items-center justify-between">
                <A href="/" attr:class="text-[16px] font-semibold tracking-tight text-[#1A1A1A] no-underline">
                    "OnchainAI"
                </A>
                <nav class="flex items-center gap-6 text-[14px]">
                    <A href="/tools" attr:class="text-[#6B6B6B] hover:text-[#1A1A1A] no-underline">
                        "Tools"
                    </A>
                    <A href="/about" attr:class="text-[#6B6B6B] hover:text-[#1A1A1A] no-underline">
                        "About"
                    </A>
                </nav>
            </div>
        </header>
    }
}