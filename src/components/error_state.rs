//! Error state with Retry — UI_UX_DESIGN §6.

use leptos::prelude::*;

#[component]
pub fn ErrorState(message: String, on_retry: impl Fn() + 'static + Copy) -> impl IntoView {
    view! {
        <div class="error-state-panel" role="alert">
            <p class="error-state-message">{message}</p>
            <button
                type="button"
                class="error-retry-btn"
                on:click=move |_| on_retry()
            >
                "Retry"
            </button>
        </div>
    }
}
