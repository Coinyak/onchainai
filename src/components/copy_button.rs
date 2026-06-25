//! Copy-to-clipboard button — `data-copy` attribute + shell listener for SSR/hydration.

use leptos::prelude::*;

#[component]
pub fn CopyButton(
    text: String,
    #[prop(optional)] label: Option<&'static str>,
) -> impl IntoView {
    view! {
        <button type="button" class="copy-btn" data-copy=text>
            {label.unwrap_or("Copy")}
        </button>
    }
}