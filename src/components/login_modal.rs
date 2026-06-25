//! Sign-in modal — delegates to shared LoginForm.

use crate::components::login_form::LoginForm;
use leptos::prelude::*;

#[component]
pub fn LoginModal(show: RwSignal<bool>) -> impl IntoView {
    view! {
        {move || {
            if !show.get() {
                return None;
            }
            Some(view! {
                <div
                    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40"
                    on:click=move |_| show.set(false)
                >
                    <div
                        class="w-full max-w-sm rounded-xl bg-white border border-[#E5E5E5] shadow-lg p-6"
                        role="dialog"
                        aria-labelledby="login-title"
                        on:click=|ev| ev.stop_propagation()
                    >
                        <LoginForm
                            compact=true
                            on_cancel=Callback::new(move |_| show.set(false))
                        />
                    </div>
                </div>
            })
        }}
    }
}