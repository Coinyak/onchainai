//! Public dashboard page — no-login market map for tool coverage.

use crate::components::error_state::ErrorState;
use crate::components::site_shell::SiteShell;
use crate::components::skeleton::ToolListSkeleton;
use crate::components::tool_card::ToolCard;
use crate::server::functions::{
    get_public_dashboard_snapshot, DashboardBucket, PublicDashboardSnapshot,
};
use leptos::prelude::*;

fn number_label(value: i64) -> String {
    value.to_string()
}

#[component]
fn MetricTile(label: &'static str, value: i64, href: &'static str) -> impl IntoView {
    view! {
        <a href=href class="dashboard-metric-tile no-underline">
            <span class="dashboard-metric-value">{number_label(value)}</span>
            <span class="dashboard-metric-label">{label}</span>
        </a>
    }
}

#[component]
fn BucketPanel(title: &'static str, buckets: Vec<DashboardBucket>) -> impl IntoView {
    view! {
        <section class="dashboard-panel">
            <h2>{title}</h2>
            <div class="dashboard-bucket-list">
                {if buckets.is_empty() {
                    view! { <p class="dashboard-muted">"No data yet."</p> }.into_any()
                } else {
                    buckets.into_iter().map(|bucket| {
                        view! {
                            <a href=bucket.href class="dashboard-bucket-row no-underline">
                                <span>{bucket.label}</span>
                                <strong>{number_label(bucket.count)}</strong>
                            </a>
                        }
                    }).collect_view().into_any()
                }}
            </div>
        </section>
    }
}

#[component]
fn ToolRail(title: &'static str, tools: Vec<crate::models::Tool>) -> impl IntoView {
    view! {
        <section class="dashboard-panel dashboard-tool-panel">
            <div class="dashboard-panel-heading">
                <h2>{title}</h2>
                <a href="/tools" class="dashboard-panel-link">"All tools"</a>
            </div>
            <div class="dashboard-tool-list">
                {if tools.is_empty() {
                    view! { <p class="dashboard-muted">"No tools yet."</p> }.into_any()
                } else {
                    tools.into_iter().map(|tool| {
                        view! { <ToolCard tool=tool initially_starred=false/> }
                    }).collect_view().into_any()
                }}
            </div>
        </section>
    }
}

#[component]
fn DashboardContent(snapshot: PublicDashboardSnapshot) -> impl IntoView {
    let metrics = snapshot.metrics.clone();
    view! {
        <div class="public-dashboard-page">
            <section class="dashboard-header">
                <div>
                    <p class="dashboard-kicker">"Public dashboard"</p>
                    <h1>"Crypto tool coverage"</h1>
                    <p>
                        "Live snapshot of approved MCP, CLI, SDK, API, x402, RWA, and agent tools indexed by OnchainAI."
                    </p>
                </div>
                <div class="dashboard-as-of">
                    <span>"Snapshot"</span>
                    <strong>{snapshot.as_of.format("%Y-%m-%d %H:%M UTC").to_string()}</strong>
                </div>
            </section>

            <section class="dashboard-metrics-grid" aria-label="Dashboard metrics">
                <MetricTile label="Public tools" value=metrics.public_tools href="/tools"/>
                <MetricTile label="MCP" value=metrics.mcp_tools href="/tools?type=mcp"/>
                <MetricTile label="CLI" value=metrics.cli_tools href="/tools?type=cli"/>
                <MetricTile label="SDK" value=metrics.sdk_tools href="/tools?type=sdk"/>
                <MetricTile label="API" value=metrics.api_tools href="/tools?type=api"/>
                <MetricTile label="x402" value=metrics.x402_tools href="/tools?pricing=x402"/>
                <MetricTile label="Official" value=metrics.official_tools href="/tools?status=official"/>
                <MetricTile label="Verified" value=metrics.verified_tools href="/tools?status=verified"/>
                <MetricTile label="Updated 30d" value=metrics.updated_recently href="/tools?sort=new"/>
            </section>

            <div class="dashboard-grid">
                <BucketPanel title="Tool types" buckets=snapshot.type_counts/>
                <BucketPanel title="Functions" buckets=snapshot.function_counts/>
                <BucketPanel title="Chains" buckets=snapshot.chain_counts/>
                <BucketPanel title="Trust" buckets=snapshot.trust_counts/>
                <BucketPanel title="Pricing" buckets=snapshot.pricing_counts/>
            </div>

            <div class="dashboard-rails">
                <ToolRail title="Popular tools" tools=snapshot.popular_tools/>
                <ToolRail title="Newest tools" tools=snapshot.new_tools/>
                <ToolRail title="x402-ready tools" tools=snapshot.x402_tools/>
                <ToolRail title="High-trust tools" tools=snapshot.high_trust_tools/>
            </div>
        </div>
    }
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    let retry = RwSignal::new(0u32);
    let snapshot = Resource::new_blocking(
        move || retry.get(),
        |_| async move { get_public_dashboard_snapshot(6).await },
    );

    view! {
        <SiteShell>
            <Suspense fallback=|| view! { <div class="public-dashboard-page"><ToolListSkeleton count=6/></div> }>
                {move || match snapshot.get() {
                    Some(Ok(data)) => view! { <DashboardContent snapshot=data/> }.into_any(),
                    Some(Err(error)) => view! {
                        <div class="public-dashboard-page">
                            <ErrorState
                                message=format!("Dashboard failed to load: {error}")
                                on_retry=move || retry.update(|n| *n = n.wrapping_add(1))
                            />
                        </div>
                    }.into_any(),
                    None => view! { <div class="public-dashboard-page"><ToolListSkeleton count=6/></div> }.into_any(),
                }}
            </Suspense>
        </SiteShell>
    }
}
