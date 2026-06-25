//! Copy-to-clipboard button — Leptos on:click (hydrate / browser).

use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn CopyButton(
    text: String,
    #[prop(optional)] label: Option<&'static str>,
) -> impl IntoView {
    let copied = RwSignal::new(false);
    let label = label.unwrap_or("Copy");

    view! {
        <button
            type="button"
            class="copy-btn"
            on:click=move |_| {
                let t = text.clone();
                spawn_local(async move {
                    copy_text_to_clipboard(&t, copied).await;
                });
            }
        >
            {move || if copied.get() { "Copied" } else { label }}
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