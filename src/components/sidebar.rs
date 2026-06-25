//! Filter sidebar for tools list (simplified MVP — function categories).

use crate::models::Category;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Sidebar(categories: Vec<(Category, i64)>, active_function: Option<String>) -> impl IntoView {
    let active = active_function.clone();
    let all_class = if active_function.is_none() {
        "sidebar-link active"
    } else {
        "sidebar-link"
    };
    view! {
        <aside class="tools-sidebar">
            <h3 class="sidebar-title">"Function"</h3>
            <ul class="sidebar-list">
                <li>
                    <A href="/tools" attr:class=all_class>
                        "All"
                    </A>
                </li>
                {categories
                    .into_iter()
                    .map(|(cat, count)| {
                        let href = format!("/tools?function={}", cat.id);
                        let id = cat.id.clone();
                        let is_active = active.as_deref() == Some(id.as_str());
                        let class = if is_active {
                            "sidebar-link active"
                        } else {
                            "sidebar-link"
                        };
                        view! {
                            <li>
                                <A href=href attr:class=class>
                                    {cat.label}
                                    <span class="sidebar-count">{count}</span>
                                </A>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </aside>
    }
}