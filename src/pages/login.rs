//! Login page — shared LoginForm (GitHub, email, SIWX).

use crate::components::login_form::LoginForm;
use crate::components::site_shell::SiteShell;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

fn auth_error_message(code: &str) -> Option<&'static str> {
    match code {
        "github_denied" => Some("GitHub sign-in was cancelled."),
        "github_missing_code" | "github_missing_state" => {
            Some("GitHub sign-in could not be completed. Please try again.")
        }
        "github_state_mismatch" => {
            Some("GitHub sign-in session expired. Clear cookies and try again.")
        }
        "github_token_exchange" | "github_user_fetch" => {
            Some("GitHub sign-in failed. Check OAuth app settings and try again.")
        }
        "github_profile" => Some("Could not create your profile. Try again in a moment."),
        _ => None,
    }
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let query = use_query_map();
    let auth_error = Memo::new(move |_| {
        query
            .get()
            .get("auth")
            .and_then(|v| auth_error_message(v.as_str()))
    });

    view! {
        <SiteShell>
            <div class="max-w-[420px] px-4 py-12">
                {move || auth_error.get().map(|msg| view! {
                    <p
                        class="mb-4 rounded-lg border border-[#F5C6C6] bg-[#FDF2F2] px-3 py-2 text-[14px] text-[#C0392B]"
                        role="alert"
                    >
                        {msg}
                    </p>
                })}
                <LoginForm/>
            </div>
        </SiteShell>
    }
}
