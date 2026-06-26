//! Loading skeletons matching tool card layout.

use leptos::prelude::*;

#[component]
pub fn ToolCardSkeleton() -> impl IntoView {
    view! {
        <article class="tool-card skeleton-card" aria-hidden="true">
            <div class="tool-card-inner">
                <div class="tool-logo skeleton-block"></div>
                <div class="tool-card-body">
                    <div class="skeleton-line skeleton-title"></div>
                    <div class="skeleton-line skeleton-desc"></div>
                    <div class="skeleton-line skeleton-meta"></div>
                </div>
            </div>
        </article>
    }
}

#[component]
pub fn ToolListSkeleton(#[prop(default = 6)] count: usize) -> impl IntoView {
    view! {
        <div class="tool-list">
            {(0..count).map(|_| view! { <ToolCardSkeleton/> }).collect_view()}
        </div>
    }
}
