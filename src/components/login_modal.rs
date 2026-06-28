//! Sign-in modal — delegates to shared LoginForm.

use crate::components::login_form::LoginForm;
use leptos::prelude::*;

#[component]
pub fn LoginModal(show: RwSignal<bool>) -> impl IntoView {
    let backdrop_ref = NodeRef::<leptos::html::Div>::new();

    Effect::new(move |_| {
        if show.get() {
            if let Some(el) = backdrop_ref.get() {
                let _ = el.focus();
            }
        }
    });

    view! {
        {move || {
            if !show.get() {
                return None;
            }
            Some(view! {
                <div
                    node_ref=backdrop_ref
                    class="modal-overlay"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="login-title"
                    tabindex="-1"
                    on:keydown=move |ev| {
                        if ev.key() == "Escape" {
                            ev.stop_propagation();
                            show.set(false);
                        }
                    }
                    on:click=move |_| show.set(false)
                >
                    <div
                        class="w-full max-w-sm rounded-xl bg-white border border-[#E5E5E5] shadow-lg p-6"
                        on:click=|ev| ev.stop_propagation()
                    >
                        <LoginForm
                            compact=true
                            heading_id="login-title"
                            on_cancel=Callback::new(move |_| show.set(false))
                        />
                    </div>
                </div>
            })
        }}
    }
}
