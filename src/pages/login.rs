//! Login page — shared LoginForm (GitHub, email, SIWX).

use crate::components::login_form::LoginForm;
use crate::components::site_shell::SiteShell;
use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <SiteShell>
            <div class="max-w-[420px] px-4 py-12">
                <LoginForm/>
            </div>
        </SiteShell>
    }
}
