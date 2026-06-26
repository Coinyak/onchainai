//! Tools list page — shared browser at `/tools`.

use crate::components::tools_browser::{BrowserBase, ToolsBrowser};
use leptos::prelude::*;

#[component]
pub fn ToolsListPage() -> impl IntoView {
    view! {
        <ToolsBrowser base=BrowserBase::Tools show_toolbar_search=true/>
    }
}
