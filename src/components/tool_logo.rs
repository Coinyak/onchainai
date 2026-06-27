//! Tool logo — monogram always present; image overlays and hides on error.

use crate::models::tool::{display_monogram, tool_logo_img_url};
use crate::models::Tool;
use leptos::prelude::*;

#[cfg(feature = "hydrate")]
fn sync_logo_img_error_handling(img: &web_sys::HtmlImageElement, show: RwSignal<bool>) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    let img_for_handler = img.clone();
    let cb = Closure::wrap(Box::new(move || {
        show.set(false);
        let _ = img_for_handler.remove();
    }) as Box<dyn FnMut()>);
    img.set_onerror(Some(cb.as_ref().unchecked_ref()));
    cb.forget();
    // Image may have failed before the Effect runs (cached 404, CSP block).
    if img.complete() && img.natural_width() == 0 {
        show.set(false);
        let _ = img.remove();
    }
}

#[component]
pub fn ToolLogo(
    tool: Tool,
    #[prop(default = "tool-logo")] class: &'static str,
    #[prop(default = "tool-logo-img")] img_class: &'static str,
) -> impl IntoView {
    let mono = display_monogram(&tool);
    let logo_img = tool_logo_img_url(&tool);
    let show_logo_img = RwSignal::new(logo_img.is_some());
    let img_ref = NodeRef::<leptos::html::Img>::new();

    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        if let Some(img) = img_ref.get() {
            let el: web_sys::HtmlImageElement = img.clone().into();
            sync_logo_img_error_handling(&el, show_logo_img);
        }
    });

    view! {
        <div class=class aria-hidden="true">
            <span class="tool-logo-monogram">{mono}</span>
            {move || {
                if show_logo_img.get() {
                    if let Some(url) = logo_img.clone() {
                        view! {
                            <img
                                node_ref=img_ref
                                class=img_class
                                src=url
                                alt=""
                                loading="lazy"
                                referrerpolicy="no-referrer"
                            />
                        }
                        .into_any()
                    } else {
                        ().into_any()
                    }
                } else {
                    ().into_any()
                }
            }}
        </div>
    }
}
