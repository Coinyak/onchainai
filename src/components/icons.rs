//! Minimal Lucide-style SVG icons (stroke #4B4B4B, no emojis).

use leptos::prelude::*;

/// Render a 20×20 Lucide-style icon by name from `categories.icon`.
#[component]
pub fn LucideIcon(name: String) -> impl IntoView {
    let stroke = "#4B4B4B";
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke=stroke
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            {match name.as_str() {
                "git-branch" => view! {
                    <line x1="6" y1="3" x2="6" y2="15"/>
                    <circle cx="18" cy="6" r="3"/>
                    <circle cx="6" cy="18" r="3"/>
                    <path d="M18 9a9 9 0 0 1-9 9"/>
                }.into_any(),
                "arrow-left-right" => view! {
                    <path d="M8 3 4 7l4 4"/>
                    <path d="M4 7h16"/>
                    <path d="m16 21 4-4-4-4"/>
                    <path d="M20 17H4"/>
                }.into_any(),
                "credit-card" => view! {
                    <rect width="20" height="14" x="2" y="5" rx="2"/>
                    <line x1="2" y1="10" x2="22" y2="10"/>
                }.into_any(),
                "dollar-sign" => view! {
                    <line x1="12" y1="2" x2="12" y2="22"/>
                    <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>
                }.into_any(),
                "banknote" => view! {
                    <rect width="20" height="12" x="2" y="6" rx="2"/>
                    <circle cx="12" cy="12" r="2"/>
                    <path d="M6 12h.01M18 12h.01"/>
                }.into_any(),
                "lock" => view! {
                    <rect width="18" height="11" x="3" y="11" rx="2" ry="2"/>
                    <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                }.into_any(),
                "trending-up" => view! {
                    <polyline points="22 7 13.5 15.5 8.5 10.5 2 17"/>
                    <polyline points="16 7 22 7 22 13"/>
                }.into_any(),
                "image" => view! {
                    <rect width="18" height="18" x="3" y="3" rx="2" ry="2"/>
                    <circle cx="9" cy="9" r="2"/>
                    <path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/>
                }.into_any(),
                "bar-chart" => view! {
                    <line x1="12" y1="20" x2="12" y2="10"/>
                    <line x1="18" y1="20" x2="18" y2="4"/>
                    <line x1="6" y1="20" x2="6" y2="16"/>
                }.into_any(),
                "terminal" => view! {
                    <polyline points="4 17 10 11 4 5"/>
                    <line x1="12" y1="19" x2="20" y2="19"/>
                }.into_any(),
                "fingerprint" => view! {
                    <path d="M12 10a2 2 0 0 0-2 2c0 1.02-.1 2.51-.26 4"/>
                    <path d="M14 13.12c0 2.38 0 6.38-1 8.88"/>
                    <path d="M17.29 21.02c.12-.6.43-2.3.5-3.02"/>
                    <path d="M2 12a10 10 0 0 1 18-6"/>
                    <path d="M2 16h.01"/>
                    <path d="M21.8 16c.2-1 .5-2.5.7-4"/>
                }.into_any(),
                "vote" => view! {
                    <path d="m9 12 2 2 4-4"/>
                    <path d="M5 7c0-1.1.9-2 2-2h10a2 2 0 0 1 2 2v12H5V7z"/>
                }.into_any(),
                "message-circle" => view! {
                    <path d="M7.9 20A9 9 0 1 0 4 16.1L2 22Z"/>
                }.into_any(),
                "bot" => view! {
                    <path d="M12 8V4H8"/>
                    <rect width="16" height="12" x="4" y="8" rx="2"/>
                    <path d="M2 14h2"/>
                    <path d="M20 14h2"/>
                    <path d="M15 13v2"/>
                    <path d="M9 13v2"/>
                }.into_any(),
                _ => view! { <circle cx="12" cy="12" r="8"/> }.into_any(),
            }}
        </svg>
    }
}
