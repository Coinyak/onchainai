//! Shared sign-in form — GitHub OAuth, email magic link, SIWX wallet.

use crate::auth::siwx_client::siwx_connect_evm;
use leptos::prelude::*;
use leptos::task::spawn_local;

async fn post_json(path: &str, body: serde_json::Value) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        gloo_net::http::Request::post(path)
            .header("Content-Type", "application/json")
            .credentials(web_sys::RequestCredentials::Include)
            .body(body.to_string())
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| {
                if r.ok() {
                    Ok(())
                } else {
                    Err(format!("Request failed ({})", r.status()))
                }
            })
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = (path, body);
        Err("Email sign-in requires browser JavaScript.".to_string())
    }
}

#[component]
pub fn LoginForm(
    #[prop(optional)] on_cancel: Option<Callback<()>>,
    #[prop(optional)] compact: bool,
) -> impl IntoView {
    let email = RwSignal::new(String::new());
    let email_msg = RwSignal::new(None::<String>);
    let siwx_error = RwSignal::new(None::<String>);
    let email_busy = RwSignal::new(false);
    let siwx_busy = RwSignal::new(false);

    let heading_class = if compact {
        "text-[18px] font-semibold mb-2"
    } else {
        "text-[24px] font-semibold mb-2"
    };
    let desc_class = if compact {
        "text-[14px] text-[#6B6B6B] mb-6"
    } else {
        "text-[14px] text-[#6B6B6B] mb-8"
    };

    view! {
        <h1 class=heading_class>"Sign in"</h1>
        <p class=desc_class>
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
            class="flex items-center justify-center w-full mt-3 px-4 py-2.5 rounded-lg border border-[#E5E5E5] text-[14px] font-medium hover:bg-[#FAFAFA] disabled:opacity-60"
            disabled=move || siwx_busy.get()
            on:click=move |_| {
                siwx_error.set(None);
                if !is_browser() {
                    siwx_error.set(Some("Wallet sign-in requires a browser.".into()));
                    return;
                }
                siwx_busy.set(true);
                spawn_local(async move {
                    match siwx_connect_evm().await {
                        Ok(redirect) => {
                            #[cfg(feature = "hydrate")]
                            if let Some(win) = web_sys::window() {
                                let _ = win.location().set_href(&redirect);
                            }
                            #[cfg(not(feature = "hydrate"))]
                            let _ = redirect;
                        }
                        Err(e) => siwx_error.set(Some(e)),
                    }
                    siwx_busy.set(false);
                });
            }
        >
            {move || if siwx_busy.get() { "Connecting wallet..." } else { "Connect Wallet (SIWX)" }}
        </button>
        {move || siwx_error.get().map(|e| view! {
            <p class="mt-2 text-[13px] text-[#C0392B]">{e}</p>
        })}
        {move || on_cancel.map(|cb| view! {
            <button
                type="button"
                class="mt-3 w-full text-[14px] text-[#6B6B6B] hover:text-[#1A1A1A]"
                on:click=move |_| cb.run(())
            >
                "Cancel"
            </button>
        })}
    }
}