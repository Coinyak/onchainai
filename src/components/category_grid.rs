//! Category grid — 14 function categories with Lucide icons and counts.

use crate::components::icons::LucideIcon;
use crate::models::Category;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn CategoryGrid(categories: Vec<(Category, i64)>) -> impl IntoView {
    view! {
        <section class="mb-10">
            <h2 class="text-[20px] font-semibold mb-4">"Browse by function"</h2>
            <div class="category-grid">
                {categories
                    .into_iter()
                    .map(|(cat, count)| {
                        let href = format!("/tools?function={}", cat.id);
                        view! {
                            <A
                                href=href
                                attr:class="category-card no-underline text-[#1A1A1A] hover:border-[#D1D1D1]"
                            >
                                <span class="category-icon">
                                    <LucideIcon name=cat.icon/>
                                </span>
                                <span class="category-label">{cat.label}</span>
                                <span class="category-count">{count}</span>
                            </A>
                        }
                    })
                    .collect_view()}
            </div>
        </section>
    }
}