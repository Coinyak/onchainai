//! Sticky top navigation — UI spec: Logo + Submit + GitHub (+ admin session).

use crate::auth::session::SessionUser;
use crate::server::functions::get_current_user;
use leptos::prelude::*;

const GITHUB_REPO: &str = "https://github.com/hoyeon4315-cpu/onchainai";

#[component]
fn AuthNav(
    user_res: Result<Option<SessionUser>, leptos::server_fn::ServerFnError>,
) -> impl IntoView {
    match user_res {
        Ok(Some(session)) if session.is_admin => view! {
            <a href="/admin" class="text-[#E76F00] hover:underline no-underline font-medium">
                "Admin"
            </a>
            <span class="text-[#6B6B6B] hidden sm:inline">
                {session.nickname.clone().unwrap_or_else(|| "admin".into())}
            </span>
            <form action="/auth/logout" method="post" class="inline">
                <button
                    type="submit"
                    class="text-[#6B6B6B] hover:text-[#1A1A1A] bg-transparent border-0 cursor-pointer text-[14px] p-0"
                >
                    "Sign out"
                </button>
            </form>
        }
        .into_any(),
        Ok(Some(_session)) => view! {
            <form action="/auth/logout" method="post" class="inline">
                <button
                    type="submit"
                    class="text-[#6B6B6B] hover:text-[#1A1A1A] bg-transparent border-0 cursor-pointer text-[14px] p-0"
                >
                    "Sign out"
                </button>
            </form>
        }
        .into_any(),
        Ok(None) | Err(_) => ().into_any(),
    }
}

/// Site logo + primary actions — rendered at the top of the left sidebar.
#[component]
pub fn SidebarBrand() -> impl IntoView {
    view! {
        <div class="sidebar-brand">
            <a
                href="/"
                class="sidebar-brand-logo text-[16px] font-semibold tracking-tight text-[#1A1A1A] no-underline"
            >
                "OnchainAI"
            </a>
            <nav class="sidebar-brand-nav">
                <a
                    href="/submit"
                    class="sidebar-brand-submit inline-flex items-center justify-center h-8 px-3 rounded-lg bg-[#E76F00] text-white text-[13px] font-medium no-underline hover:bg-[#D96400]"
                >
                    "Submit"
                </a>
                <a
                    href=GITHUB_REPO
                    target="_blank"
                    rel="noopener noreferrer"
                    class="sidebar-brand-link text-[#6B6B6B] hover:text-[#1A1A1A] no-underline text-[13px]"
                >
                    "GitHub"
                </a>
                <Await future=async move { get_current_user().await } let:user_res blocking=true>
                    <div class="sidebar-brand-auth">
                        <AuthNav user_res=user_res.clone()/>
                    </div>
                </Await>
            </nav>
        </div>
    }
}

/// Legacy horizontal header — unused; site uses `SidebarBrand` in the left sidebar.
#[component]
pub fn TopNav() -> impl IntoView {
    view! {
        <header class="sticky top-0 z-50 bg-white border-b border-[#E5E5E5]">
            <div class="max-w-[1200px] mx-auto px-4 md:px-6 h-12 md:h-14 flex items-center justify-between">
                <a href="/" class="text-[16px] font-semibold tracking-tight text-[#1A1A1A] no-underline">
                    "OnchainAI"
                </a>
                <nav class="flex items-center gap-2 md:gap-5 text-[14px]">
                    <a
                        href="/submit"
                        class="inline-flex items-center justify-center h-8 md:h-9 px-3 md:px-4 rounded-lg bg-[#E76F00] text-white text-[13px] md:text-[14px] font-medium no-underline hover:bg-[#D96400]"
                    >
                        "Submit"
                    </a>
                    <a
                        href=GITHUB_REPO
                        target="_blank"
                        rel="noopener noreferrer"
                        class="hidden sm:inline text-[#6B6B6B] hover:text-[#1A1A1A] no-underline"
                    >
                        "GitHub"
                    </a>
                    <Await future=async move { get_current_user().await } let:user_res blocking=true>
                        <AuthNav user_res=user_res.clone()/>
                    </Await>
                </nav>
            </div>
        </header>
    }
}
