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
        "github_token_exchange" => Some(
            "GitHub sign-in failed. If GitHub showed \"redirect_uri is not associated with this application\", \
             register this app's callback URL on your OAuth app (local dev: http://localhost:3000/auth/callback). \
             Run ./scripts/local-auth-check.sh to compare .env and the running server.",
        ),
        "github_user_fetch" => Some(
            "GitHub authorized sign-in, but we could not load your profile. Try again in a moment.",
        ),
        "github_profile_exists" => {
            Some("This GitHub account is already linked. Try signing in again.")
        }
        "github_profile_setup" => Some(
            "Account setup failed while creating your profile. Try again in a moment, or use email sign-in.",
        ),
        "github_profile" => Some("Could not save your profile. Try again in a moment."),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::auth_error_message;

    #[test]
    fn profile_setup_errors_are_user_friendly() {
        assert!(auth_error_message("github_profile_setup")
            .unwrap()
            .contains("profile"));
        assert!(auth_error_message("github_profile_exists")
            .unwrap()
            .contains("linked"));
    }

    #[test]
    fn token_exchange_error_mentions_redirect_uri() {
        let msg = auth_error_message("github_token_exchange").unwrap();
        assert!(msg.contains("redirect_uri"));
        assert!(msg.contains("local-auth-check"));
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
