//! Sticky top navigation — logo left, Submit + GitHub + auth on the right.

use crate::auth::session::SessionUser;
use crate::components::login_modal::LoginModal;
use crate::models::tool::monogram_from_name;
use crate::server::functions::get_current_user;
use leptos::prelude::*;

const GITHUB_REPO: &str = "https://github.com/hoyeon4315-cpu/onchainai";

fn profile_monogram(nickname: Option<&str>) -> String {
    nickname
        .filter(|n| !n.is_empty())
        .map(monogram_from_name)
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| "?".into())
}

#[component]
fn ProfileMenu(session: SessionUser) -> impl IntoView {
    let menu_open = RwSignal::new(false);
    let nickname = session.nickname.clone().unwrap_or_else(|| "User".into());
    let monogram = profile_monogram(session.nickname.as_deref());
    let avatar_url = session.avatar_url.clone();
    let is_admin = session.is_admin;

    view! {
        <div class="site-profile-menu" data-testid="profile-menu">
            <button
                type="button"
                class="site-profile-btn"
                data-testid="profile-menu-btn"
                aria-label=format!("Account menu for {nickname}")
                aria-haspopup="menu"
                aria-expanded=move || if menu_open.get() { "true" } else { "false" }
                on:click=move |ev| {
                    ev.stop_propagation();
                    menu_open.update(|open| *open = !*open);
                }
            >
                {match avatar_url {
                    Some(url) if !url.is_empty() => view! {
                        <img
                            class="site-profile-avatar"
                            src=url
                            alt=""
                            width="32"
                            height="32"
                        />
                    }
                    .into_any(),
                    _ => view! {
                        <span class="site-profile-monogram" aria-hidden="true">
                            {monogram}
                        </span>
                    }
                    .into_any(),
                }}
            </button>

            <Show when=move || menu_open.get()>
                <div
                    class="site-profile-backdrop"
                    aria-hidden="true"
                    on:click=move |_| menu_open.set(false)
                ></div>
                <div
                    class="site-profile-dropdown"
                    role="menu"
                    data-testid="profile-menu-dropdown"
                    on:click=|ev| ev.stop_propagation()
                >
                    <a
                        href="/dashboard"
                        role="menuitem"
                        class="site-profile-dropdown-item"
                        data-testid="profile-menu-dashboard"
                    >
                        "Dashboard"
                    </a>
                    <a
                        href="/toolkit"
                        role="menuitem"
                        class="site-profile-dropdown-item"
                        data-testid="profile-menu-toolkit"
                    >
                        "My Toolkit"
                    </a>
                    <Show when=move || is_admin>
                        <a
                            href="/admin"
                            role="menuitem"
                            class="site-profile-dropdown-item site-profile-dropdown-item-admin"
                            data-testid="profile-menu-admin"
                        >
                            "Admin"
                        </a>
                    </Show>
                    <form action="/auth/logout" method="post" class="site-profile-dropdown-signout">
                        <button
                            type="submit"
                            role="menuitem"
                            class="site-profile-dropdown-item site-profile-dropdown-item-signout"
                            data-testid="profile-menu-sign-out"
                        >
                            "Sign out"
                        </button>
                    </form>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn AuthNav(
    user_res: Result<Option<SessionUser>, leptos::server_fn::ServerFnError>,
    show_login: RwSignal<bool>,
) -> impl IntoView {
    match user_res {
        Ok(Some(session)) => view! {
            <div class="site-top-nav-auth" data-testid="auth-signed-in">
                <ProfileMenu session=session/>
            </div>
        }
        .into_any(),
        Ok(None) | Err(_) => view! {
            <div class="site-top-nav-auth" data-testid="auth-sign-in">
                <button
                    type="button"
                    class="site-top-nav-btn site-top-nav-btn-outline"
                    data-testid="top-nav-sign-in"
                    on:click=move |_| show_login.set(true)
                >
                    "Sign in"
                </button>
            </div>
        }
        .into_any(),
    }
}

/// Site-wide sticky header — logo left, primary actions + auth on the right.
#[component]
pub fn TopNav() -> impl IntoView {
    let show_login = RwSignal::new(false);
    // Blocking SSR keeps auth markup in the initial HTML so hydration matches WASM.
    // Login flows land via full-page redirects, so one auth fetch per shell load is enough.
    let user = ArcOnceResource::new_blocking(async move { get_current_user().await });
    let user_ready = user.ready();

    view! {
        <LoginModal show=show_login/>
        <header class="site-top-nav">
            <div class="site-top-nav-inner">
                <a href="/" class="site-top-nav-logo">
                    "OnchainAI"
                </a>
                <nav class="site-top-nav-actions" aria-label="Site actions">
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
                    <Suspense fallback=move || {
                        view! {
                            <AuthNav user_res=Ok(None) show_login=show_login/>
                        }
                    }>
                        {Suspend::new(async move {
                            user_ready.await;
                            let user_res = user.read_untracked().as_ref().cloned().unwrap_or(Ok(None));
                            view! {
                                <AuthNav user_res=user_res show_login=show_login/>
                            }
                        })}
                    </Suspense>
                </nav>
            </div>
        </header>
    }
}
