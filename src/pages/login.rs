//! Login page — SSR-visible sign-in options (no client hydration required).

use crate::components::top_nav::TopNav;
use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <TopNav/>
        <div class="max-w-[420px] mx-auto px-4 py-12">
            <h1 class="text-[24px] font-semibold mb-2">"Sign in"</h1>
            <p class="text-[14px] text-[#6B6B6B] mb-8">
                "Sign in to comment, bookmark tools, and access admin features."
            </p>
            <a
                href="/auth/github"
                class="flex items-center justify-center w-full px-4 py-2.5 rounded-lg bg-[#1A1A1A] text-white text-[14px] font-medium hover:opacity-90 no-underline"
            >
                "Continue with GitHub"
            </a>
            <form id="email-login-form" class="mt-3 flex gap-2">
                <input
                    id="email-login-input"
                    type="email"
                    autocomplete="email"
                    placeholder="you@example.com"
                    class="flex-1 rounded-lg border border-[#E5E5E5] px-3 py-2.5 text-[14px]"
                />
                <button
                    type="submit"
                    class="px-3 py-2.5 rounded-lg border border-[#E5E5E5] text-[14px] font-medium hover:bg-[#FAFAFA]"
                >
                    "Email"
                </button>
            </form>
            <p id="email-login-msg" class="mt-2 text-[13px] text-[#6B6B6B] hidden"></p>
            <button
                type="button"
                id="siwx-connect-btn"
                class="flex items-center justify-center w-full mt-3 px-4 py-2.5 rounded-lg border border-[#E5E5E5] text-[14px] font-medium hover:bg-[#FAFAFA]"
            >
                "Connect Wallet (SIWX)"
            </button>
            <p id="siwx-error" class="mt-2 text-[13px] text-[#C0392B] hidden"></p>
        </div>
    }
}