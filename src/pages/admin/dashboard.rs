//! Admin operations dashboard — queue counts, crawler health, quick links.

use crate::components::top_nav::TopNav;
use crate::pages::admin::admin_page_shell;
use crate::server::functions::{get_admin_dashboard_stats, AdminDashboardStats, CrawlerSourceView};
use leptos::prelude::*;

#[component]
pub fn AdminDashboardPage() -> impl IntoView {
    let stats = Resource::new(|| (), |_| async move { get_admin_dashboard_stats().await });

    admin_page_shell(move || {
        view! {
            <TopNav/>
            <div class="px-4 md:px-6 py-8 max-w-[1100px] mx-auto">
                <div class="mb-6">
                    <h1 class="text-[20px] font-semibold tracking-tight">"Operator Dashboard"</h1>
                    <p class="text-[#6B6B6B] text-[14px] mt-1">
                        "Review queue pressure, publication health, and crawler source status."
                    </p>
                </div>

                <Suspense fallback=|| view! {
                    <p class="text-[#6B6B6B] text-[14px]">"Loading dashboard..."</p>
                }>
                    {move || match stats.get() {
                        Some(Ok(data)) => view! { <DashboardContent data=data/> }.into_any(),
                        Some(Err(e)) => view! {
                            <p class="text-[14px] text-[#C0392B]">
                                "Failed to load dashboard: " {e.to_string()}
                            </p>
                        }.into_any(),
                        None => ().into_any(),
                    }}
                </Suspense>
            </div>
        }
    })
}

#[component]
fn DashboardContent(data: AdminDashboardStats) -> impl IntoView {
    view! {
        <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4 mb-8">
            <StatCard
                label="Pending candidates"
                value=data.pending_candidates
                href="/admin/tools?queue=new_candidate"
                accent=false
            />
            <StatCard
                label="Known updates"
                value=data.known_updates
                href="/admin/tools?queue=known_update"
                accent=false
            />
            <StatCard
                label="High risk installs"
                value=data.high_risk_installs
                href="/admin/tools?queue=high_risk_install"
                accent=true
            />
            <StatCard
                label="Open reports"
                value=data.open_reports
                href="/admin/tools?queue=reported"
                accent=false
            />
            <StatCard
                label="Needs manual research"
                value=data.needs_manual_research
                href="/admin/tools?queue=needs_manual_research"
                accent=false
            />
            <StatCard
                label="Low relevance"
                value=data.low_relevance
                href="/admin/tools?queue=low_relevance"
                accent=false
            />
            <StatCard
                label="Public tools"
                value=data.public_tool_count
                href="/tools"
                accent=false
            />
            <StatCard
                label="Reported queue"
                value=data.reported
                href="/admin/tools?queue=reported"
                accent=false
            />
        </div>

        <section class="mb-8">
            <div class="flex items-baseline justify-between gap-4 mb-3">
                <h2 class="text-[16px] font-semibold">"Crawler source health"</h2>
                <a href="/admin/crawler" class="text-[13px] text-[#E76F00] hover:underline">
                    "Crawler control"
                </a>
            </div>
            <div class="overflow-x-auto rounded-lg border border-[#E5E5E5]">
                <table class="w-full text-left text-[14px]">
                    <thead class="bg-[#FAFAFA] text-[#6B6B6B]">
                        <tr>
                            <th class="px-4 py-3 font-medium">"Source"</th>
                            <th class="px-4 py-3 font-medium">"Status"</th>
                            <th class="px-4 py-3 font-medium">"Items"</th>
                            <th class="px-4 py-3 font-medium">"Last successful crawl"</th>
                            <th class="px-4 py-3 font-medium">"Schedule"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {data.crawler_sources.into_iter().map(|row| view! {
                            <CrawlerHealthRow row=row/>
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        </section>

        <nav class="flex flex-col gap-2 max-w-[360px]">
            <h2 class="text-[16px] font-semibold mb-1">"Admin sections"</h2>
            <a
                href="/admin/tools"
                class="rounded-lg border border-[#E5E5E5] px-4 py-3 text-[14px] font-medium hover:bg-[#FAFAFA]"
            >
                "Review queues"
            </a>
            <a
                href="/admin/crawler"
                class="rounded-lg border border-[#E5E5E5] px-4 py-3 text-[14px] font-medium hover:bg-[#FAFAFA]"
            >
                "Crawler control"
            </a>
            <a
                href="/admin/featured"
                class="rounded-lg border border-[#E5E5E5] px-4 py-3 text-[14px] font-medium hover:bg-[#FAFAFA]"
            >
                "Featured carousel"
            </a>
            <a
                href="/admin/settings"
                class="rounded-lg border border-[#E5E5E5] px-4 py-3 text-[14px] font-medium hover:bg-[#FAFAFA]"
            >
                "Site settings"
            </a>
        </nav>
    }
}

#[component]
fn StatCard(label: &'static str, value: i64, href: &'static str, accent: bool) -> impl IntoView {
    let border = if accent {
        "border-[#C0392B]/30 bg-[#C0392B]/5"
    } else {
        "border-[#E5E5E5] bg-white"
    };
    view! {
        <a
            href=href
            class=format!("rounded-xl border px-4 py-4 block hover:bg-[#FAFAFA] no-underline text-inherit {border}")
        >
            <div class="text-[12px] text-[#6B6B6B] uppercase tracking-wide">{label}</div>
            <div class="text-[28px] font-semibold mt-1 leading-none">{value}</div>
        </a>
    }
}

#[component]
fn CrawlerHealthRow(row: CrawlerSourceView) -> impl IntoView {
    let name = row.name.clone();
    let schedule = row.schedule.clone();
    let items_found = row.items_found;
    let last_crawled_at = row.last_crawled_at;
    let status_class = match row.crawl_status.as_str() {
        "success" => "text-[#1A7F4B]".to_string(),
        "error" => "text-[#C0392B]".to_string(),
        _ => "text-[#6B6B6B]".to_string(),
    };
    let status_label = match row.crawl_status.as_str() {
        "success" => "OK".to_string(),
        "error" => "Error".to_string(),
        "pending" => "Pending".to_string(),
        other => other.to_string(),
    };
    view! {
        <tr class="border-t border-[#E5E5E5]">
            <td class="px-4 py-3 font-medium">{name}</td>
            <td class=format!("px-4 py-3 font-medium {status_class}")>{status_label}</td>
            <td class="px-4 py-3">{items_found}</td>
            <td class="px-4 py-3 text-[#6B6B6B]">{format_last_crawl(last_crawled_at)}</td>
            <td class="px-4 py-3 text-[#6B6B6B]">{schedule}</td>
        </tr>
    }
}

fn format_last_crawl(at: Option<chrono::DateTime<chrono::Utc>>) -> String {
    match at {
        Some(t) => t.format("%Y-%m-%d %H:%M UTC").to_string(),
        None => "—".into(),
    }
}
