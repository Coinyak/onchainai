//! Empty state with filter summary, clear filters, and submit CTA — UI_UX_DESIGN §6.

use leptos::prelude::*;

#[component]
pub fn EmptyState(
    #[prop(default = "No tools match your filters.")] message: &'static str,
    #[prop(default = Vec::new())] filter_lines: Vec<String>,
    #[prop(default = String::new())] clear_href: String,
) -> impl IntoView {
    let has_filters = !filter_lines.is_empty();
    let show_clear = has_filters && !clear_href.is_empty();

    view! {
        <div class="empty-state-panel">
            <p class="empty-state-message">{message}</p>
            {if has_filters {
                view! {
                    <div class="empty-state-filters" aria-label="Active filters">
                        <p class="empty-state-filters-heading">"Current filters"</p>
                        <ul class="empty-state-filter-list">
                            {filter_lines.into_iter().map(|line| view! {
                                <li>{line}</li>
                            }).collect_view()}
                        </ul>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}
            <p class="empty-state-hint">
                "Try a different keyword, suggest a tool for operator review, or clear your filters."
            </p>
            <div class="empty-state-actions">
                {if show_clear {
                    view! {
                        <a href=clear_href.clone() class="empty-state-clear-btn">"Clear filters"</a>
                    }.into_any()
                } else {
                    ().into_any()
                }}
                <a
                    href="/submit"
                    class="empty-state-submit-btn"
                >
                    "Suggest a tool →"
                </a>
            </div>
        </div>
    }
}
