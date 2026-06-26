//! Admin pages — tool approval and site management.

pub mod categories;
pub mod comments;
pub mod crawler;
pub mod dashboard;
pub mod featured;
pub mod settings;
pub mod tools;
pub mod users;

pub use categories::AdminCategoriesPage;
pub use comments::AdminCommentsPage;
pub use crawler::AdminCrawlerPage;
pub use dashboard::AdminDashboardPage;
pub use featured::AdminFeaturedPage;
pub use settings::AdminSettingsPage;
pub use tools::AdminToolsPage;
pub use users::AdminUsersPage;

use crate::components::admin_shell::AdminShell;
use crate::server::functions::check_admin_access;
use leptos::prelude::*;
use std::sync::Arc;

/// Wrap admin page content with a server-side `is_admin` check (non-admins see 404).
pub fn admin_page_shell<F, V>(inner: F) -> impl IntoView
where
    F: Fn() -> V + Send + Sync + 'static,
    V: IntoView + 'static,
{
    let inner = Arc::new(inner);
    let gate = Resource::new(|| (), |_| async move { check_admin_access().await });

    view! {
        <Suspense fallback=|| view! {
            <p class="px-6 py-12 text-[#6B6B6B] text-[14px]">"Checking access..."</p>
        }>
            {move || {
                let inner = inner.clone();
                match gate.get() {
                    Some(Ok(_)) => view! {
                        <AdminShell>
                            {inner()}
                        </AdminShell>
                    }
                    .into_any(),
                    Some(Err(_)) => view! {
                        <div class="px-6 py-12 max-w-[720px] mx-auto text-center">
                            <h1 class="text-[28px] font-bold mb-4">"404"</h1>
                            <p class="text-[#6B6B6B]">"Page not found."</p>
                        </div>
                    }
                    .into_any(),
                    None => ().into_any(),
                }
            }}
        </Suspense>
    }
}
