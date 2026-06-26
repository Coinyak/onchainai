//! Admin crawler control — source status and manual triggers.

use crate::pages::admin::admin_page_shell;
use crate::server::functions::{list_crawler_sources, trigger_crawler_source};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn AdminCrawlerPage() -> impl IntoView {
    admin_page_shell(move || view! { <AdminCrawlerContent/> })
}

#[component]
fn AdminCrawlerContent() -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let sources = Resource::new(
        move || refresh.get(),
        |_| async move { list_crawler_sources().await },
    );
    let action_error = RwSignal::new(None::<String>);
    let action_busy = RwSignal::new(false);

    let trigger = move |source: String| {
        if action_busy.get_untracked() {
            return;
        }
        action_busy.set(true);
        action_error.set(None);
        spawn_local(async move {
            let result = trigger_crawler_source(source).await;
            action_busy.set(false);
            match result {
                Ok(()) => refresh.update(|n| *n = n.wrapping_add(1)),
                Err(e) => action_error.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <div class="px-4 md:px-6 py-8 max-w-[960px] mx-auto">
            <div class="mb-6">
                <h1 class="text-[20px] font-semibold tracking-tight">"Crawler Control"</h1>
                <p class="text-[#6B6B6B] text-[14px] mt-1">
                    "Monitor discovery sources and run crawls manually."
                </p>
            </div>

            {move || action_error.get().map(|msg| view! {
                <p class="text-[14px] text-[#C0392B] mb-4">{msg}</p>
            })}

            <Suspense fallback=|| view! {
                <p class="text-[#6B6B6B] text-[14px]">"Loading sources..."</p>
            }>
                {move || match sources.get() {
                    Some(Ok(rows)) => view! {
                        <div class="overflow-x-auto rounded-lg border border-[#E5E5E5]">
                            <table class="w-full text-left text-[14px]">
                                <thead class="bg-[#FAFAFA] text-[#6B6B6B]">
                                    <tr>
                                        <th class="px-4 py-3 font-medium">"Source"</th>
                                        <th class="px-4 py-3 font-medium">"Status"</th>
                                        <th class="px-4 py-3 font-medium">"Items"</th>
                                        <th class="px-4 py-3 font-medium">"Last run"</th>
                                        <th class="px-4 py-3 font-medium">"Schedule"</th>
                                        <th class="px-4 py-3 font-medium">"Actions"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {rows.into_iter().map(|row| {
                                        let name = row.name.clone();
                                        let trigger_name = name.clone();
                                        view! {
                                            <tr class="border-t border-[#E5E5E5]">
                                                <td class="px-4 py-3">
                                                    <div class="font-medium">{name}</div>
                                                    <div class="text-[12px] text-[#6B6B6B] truncate max-w-[200px]">
                                                        {row.url}
                                                    </div>
                                                </td>
                                                <td class="px-4 py-3">
                                                    <StatusBadge status=row.crawl_status.clone()/>
                                                    {row.error_message.clone().map(|e| {
                                                        let title = e.clone();
                                                        view! {
                                                            <div class="text-[12px] text-[#C0392B] mt-1 max-w-[180px] truncate" title=title>
                                                                {e}
                                                            </div>
                                                        }
                                                    })}
                                                </td>
                                                <td class="px-4 py-3">{row.items_found}</td>
                                                <td class="px-4 py-3 text-[#6B6B6B]">
                                                    {format_last_run(row.last_crawled_at)}
                                                </td>
                                                <td class="px-4 py-3 text-[#6B6B6B]">{row.schedule}</td>
                                                <td class="px-4 py-3">
                                                    <button
                                                        type="button"
                                                        class="text-[13px] px-3 py-1.5 rounded-md border border-[#E5E5E5] hover:bg-[#FAFAFA] disabled:opacity-50"
                                                        disabled=move || action_busy.get()
                                                        on:click=move |_| trigger(trigger_name.clone())
                                                    >
                                                        "Run now"
                                                    </button>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                        <div class="mt-6 rounded-lg border border-[#E5E5E5] p-4 flex items-center justify-between gap-4">
                            <div>
                                <div class="font-medium text-[14px]">"GitHub Stars Sync"</div>
                                <div class="text-[12px] text-[#6B6B6B]">"Updates star counts for known repos (every 30m scheduled)."</div>
                            </div>
                            <button
                                type="button"
                                class="text-[13px] px-3 py-1.5 rounded-md bg-[#1A1A1A] text-white hover:opacity-90 disabled:opacity-50"
                                disabled=move || action_busy.get()
                                on:click=move |_| trigger("sync_stars".into())
                            >
                                "Sync now"
                            </button>
                        </div>
                    }.into_any(),
                    Some(Err(e)) => view! {
                        <p class="text-[14px] text-[#C0392B]">"Failed to load sources: " {e.to_string()}</p>
                    }.into_any(),
                    None => ().into_any(),
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn StatusBadge(status: String) -> impl IntoView {
    let (label, class) = match status.as_str() {
        "success" => ("OK".to_string(), "text-[#1A7F4B]"),
        "error" => ("Error".to_string(), "text-[#C0392B]"),
        "pending" => ("Pending".to_string(), "text-[#6B6B6B]"),
        other => (other.to_string(), "text-[#6B6B6B]"),
    };
    view! {
        <span class=format!("font-medium {class}")>{label}</span>
    }
}

fn format_last_run(at: Option<chrono::DateTime<chrono::Utc>>) -> String {
    match at {
        Some(t) => t.format("%Y-%m-%d %H:%M UTC").to_string(),
        None => "—".into(),
    }
}
