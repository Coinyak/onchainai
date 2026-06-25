//! Category grid — 14 function categories with Lucide icons; click-to-filter.

use crate::components::icons::LucideIcon;
use crate::components::tools_browser::BrowserBase;
use crate::models::Category;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn CategoryGrid(
    categories: Vec<(Category, i64)>,
    #[prop(default = BrowserBase::Tools)] base: BrowserBase,
) -> impl IntoView {
    let root = base.path();
    view! {
        <section class="mb-10">
            <h2 class="text-[20px] font-semibold mb-4">"Browse by function"</h2>
            <div class="category-grid">
                {categories
                    .into_iter()
                    .map(|(cat, count)| {
                        let href = format!("{root}?function={}", cat.id);
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