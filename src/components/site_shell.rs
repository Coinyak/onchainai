//! Site-wide content shell for pages without filter sidebar.

use leptos::prelude::*;

#[component]
pub fn SiteShell(children: Children) -> impl IntoView {
    view! {
        <div class="site-content-shell">
            <main class="site-main site-main-full">
                {children()}
            </main>
        </div>
    }
}
