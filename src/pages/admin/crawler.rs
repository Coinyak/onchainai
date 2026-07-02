//! Admin crawler control — source status and manual triggers.

use crate::pages::admin::admin_page_shell;
use crate::server::functions::{
    list_crawler_sources, trigger_crawler_source, update_crawler_source, UpdateCrawlerSourcePayload,
};
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
                                        <th class="px-4 py-3 font-medium">"Interval (min)"</th>
                                        <th class="px-4 py-3 font-medium">"Enabled"</th>
                                        <th class="px-4 py-3 font-medium">"Actions"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {rows.into_iter().map(|row| {
                                        let name = row.name.clone();
                                        let trigger_name = name.clone();
                                        let source_id = row.id;
                                        let schedule_minutes = row.schedule_minutes;
                                        let enabled = row.enabled;
                                        view! {
                                            <CrawlerSourceRow
                                                row_id=source_id
                                                name=name
                                                url=row.url
                                                crawl_status=row.crawl_status
                                                error_message=row.error_message
                                                items_found=row.items_found
                                                last_crawled_at=row.last_crawled_at
                                                schedule=row.schedule
                                                schedule_minutes=schedule_minutes
                                                enabled=enabled
                                                action_busy=action_busy
                                                action_error=action_error
                                                refresh=refresh
                                                on_trigger=trigger
                                                trigger_name=trigger_name
                                            />
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
fn CrawlerSourceRow(
    row_id: Option<uuid::Uuid>,
    name: String,
    url: String,
    crawl_status: String,
    error_message: Option<String>,
    items_found: i32,
    last_crawled_at: Option<chrono::DateTime<chrono::Utc>>,
    schedule: String,
    schedule_minutes: i32,
    enabled: bool,
    action_busy: RwSignal<bool>,
    action_error: RwSignal<Option<String>>,
    refresh: RwSignal<u32>,
    on_trigger: impl Fn(String) + Clone + 'static,
    trigger_name: String,
) -> impl IntoView {
    let interval = RwSignal::new(schedule_minutes.to_string());
    let is_enabled = RwSignal::new(enabled);

    let save_schedule = move |_| {
        let Some(id) = row_id else {
            action_error.set(Some(
                "Source must run once before schedule can be saved.".into(),
            ));
            return;
        };
        if action_busy.get_untracked() {
            return;
        }
        let minutes = match interval.get_untracked().trim().parse::<i32>() {
            Ok(value) => value,
            Err(_) => {
                action_error.set(Some("Schedule interval must be numeric.".into()));
                return;
            }
        };
        action_busy.set(true);
        action_error.set(None);
        let enabled_value = is_enabled.get_untracked();
        spawn_local(async move {
            let result = update_crawler_source(
                id,
                UpdateCrawlerSourcePayload {
                    schedule_minutes: minutes,
                    enabled: enabled_value,
                },
            )
            .await;
            action_busy.set(false);
            match result {
                Ok(_) => refresh.update(|n| *n = n.wrapping_add(1)),
                Err(e) => action_error.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <tr class="border-t border-[#E5E5E5]">
            <td class="px-4 py-3">
                <div class="font-medium">{name}</div>
                <div class="text-[12px] text-[#6B6B6B] truncate max-w-[200px]">{url}</div>
            </td>
            <td class="px-4 py-3">
                <StatusBadge status=crawl_status/>
                {error_message.map(|e| {
                    let title = e.clone();
                    view! {
                        <div class="text-[12px] text-[#C0392B] mt-1 max-w-[180px] truncate" title=title>
                            {e}
                        </div>
                    }
                })}
            </td>
            <td class="px-4 py-3">{items_found}</td>
            <td class="px-4 py-3 text-[#6B6B6B]">{format_last_run(last_crawled_at)}</td>
            <td class="px-4 py-3 text-[#6B6B6B]">{schedule}</td>
            <td class="px-4 py-3">
                <input
                    class="w-20 rounded-md border border-[#E5E5E5] px-2 py-1 text-[13px]"
                    inputmode="numeric"
                    prop:value=move || interval.get()
                    on:input=move |ev| interval.set(event_target_value(&ev))
                    disabled=move || row_id.is_none()
                />
            </td>
            <td class="px-4 py-3">
                <input
                    type="checkbox"
                    prop:checked=move || is_enabled.get()
                    on:change=move |ev| is_enabled.set(event_target_checked(&ev))
                    disabled=move || row_id.is_none()
                />
            </td>
            <td class="px-4 py-3 space-x-2 whitespace-nowrap">
                <button
                    type="button"
                    class="text-[13px] px-3 py-1.5 rounded-md border border-[#E5E5E5] hover:bg-[#FAFAFA] disabled:opacity-50"
                    disabled=move || action_busy.get() || row_id.is_none()
                    on:click=move |_| save_schedule(())
                >
                    "Save"
                </button>
                <button
                    type="button"
                    class="text-[13px] px-3 py-1.5 rounded-md border border-[#E5E5E5] hover:bg-[#FAFAFA] disabled:opacity-50"
                    disabled=move || action_busy.get()
                    on:click=move |_| on_trigger(trigger_name.clone())
                >
                    "Run now"
                </button>
            </td>
        </tr>
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
