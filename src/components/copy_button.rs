//! Copy-to-clipboard button — icon-only clipboard SVG, "Copied" feedback.

use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn CopyButton(text: String, #[prop(optional)] _label: Option<&'static str>) -> impl IntoView {
    let copied = RwSignal::new(false);

    view! {
        <button
            type="button"
            class=move || if copied.get() { "copy-btn copied" } else { "copy-btn" }
            aria-label="Copy to clipboard"
            on:click=move |ev| {
                ev.stop_propagation();
                ev.prevent_default();
                let t = text.clone();
                spawn_local(async move {
                    copy_text_to_clipboard(&t, copied).await;
                });
            }
        >
            {move || if copied.get() {
                "Copied".into_any()
            } else {
                view! {
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        aria-hidden="true"
                    >
                        <rect width="14" height="14" x="8" y="8" rx="2" ry="2"/>
                        <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>
                    </svg>
                }.into_any()
            }}
        </button>
    }
}

async fn copy_text_to_clipboard(text: &str, copied: RwSignal<bool>) {
    if !is_browser() {
        return;
    }
    #[cfg(feature = "hydrate")]
    {
        if let Some(win) = web_sys::window() {
            let clipboard = win.navigator().clipboard();
            if clipboard.write_text(text).await.is_ok() {
                copied.set(true);
                gloo_timers::future::TimeoutFuture::new(2000).await;
                copied.set(false);
            }
        }
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = (text, copied);
    }
}
