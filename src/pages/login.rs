//! Login page — shared LoginForm (GitHub, email, SIWX).

use crate::components::login_form::LoginForm;
use crate::components::top_nav::TopNav;
use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <TopNav/>
        <div class="max-w-[420px] mx-auto px-4 py-12">
            <LoginForm/>
        </div>
    }
}
