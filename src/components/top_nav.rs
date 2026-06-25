//! Sticky top navigation — UI spec: Logo + Submit + GitHub (+ admin session).

use crate::auth::session::SessionUser;
use crate::server::functions::get_current_user;
use leptos::prelude::*;
use leptos_router::components::A;

const GITHUB_REPO: &str = "https://github.com/hoyeon4315-cpu/onchainai";

#[component]
fn AuthNav(user_res: Result<Option<SessionUser>, leptos::server_fn::ServerFnError>) -> impl IntoView {
    match user_res {
        Ok(Some(session)) if session.is_admin => view! {
            <A href="/admin" attr:class="text-[#E76F00] hover:underline no-underline font-medium">
                "Admin"
            </A>
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

#[component]
pub fn TopNav() -> impl IntoView {
    view! {
        <header class="sticky top-0 z-50 bg-white border-b border-[#E5E5E5]">
            <div class="max-w-[1200px] mx-auto px-4 md:px-6 h-14 flex items-center justify-between">
                <A href="/" attr:class="text-[16px] font-semibold tracking-tight text-[#1A1A1A] no-underline">
                    "OnchainAI"
                </A>
                <nav class="flex items-center gap-3 md:gap-5 text-[14px]">
                    <A
                        href="/about#submit"
                        attr:class="inline-flex items-center justify-center h-9 px-4 rounded-lg bg-[#E76F00] text-white text-[14px] font-medium no-underline hover:opacity-90"
                    >
                        "Submit"
                    </A>
                    <a
                        href=GITHUB_REPO
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-[#6B6B6B] hover:text-[#1A1A1A] no-underline"
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