//! Tool logo — monogram always present; image overlays and removes itself on error.

use crate::models::tool::{display_monogram, tool_logo_img_url};
use crate::models::Tool;
use leptos::prelude::*;

#[cfg(feature = "hydrate")]
fn attach_native_logo_error(img: &web_sys::HtmlImageElement, show: RwSignal<bool>) {
    use wasm_bindgen::closure::Closure;
    let img = img.clone();
    let cb = Closure::wrap(Box::new(move || {
        show.set(false);
        let _ = img.remove();
    }) as Box<dyn FnMut()>);
    img.set_onerror(Some(cb.as_ref().unchecked_ref()));
    cb.forget();
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
    let has_logo_url = logo_img.is_some();
    let img_ref = NodeRef::<leptos::html::Img>::new();

    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        if let Some(img) = img_ref.get() {
            let el: web_sys::HtmlImageElement = img.clone().into();
            attach_native_logo_error(&el, show_logo_img);
        }
    });

    view! {
        <div class=class aria-hidden="true" data-has-logo-url=has_logo_url>
            <span class="tool-logo-monogram">{mono}</span>
            {logo_img.map(|url| {
                view! {
                    <img
                        node_ref=img_ref
                        class=img_class
                        src=url
                        alt=""
                        loading="lazy"
                        referrerpolicy="no-referrer"
                        on:error=move |_| show_logo_img.set(false)
                    />
                }
            })}
        </div>
    }
}
