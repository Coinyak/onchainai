//! Sticky top navigation with auth-aware links.

use crate::components::login_modal::LoginModal;
use crate::server::functions::get_current_user;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn TopNav() -> impl IntoView {
    let user = Resource::new(|| (), |_| async move { get_current_user().await });
    let show_login = RwSignal::new(false);

    view! {
        <header class="sticky top-0 z-50 bg-white border-b border-[#E5E5E5]">
            <div class="max-w-[1200px] mx-auto px-4 md:px-6 h-14 flex items-center justify-between">
                <A href="/" attr:class="text-[16px] font-semibold tracking-tight text-[#1A1A1A] no-underline">
                    "OnchainAI"
                </A>
                <nav class="flex items-center gap-4 md:gap-6 text-[14px]">
                    <A href="/tools" attr:class="text-[#6B6B6B] hover:text-[#1A1A1A] no-underline">
                        "Tools"
                    </A>
                    <A href="/about" attr:class="text-[#6B6B6B] hover:text-[#1A1A1A] no-underline">
                        "About"
                    </A>
                    <Suspense fallback=|| ()>
                        {move || user.get().map(|res| match res {
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
                            }.into_any(),
                            Ok(Some(_session)) => view! {
                                <form action="/auth/logout" method="post" class="inline">
                                    <button
                                        type="submit"
                                        class="text-[#6B6B6B] hover:text-[#1A1A1A] bg-transparent border-0 cursor-pointer text-[14px] p-0"
                                    >
                                        "Sign out"
                                    </button>
                                </form>
                            }.into_any(),
                            Ok(None) => view! {
                                <button
                                    type="button"
                                    class="text-[#1A1A1A] font-medium bg-transparent border-0 cursor-pointer text-[14px] p-0"
                                    on:click=move |_| show_login.set(true)
                                >
                                    "Sign in"
                                </button>
                            }.into_any(),
                            Err(_) => view! {
                                <button
                                    type="button"
                                    class="text-[#1A1A1A] font-medium bg-transparent border-0 cursor-pointer text-[14px] p-0"
                                    on:click=move |_| show_login.set(true)
                                >
                                    "Sign in"
                                </button>
                            }.into_any(),
                        })}
                    </Suspense>
                </nav>
            </div>
        </header>
        <LoginModal show=show_login/>
    }
}