//! Chain logo with a text fallback for broken or unsupported assets.

use crate::chains::{chain_fallback_label, chain_logo_path};
use leptos::prelude::*;

#[cfg(feature = "hydrate")]
fn sync_chain_img_error_handling(img: &web_sys::HtmlImageElement, show: RwSignal<bool>) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    let cb = Closure::wrap(Box::new(move || {
        show.set(false);
    }) as Box<dyn FnMut()>);
    img.set_onerror(Some(cb.as_ref().unchecked_ref()));
    cb.forget();
    if img.complete() && img.natural_width() == 0 {
        show.set(false);
    }
}

#[component]
pub fn ChainLogo(
    id: String,
    label: String,
    #[prop(default = "chain-logo")] class: &'static str,
    #[prop(default = 20)] size: u32,
) -> impl IntoView {
    let src = chain_logo_path(&id);
    let fallback = chain_fallback_label(&label);
    let show_image = RwSignal::new(true);
    let img_ref = NodeRef::<leptos::html::Img>::new();

    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        if let Some(img) = img_ref.get() {
            let el: web_sys::HtmlImageElement = img.clone().into();
            sync_chain_img_error_handling(&el, show_image);
        }
    });

    view! {
        {move || {
            if show_image.get() {
                view! {
                    <img
                        node_ref=img_ref
                        class=class
                        src=src.clone()
                        alt=label.clone()
                        title=label.clone()
                        width=size
                        height=size
                        loading="lazy"
                        decoding="async"
                        on:error=move |_| show_image.set(false)
                        on:load=move |_| {
                            #[cfg(feature = "hydrate")]
                            if let Some(img) = img_ref.get() {
                                let el: web_sys::HtmlImageElement = img.clone().into();
                                if el.natural_width() == 0 {
                                    show_image.set(false);
                                }
                            }
                        }
                    />
                }.into_any()
            } else {
                view! {
                    <span
                        class=format!("{class} chain-logo-fallback")
                        title=label.clone()
                        aria-label=label.clone()
                    >
                        {fallback.clone()}
                    </span>
                }.into_any()
            }
        }}
    }
}
