//! Sticky top navigation — logo left, Submit + GitHub + auth on the right.

use crate::auth::session::SessionUser;
use crate::components::login_form::WalletConnectButton;
use crate::server::functions::get_current_user;
use leptos::prelude::*;

const GITHUB_REPO: &str = "https://github.com/hoyeon4315-cpu/onchainai";

#[component]
fn AuthNav(
    user_res: Result<Option<SessionUser>, leptos::server_fn::ServerFnError>,
) -> impl IntoView {
    match user_res {
        Ok(Some(session)) if session.is_admin => view! {
            <div class="site-top-nav-auth" data-testid="auth-signed-in">
                <a href="/admin" class="site-top-nav-admin">
                    "Admin"
                </a>
                <span class="site-top-nav-nickname">
                    {session.nickname.clone().unwrap_or_else(|| "admin".into())}
                </span>
                <form action="/auth/logout" method="post" class="inline">
                    <button type="submit" class="site-top-nav-signout">
                        "Sign out"
                    </button>
                </form>
            </div>
        }
        .into_any(),
        Ok(Some(_session)) => view! {
            <div class="site-top-nav-auth" data-testid="auth-signed-in">
                <form action="/auth/logout" method="post" class="inline">
                    <button type="submit" class="site-top-nav-signout">
                        "Sign out"
                    </button>
                </form>
            </div>
        }
        .into_any(),
        Ok(None) | Err(_) => view! {
            <div class="site-top-nav-auth" data-testid="auth-sign-in">
                <a
                    href="/auth/github"
                    class="site-top-nav-btn site-top-nav-btn-outline"
                    data-testid="github-sign-in"
                >
                    "GitHub"
                </a>
                <WalletConnectButton
                    label="Wallet"
                    class="site-top-nav-btn site-top-nav-btn-outline"
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
        <header class="site-top-nav">
            <div class="site-top-nav-inner">
                <a href="/" class="site-top-nav-logo">
                    "OnchainAI"
                </a>
                <nav class="site-top-nav-actions" aria-label="Site actions">
                    <a href="/dashboard" class="site-top-nav-repo site-top-nav-link-dashboard">
                        "Dashboard"
                    </a>
                    <a href="/toolkit" class="site-top-nav-repo site-top-nav-link-toolkit">
                        "Toolkit"
                    </a>
                    <a href="/submit" class="site-top-nav-btn site-top-nav-btn-primary">
                        "Submit"
                    </a>
                    <a
                        href=GITHUB_REPO
                        target="_blank"
                        rel="noopener noreferrer"
                        class="site-top-nav-repo"
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
