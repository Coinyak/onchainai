//! Category page — tools filtered by category id via shared ToolsBrowser.

use crate::components::tools_browser::{BrowserBase, ToolsBrowser};
use crate::components::top_nav::TopNav;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn CategoryPage() -> impl IntoView {
    let params = use_params_map();
    let cat_id = Memo::new(move |_| params.with(|p| p.get("id").unwrap_or_default()));

    view! {
        <TopNav/>
        {move || {
            let id = cat_id.get();
            view! {
                <ToolsBrowser base=BrowserBase::Category(id) show_toolbar_search=false/>
            }
        }}
    }
}
