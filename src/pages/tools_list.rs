//! Tools list page — shared browser at `/tools`.

use crate::components::tools_browser::{BrowserBase, ToolsBrowser};
use crate::components::top_nav::TopNav;
use leptos::prelude::*;

#[component]
pub fn ToolsListPage() -> impl IntoView {
    view! {
        <TopNav/>
        <ToolsBrowser base=BrowserBase::Tools show_toolbar_search=true/>
    }
}