//! Mobile bottom sheet — 60% slide-up with drag-to-expand.

use crate::components::tool_detail_content::ToolDetailContent;
use crate::models::Tool;
use leptos::prelude::*;

#[component]
pub fn BottomSheet(tool: Tool, close_href: String, full_page_href: String) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let drag_start_y = RwSignal::new(None::<f64>);
    let drag_delta = RwSignal::new(0.0_f64);
    let close_stored = StoredValue::new(close_href.clone());

    let sheet_class = move || {
        if expanded.get() {
            "bottom-sheet bottom-sheet-full"
        } else {
            "bottom-sheet"
        }
    };

    view! {
        <a href=close_stored.get_value() class="bottom-sheet-backdrop" aria-label="Close preview">
            <span class="sr-only">"Close"</span>
        </a>
        <div
            class=sheet_class
            role="dialog"
            aria-label="Tool preview"
            style=move || {
                let d = drag_delta.get();
                if d != 0.0 && !expanded.get() {
                    // Only apply downward drag as visual feedback (closing direction)
                    if d > 0.0 { format!("transform: translateY({d}px)") } else { String::new() }
                } else if d < 0.0 && expanded.get() {
                    // While expanded, show downward drag as visual feedback
                    format!("transform: translateY({})", d.max(0.0))
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
                        let delta = ev.client_y() as f64 - start;
                        drag_delta.set(delta);
                        if !expanded.get() {
                            // Dragging up (negative delta) → expand
                            if delta < -80.0 {
                                expanded.set(true);
                                drag_delta.set(0.0);
                                drag_start_y.set(None);
                            }
                        } else {
                            // Dragging down (positive delta) while expanded → close
                            if delta > 100.0 {
                                #[cfg(feature = "hydrate")]
                                if let Some(win) = web_sys::window() {
                                    let _ = win.location().set_href(&close_stored.get_value());
                                }
                                drag_delta.set(0.0);
                                drag_start_y.set(None);
                            }
                        }
                    }
                }
                on:pointerup=move |_| {
                    if drag_start_y.get().is_some() {
                        let delta = drag_delta.get();
                        if !expanded.get() && delta < -60.0 {
                            expanded.set(true);
                        } else if expanded.get() && delta > 80.0 {
                            #[cfg(feature = "hydrate")]
                            if let Some(win) = web_sys::window() {
                                let _ = win.location().set_href(&close_stored.get_value());
                            }
                        }
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
