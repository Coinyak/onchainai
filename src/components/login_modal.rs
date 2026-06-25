//! Sign-in modal — GitHub OAuth, email magic link, SIWX (pure Leptos + gloo-net).

use leptos::prelude::*;
use leptos::task::spawn_local;

async fn post_json(path: &str, body: serde_json::Value) -> Result<(), String> {
    gloo_net::http::Request::post(path)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| if r.ok() { Ok(()) } else { Err("request failed".into()) })
}

#[component]
pub fn LoginModal(show: RwSignal<bool>) -> impl IntoView {
    let email = RwSignal::new(String::new());
    let email_msg = RwSignal::new(None::<String>);
    let siwx_error = RwSignal::new(None::<String>);
    let email_busy = RwSignal::new(false);

    view! {
        {move || {
            if !show.get() {
                return None;
            }
            Some(view! {
                <div
                    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40"
                    on:click=move |_| show.set(false)
                >
                    <div
                        class="w-full max-w-sm rounded-xl bg-white border border-[#E5E5E5] shadow-lg p-6"
                        role="dialog"
                        aria-labelledby="login-title"
                        on:click=|ev| ev.stop_propagation()
                    >
                        <h2 id="login-title" class="text-[18px] font-semibold mb-2">
                            "Sign in"
                        </h2>
                        <p class="text-[14px] text-[#6B6B6B] mb-6">
                            "Sign in to comment, bookmark tools, and access admin features."
                        </p>
                        <a
                            href="/auth/github"
                            class="flex items-center justify-center w-full px-4 py-2.5 rounded-lg bg-[#1A1A1A] text-white text-[14px] font-medium hover:opacity-90 no-underline"
                        >
                            "Continue with GitHub"
                        </a>
                        <form
                            class="mt-3 flex gap-2"
                            on:submit=move |ev| {
                                ev.prevent_default();
                                let addr = email.get_untracked();
                                if addr.trim().is_empty() {
                                    return;
                                }
                                email_busy.set(true);
                                email_msg.set(Some("Sending magic link...".into()));
                                spawn_local(async move {
                                    match post_json("/auth/email", serde_json::json!({ "email": addr })).await {
                                        Ok(()) => email_msg.set(Some("Check your email for the sign-in link.".into())),
                                        Err(_) => email_msg.set(Some("Could not send magic link. Try again.".into())),
                                    }
                                    email_busy.set(false);
                                });
                            }
                        >
                            <input
                                type="email"
                                autocomplete="email"
                                placeholder="you@example.com"
                                class="flex-1 rounded-lg border border-[#E5E5E5] px-3 py-2.5 text-[14px]"
                                prop:value=move || email.get()
                                on:input=move |ev| email.set(event_target_value(&ev))
                            />
                            <button
                                type="submit"
                                class="px-3 py-2.5 rounded-lg border border-[#E5E5E5] text-[14px] font-medium hover:bg-[#FAFAFA]"
                                disabled=move || email_busy.get()
                            >
                                "Email"
                            </button>
                        </form>
                        {move || email_msg.get().map(|m| view! {
                            <p class="mt-2 text-[13px] text-[#6B6B6B]">{m}</p>
                        })}
                        <button
                            type="button"
                            class="flex items-center justify-center w-full mt-3 px-4 py-2.5 rounded-lg border border-[#E5E5E5] text-[14px] font-medium hover:bg-[#FAFAFA]"
                            on:click=move |_| {
                                siwx_error.set(None);
                                if !is_browser() {
                                    siwx_error.set(Some("Wallet sign-in requires a browser.".into()));
                                    return;
                                }
                                spawn_local(async move {
                                    siwx_error.set(Some(
                                        "Open /login in your browser with MetaMask to use wallet sign-in.".into(),
                                    ));
                                });
                            }
                        >
                            "Connect Wallet (SIWX)"
                        </button>
                        {move || siwx_error.get().map(|e| view! {
                            <p class="mt-2 text-[13px] text-[#C0392B]">{e}</p>
                        })}
                        <button
                            type="button"
                            class="mt-3 w-full text-[14px] text-[#6B6B6B] hover:text-[#1A1A1A]"
                            on:click=move |_| show.set(false)
                        >
                            "Cancel"
                        </button>
                    </div>
                </div>
            })
        }}
    }
}