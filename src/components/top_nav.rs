//! Sticky top navigation — logo left, Submit + GitHub + auth on the right.

use crate::auth::session::SessionUser;
use crate::components::login_form::WalletConnectButton;
use crate::server::functions::get_current_user;
use leptos::prelude::*;

const GITHUB_REPO: &str = "https://github.com/hoyeon4315-cpu/onchainai";

const AUTH_BTN_CLASS: &str = "inline-flex items-center justify-center h-8 px-3 rounded-lg border border-[#E5E5E5] text-[#1A1A1A] text-[13px] font-medium no-underline hover:bg-[#FAFAFA]";

#[component]
fn AuthNav(
    user_res: Result<Option<SessionUser>, leptos::server_fn::ServerFnError>,
    #[prop(default = true)] inline: bool,
) -> impl IntoView {
    let layout_class = if inline {
        "flex items-center gap-2"
    } else {
        "flex flex-col gap-2 w-full"
    };

    match user_res {
        Ok(Some(session)) if session.is_admin => view! {
            <div class=layout_class>
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
            </div>
        }
        .into_any(),
        Ok(Some(_session)) => view! {
            <div class=layout_class>
                <form action="/auth/logout" method="post" class="inline">
                    <button
                        type="submit"
                        class="text-[#6B6B6B] hover:text-[#1A1A1A] bg-transparent border-0 cursor-pointer text-[14px] p-0"
                    >
                        "Sign out"
                    </button>
                </form>
            </div>
        }
        .into_any(),
        Ok(None) | Err(_) => view! {
            <div class=layout_class data-testid="auth-sign-in">
                <a href="/auth/github" class=AUTH_BTN_CLASS>
                    <span class="sm:hidden">"GitHub"</span>
                    <span class="hidden sm:inline">"Continue with GitHub"</span>
                </a>
                <WalletConnectButton
                    label="Connect Wallet"
                    class=AUTH_BTN_CLASS
                />
            </div>
        }
        .into_any(),
    }
}

/// Site-wide sticky header — logo left, primary actions + auth on the right.
#[component]
pub fn TopNav() -> impl IntoView {
    view! {
        <header class="site-top-nav bg-white border-b border-[#E5E5E5]">
            <div class="site-top-nav-inner max-w-[1200px] mx-auto px-4 md:px-6 h-12 md:h-14 flex items-center justify-between">
                <a href="/" class="text-[16px] font-semibold tracking-tight text-[#1A1A1A] no-underline">
                    "OnchainAI"
                </a>
                <nav class="flex items-center gap-2 md:gap-5 text-[14px]">
                    <a
                        href="/dashboard"
                        class="hidden sm:inline text-[#6B6B6B] hover:text-[#1A1A1A] no-underline text-[13px]"
                    >
                        "Dashboard"
                    </a>
                    <a
                        href="/toolkit"
                        class="hidden md:inline text-[#6B6B6B] hover:text-[#1A1A1A] no-underline text-[13px]"
                    >
                        "Toolkit"
                    </a>
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
                        class="hidden sm:inline text-[#6B6B6B] hover:text-[#1A1A1A] no-underline text-[13px]"
                    >
                        "GitHub"
                    </a>
                    <Await future=async move { get_current_user().await } let:user_res blocking=true>
                        <AuthNav user_res=user_res.clone() inline=true/>
                    </Await>
                </nav>
            </div>
        </header>
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn auth_btn_class_matches_nav_compact_style() {
        assert!(super::AUTH_BTN_CLASS.contains("h-8"));
        assert!(super::AUTH_BTN_CLASS.contains("text-[13px]"));
    }
}
