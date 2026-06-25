//! Mobile bottom sheet — 60% slide-up with drag-to-expand.

use crate::components::tool_detail_content::ToolDetailContent;
use crate::models::Tool;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn BottomSheet(
    tool: Tool,
    close_href: String,
    full_page_href: String,
) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let drag_start_y = RwSignal::new(None::<f64>);
    let drag_delta = RwSignal::new(0.0_f64);

    let sheet_class = move || {
        if expanded.get() {
            "bottom-sheet bottom-sheet-full"
        } else {
            "bottom-sheet"
        }
    };

    view! {
        <A href=close_href.clone() attr:class="bottom-sheet-backdrop" attr:aria-label="Close preview">
            <span class="sr-only">"Close"</span>
        </A>
        <div
            class=sheet_class
            role="dialog"
            aria-label="Tool preview"
            style=move || {
                let d = drag_delta.get();
                if d > 0.0 && !expanded.get() {
                    format!("transform: translateY({d}px)")
                } else {
                    String::new()
                }
            }
        >
            <div
                class="bottom-sheet-handle"
                aria-hidden="true"
                on:pointerdown=move |ev| {
                    drag_start_y.set(Some(ev.client_y() as f64));
                    drag_delta.set(0.0);
                }
                on:pointermove=move |ev| {
                    if let Some(start) = drag_start_y.get() {
                        let delta = (ev.client_y() as f64 - start).max(0.0);
                        drag_delta.set(delta);
                        if delta > 120.0 {
                            expanded.set(true);
                            drag_delta.set(0.0);
                            drag_start_y.set(None);
                        }
                    }
                }
                on:pointerup=move |_| {
                    if drag_delta.get() > 80.0 {
                        expanded.set(true);
                    }
                    drag_delta.set(0.0);
                    drag_start_y.set(None);
                }
                on:pointercancel=move |_| {
                    drag_delta.set(0.0);
                    drag_start_y.set(None);
                }
            ></div>
            <div class="bottom-sheet-body">
                <ToolDetailContent tool=tool compact=true full_page_href=full_page_href/>
            </div>
        </div>
    }
}