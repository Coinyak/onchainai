//! Site-wide left shell — brand sidebar + main content (non-filter pages).

use crate::components::top_nav::SidebarBrand;
use leptos::prelude::*;

#[component]
pub fn SiteShell(children: Children) -> impl IntoView {
    view! {
        <div class="site-layout">
            <aside class="tools-sidebar site-sidebar-chrome">
                <SidebarBrand/>
            </aside>
            <main class="site-main">
                {children()}
            </main>
        </div>
    }
}
