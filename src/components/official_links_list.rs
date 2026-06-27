//! Official links with icon treatment and neutral fallback labels.

use crate::models::{official_link_display_label, ToolOfficialLink};
use leptos::prelude::*;

fn link_icon_label(link_type: &str) -> &'static str {
    match link_type {
        "github" => "GH",
        "website" => "Web",
        "x" => "X",
        _ => "Link",
    }
}

#[component]
pub fn OfficialLinksList(links: Vec<ToolOfficialLink>) -> impl IntoView {
    let rendered = links.clone();
    view! {
        {(!rendered.is_empty()).then(|| {
            let items = rendered.clone();
            view! {
                <section class="rounded-xl border border-[#E5E5E5] bg-white p-4 mt-4">
                    <h3 class="text-[14px] font-semibold mb-3">"Official links"</h3>
                    <ul class="space-y-2">
                        {items.into_iter().map(|link| {
                            let label = official_link_display_label(&link);
                            let href = link.url.clone();
                            let display_url = link.url.clone();
                            let icon_label = link_icon_label(&link.link_type);
                            view! {
                                <li>
                                    <a
                                        href=href
                                        class="flex items-center gap-2 text-[14px] text-[#1A1A1A] hover:underline no-underline"
                                        target="_blank"
                                        rel="noopener noreferrer"
                                    >
                                        <span
                                            class="inline-flex items-center justify-center w-6 h-6 rounded border border-[#E5E5E5] text-[10px] font-semibold text-[#6B6B6B] shrink-0"
                                            aria-hidden="true"
                                        >
                                            {icon_label}
                                        </span>
                                        <span class="font-medium">{label}</span>
                                        <span class="text-[#6B6B6B] font-mono text-[12px] truncate">{display_url}</span>
                                    </a>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </section>
            }
        })}
    }
}
