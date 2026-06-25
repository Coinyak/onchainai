//! Copy-to-clipboard button — Leptos on:click (no shell script).

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
                    if !is_browser() {
                        return;
                    }
                    let win = window();
                    let clipboard = win.navigator().clipboard();
                    if clipboard.write_text(&t).await.is_ok() {
                        copied.set(true);
                        gloo_timers::future::TimeoutFuture::new(2000).await;
                        copied.set(false);
                    }
                });
            }
        >
            {move || if copied.get() { "Copied" } else { label }}
        </button>
    }
}