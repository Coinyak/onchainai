//! Admin tool review — split operator queues with gated review actions.

use crate::models::Tool;
use crate::pages::admin::admin_page_shell;
use crate::server::functions::{
    get_referral_dashboard_stats, list_review_queue, review_tool, update_tool_referral,
    ReferralDashboardStats, ReviewQueueItem, ReviewToolPayload, UpdateToolReferralPayload,
    REVIEW_QUEUES,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;
use std::sync::Arc;

type ReviewHandler = Arc<dyn Fn(ReviewToolPayload) + Send + Sync>;

#[derive(Clone, Copy)]
struct QueueTab {
    id: &'static str,
    label: &'static str,
}

const QUEUE_TABS: &[QueueTab] = &[
    QueueTab {
        id: "new_candidate",
        label: "New candidates",
    },
    QueueTab {
        id: "known_update",
        label: "Known updates",
    },
    QueueTab {
        id: "needs_manual_research",
        label: "Manual research",
    },
    QueueTab {
        id: "low_relevance",
        label: "Low relevance",
    },
    QueueTab {
        id: "reported",
        label: "Reported",
    },
    QueueTab {
        id: "high_risk_install",
        label: "High risk install",
    },
];

#[component]
pub fn AdminToolsPage() -> impl IntoView {
    admin_page_shell(move || view! { <AdminToolsContent/> })
}

#[component]
fn AdminToolsContent() -> impl IntoView {
    let query = use_query_map();
    let active_queue = RwSignal::new("new_candidate".to_string());
    Effect::new(move |_| {
        if let Some(queue) = query.get().get("queue").map(|q| q.to_string()) {
            if REVIEW_QUEUES.contains(&queue.as_str()) {
                active_queue.set(queue);
            }
        }
    });

    let refresh = RwSignal::new(0u32);
    let referral_stats_refresh = RwSignal::new(0u32);
    let queue_items = Resource::new(
        move || (active_queue.get(), refresh.get()),
        |(queue, _)| async move { list_review_queue(queue, 50).await },
    );
    let referral_stats = Resource::new(
        move || referral_stats_refresh.get(),
        |_| async move { get_referral_dashboard_stats().await },
    );

    let reason_modal = RwSignal::new(None::<ReasonModalState>);
    let action_error = RwSignal::new(None::<String>);
    let action_busy = RwSignal::new(false);

    let run_review: ReviewHandler = Arc::new(move |payload: ReviewToolPayload| {
        if action_busy.get_untracked() {
            return;
        }
        action_busy.set(true);
        action_error.set(None);
        spawn_local(async move {
            let result = review_tool(payload).await;
            action_busy.set(false);
            match result {
                Ok(()) => {
                    reason_modal.set(None);
                    refresh.update(|n| *n = n.wrapping_add(1));
                }
                Err(e) => action_error.set(Some(e.to_string())),
            }
        });
    });

    let run_review_for_rows = run_review.clone();
    let run_review_for_modal = run_review.clone();

    view! {
        <div class="px-4 md:px-6 py-8 max-w-[1100px] mx-auto">
            <div class="mb-6">
                <h1 class="text-[20px] font-semibold tracking-tight">"Review Queues"</h1>
                <p class="text-[#6B6B6B] text-[14px] mt-1">
                    "Split operator queues with relevance, install safety, and audit-backed actions."
                </p>
            </div>

            <Suspense fallback=|| view! {
                <div class="mb-6 grid grid-cols-2 md:grid-cols-4 gap-3">
                    <ReferralStatSkeleton/>
                    <ReferralStatSkeleton/>
                    <ReferralStatSkeleton/>
                    <ReferralStatSkeleton/>
                </div>
            }>
                {move || {
                    referral_stats.get().map(|res| match res {
                        Ok(stats) => view! { <ReferralStatsBar stats=stats/> }.into_any(),
                        Err(e) => view! {
                            <p class="mb-6 text-[14px] text-[#C0392B]">
                                "Failed to load referral stats: " {e.to_string()}
                            </p>
                        }.into_any(),
                    })
                }}
            </Suspense>

            <div class="flex flex-wrap gap-2 mb-6">
                {QUEUE_TABS.iter().map(|tab| {
                    let tab_id = tab.id;
                    let href = format!("/admin/tools?queue={tab_id}");
                    view! {
                        <a
                            href=href
                            class=move || {
                                if active_queue.get() == tab_id {
                                    "px-3 py-1.5 rounded-lg text-[13px] font-medium bg-[#1A1A1A] text-white no-underline"
                                } else {
                                    "px-3 py-1.5 rounded-lg text-[13px] font-medium border border-[#E5E5E5] hover:bg-[#FAFAFA] no-underline text-inherit"
                                }
                            }
                        >
                            {tab.label}
                        </a>
                    }
                }).collect_view()}
            </div>

            {move || {
                action_error.get().map(|msg| view! {
                    <p class="mb-4 text-[14px] text-[#C0392B] border border-[#C0392B]/30 rounded-lg px-4 py-2 bg-[#C0392B]/5">
                        {msg}
                    </p>
                })
            }}

            <Suspense fallback=|| view! {
                <p class="text-[#6B6B6B] text-[14px]">"Loading review queue..."</p>
            }>
                {move || {
                    queue_items.get().map(|res| match res {
                        Ok(items) if items.is_empty() => view! {
                            <div class="rounded-xl border border-[#E5E5E5] bg-[#FAFAFA] px-6 py-10 text-center">
                                <p class="text-[16px] font-medium mb-2">"Queue empty"</p>
                                <p class="text-[#6B6B6B] text-[14px]">
                                    "No tools match this queue right now."
                                </p>
                            </div>
                        }.into_any(),
                        Ok(items) => view! {
                            <div class="space-y-4">
                                <p class="text-[14px] text-[#6B6B6B]">
                                    {format!("{} in queue", items.len())}
                                </p>
                                {items
                                    .into_iter()
                                    .map(|item| {
                                        view! {
                                            <ReviewToolRow
                                                item=item
                                                run_review=run_review_for_rows.clone()
                                                reason_modal=reason_modal
                                                action_busy=action_busy
                                                referral_stats_refresh=referral_stats_refresh
                                            />
                                        }
                                    })
                                    .collect::<Vec<_>>()}
                            </div>
                        }.into_any(),
                        Err(e) => view! {
                            <p class="text-[14px] text-[#C0392B]">
                                "Failed to load review queue: " {e.to_string()}
                            </p>
                        }.into_any(),
                    })
                }}
            </Suspense>

            {move || {
                reason_modal.get().map(|state| {
                    let run_review = run_review_for_modal.clone();
                    view! {
                        <ReasonModal
                            state=state
                            run_review=run_review
                            reason_modal=reason_modal
                            action_busy=action_busy
                        />
                    }
                })
            }}
        </div>
    }
}

#[component]
fn ReferralStatsBar(stats: ReferralDashboardStats) -> impl IntoView {
    view! {
        <div class="mb-6 grid grid-cols-2 md:grid-cols-4 gap-3">
            <ReferralStatCard label="x402 tools" value=stats.x402_tools/>
            <ReferralStatCard label="Referral enabled" value=stats.referral_enabled_tools/>
            <ReferralStatCard label="Attribution events" value=stats.attribution_events/>
            <ReferralStatCard label="Reported settlements" value=stats.reported_settlements/>
        </div>
    }
}

#[component]
fn ReferralStatCard(label: &'static str, value: i64) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-[#E5E5E5] bg-white px-4 py-3">
            <p class="text-[12px] text-[#6B6B6B]">{label}</p>
            <p class="text-[20px] font-semibold mt-1">{value}</p>
        </div>
    }
}

#[component]
fn ReferralStatSkeleton() -> impl IntoView {
    view! {
        <div class="rounded-lg border border-[#E5E5E5] bg-[#FAFAFA] px-4 py-3">
            <p class="text-[12px] text-[#6B6B6B]">"Loading"</p>
            <p class="text-[20px] font-semibold mt-1">"..."</p>
        </div>
    }
}

#[derive(Clone)]
struct ReasonModalState {
    slug: String,
    action: String,
    title: String,
    placeholder: String,
    confirm_label: String,
    confirm_class: String,
    override_mode: bool,
}

#[component]
fn ReasonModal(
    state: ReasonModalState,
    run_review: ReviewHandler,
    reason_modal: RwSignal<Option<ReasonModalState>>,
    action_busy: RwSignal<bool>,
) -> impl IntoView {
    let reason = RwSignal::new(String::new());
    let slug = state.slug.clone();
    let action = state.action.clone();

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40">
            <div
                class="w-full max-w-md rounded-xl bg-white border border-[#E5E5E5] shadow-lg p-5"
                role="dialog"
                aria-labelledby="reason-modal-title"
            >
                <h3 id="reason-modal-title" class="text-[16px] font-semibold mb-2">
                    {state.title.clone()}
                </h3>
                <p class="text-[14px] text-[#6B6B6B] mb-4">
                    <span class="font-mono">{slug.clone()}</span>
                </p>
                <textarea
                    class="w-full min-h-[96px] rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px] focus:outline-none focus:border-[#E76F00]"
                    placeholder=state.placeholder.clone()
                    prop:value=move || reason.get()
                    on:input=move |ev| reason.set(event_target_value(&ev))
                />
                <div class="flex justify-end gap-2 mt-4">
                    <button
                        type="button"
                        class="px-3 py-1.5 text-[14px] rounded-lg border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                        on:click=move |_| reason_modal.set(None)
                    >
                        "Cancel"
                    </button>
                    <button
                        type="button"
                        class=format!(
                            "px-3 py-1.5 text-[14px] rounded-lg text-white hover:opacity-90 disabled:opacity-50 {}",
                            state.confirm_class
                        )
                        disabled=move || action_busy.get() || reason.get().trim().is_empty()
                        on:click=move |_| {
                            let text = reason.get().trim().to_string();
                            if text.is_empty() {
                                return;
                            }
                            let payload = if state.override_mode {
                                ReviewToolPayload {
                                    slug: slug.clone(),
                                    action: "approved".into(),
                                    reason: "Approved with operator override".into(),
                                    override_reason: Some(text),
                                    expected_updated_at: None,
                                    snapshot_id: None,
                                    recommendation_id: None,
                                }
                            } else {
                                ReviewToolPayload {
                                    slug: slug.clone(),
                                    action: action.clone(),
                                    reason: text,
                                    override_reason: None,
                                    expected_updated_at: None,
                                    snapshot_id: None,
                                    recommendation_id: None,
                                }
                            };
                            run_review(payload);
                        }
                    >
                        {state.confirm_label.clone()}
                    </button>
                </div>
            </div>
        </div>
    }
}

fn needs_override(item: &ReviewQueueItem) -> bool {
    item.tool.relevance_status == "rejected" || item.tool.install_risk_level == "critical"
}

#[component]
fn ReferralSettingsPanel(tool: Tool, referral_stats_refresh: RwSignal<u32>) -> impl IntoView {
    let referral_enabled = RwSignal::new(tool.referral_enabled);
    let referral_bps = RwSignal::new(
        tool.referral_bps
            .map(|value| value.to_string())
            .unwrap_or_default(),
    );
    let referral_payout_address =
        RwSignal::new(tool.referral_payout_address.clone().unwrap_or_default());
    let referral_model = RwSignal::new(
        tool.referral_model
            .clone()
            .unwrap_or_else(|| "attribution".into()),
    );
    let x402_pay_to_address = RwSignal::new(tool.x402_pay_to_address.clone().unwrap_or_default());
    let x402_builder_code = RwSignal::new(tool.x402_builder_code.clone().unwrap_or_default());
    let payment_verified = RwSignal::new(tool.payment_verified);
    let x402_endpoint_verified = RwSignal::new(tool.x402_endpoint_verified);
    let price_verified = RwSignal::new(tool.price_verified);
    let save_error = RwSignal::new(None::<String>);
    let save_ok = RwSignal::new(false);
    let saving = RwSignal::new(false);
    let slug = tool.slug.clone();

    let save = move |_| {
        if saving.get_untracked() {
            return;
        }
        let bps_text = referral_bps.get_untracked();
        let parsed_bps = if bps_text.trim().is_empty() {
            None
        } else {
            match bps_text.trim().parse::<i32>() {
                Ok(value) => Some(value),
                Err(_) => {
                    save_error.set(Some("Referral bps must be numeric.".into()));
                    save_ok.set(false);
                    return;
                }
            }
        };

        saving.set(true);
        save_error.set(None);
        save_ok.set(false);

        let payload = UpdateToolReferralPayload {
            slug: slug.clone(),
            referral_enabled: referral_enabled.get_untracked(),
            referral_bps: parsed_bps,
            referral_payout_address: Some(referral_payout_address.get_untracked())
                .filter(|value| !value.trim().is_empty()),
            referral_model: Some(referral_model.get_untracked())
                .filter(|value| !value.trim().is_empty()),
            x402_pay_to_address: Some(x402_pay_to_address.get_untracked())
                .filter(|value| !value.trim().is_empty()),
            x402_builder_code: Some(x402_builder_code.get_untracked())
                .filter(|value| !value.trim().is_empty()),
            payment_verified: payment_verified.get_untracked(),
            x402_endpoint_verified: x402_endpoint_verified.get_untracked(),
            price_verified: price_verified.get_untracked(),
        };

        spawn_local(async move {
            let result = update_tool_referral(payload).await;
            saving.set(false);
            match result {
                Ok(updated) => {
                    referral_enabled.set(updated.referral_enabled);
                    referral_bps.set(
                        updated
                            .referral_bps
                            .map(|value| value.to_string())
                            .unwrap_or_default(),
                    );
                    referral_payout_address
                        .set(updated.referral_payout_address.unwrap_or_default());
                    referral_model.set(
                        updated
                            .referral_model
                            .unwrap_or_else(|| "attribution".into()),
                    );
                    x402_pay_to_address.set(updated.x402_pay_to_address.unwrap_or_default());
                    x402_builder_code.set(updated.x402_builder_code.unwrap_or_default());
                    payment_verified.set(updated.payment_verified);
                    x402_endpoint_verified.set(updated.x402_endpoint_verified);
                    price_verified.set(updated.price_verified);
                    save_ok.set(true);
                    referral_stats_refresh.update(|n| *n = n.wrapping_add(1));
                }
                Err(e) => save_error.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <section class="mt-4 rounded-lg border border-[#E5E5E5] bg-[#FAFAFA] p-4">
            <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-2 mb-3">
                <div>
                    <h3 class="text-[14px] font-medium">"x402 referral"</h3>
                    <p class="text-[12px] text-[#6B6B6B] mt-1">
                        "Verification flags are trust signals only; unverified x402 tools can stay public."
                    </p>
                </div>
                <label class="flex items-center gap-2 text-[13px]">
                    <input
                        type="checkbox"
                        prop:checked=move || referral_enabled.get()
                        on:change=move |ev| referral_enabled.set(event_target_checked(&ev))
                    />
                    "Referral enabled"
                </label>
            </div>

            <div class="grid md:grid-cols-2 gap-3">
                <label class="block">
                    <span class="text-[12px] text-[#6B6B6B]">"Bps"</span>
                    <input
                        class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[13px] font-mono bg-white"
                        inputmode="numeric"
                        placeholder="250"
                        prop:value=move || referral_bps.get()
                        on:input=move |ev| referral_bps.set(event_target_value(&ev))
                    />
                </label>
                <label class="block">
                    <span class="text-[12px] text-[#6B6B6B]">"Model"</span>
                    <select
                        class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[13px] bg-white"
                        prop:value=move || referral_model.get()
                        on:change=move |ev| referral_model.set(event_target_value(&ev))
                    >
                        <option value="attribution">"Attribution"</option>
                        <option value="split">"Split"</option>
                    </select>
                </label>
                <label class="block md:col-span-2">
                    <span class="text-[12px] text-[#6B6B6B]">"Referral payout address"</span>
                    <input
                        class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[13px] font-mono bg-white"
                        placeholder="0x..."
                        prop:value=move || referral_payout_address.get()
                        on:input=move |ev| referral_payout_address.set(event_target_value(&ev))
                    />
                </label>
                <label class="block">
                    <span class="text-[12px] text-[#6B6B6B]">"Builder code"</span>
                    <input
                        class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[13px] font-mono bg-white"
                        placeholder="onchainai"
                        prop:value=move || x402_builder_code.get()
                        on:input=move |ev| x402_builder_code.set(event_target_value(&ev))
                    />
                </label>
                <label class="block">
                    <span class="text-[12px] text-[#6B6B6B]">"Tool pay-to address"</span>
                    <input
                        class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[13px] font-mono bg-white"
                        placeholder="0x..."
                        prop:value=move || x402_pay_to_address.get()
                        on:input=move |ev| x402_pay_to_address.set(event_target_value(&ev))
                    />
                </label>
            </div>

            <div class="flex flex-wrap gap-3 mt-3">
                <label class="flex items-center gap-2 text-[13px]">
                    <input
                        type="checkbox"
                        prop:checked=move || payment_verified.get()
                        on:change=move |ev| payment_verified.set(event_target_checked(&ev))
                    />
                    "Payment address"
                </label>
                <label class="flex items-center gap-2 text-[13px]">
                    <input
                        type="checkbox"
                        prop:checked=move || x402_endpoint_verified.get()
                        on:change=move |ev| x402_endpoint_verified.set(event_target_checked(&ev))
                    />
                    "Endpoint"
                </label>
                <label class="flex items-center gap-2 text-[13px]">
                    <input
                        type="checkbox"
                        prop:checked=move || price_verified.get()
                        on:change=move |ev| price_verified.set(event_target_checked(&ev))
                    />
                    "Price"
                </label>
            </div>

            {move || save_error.get().map(|msg| view! {
                <p class="text-[13px] text-[#C0392B] mt-3">{msg}</p>
            })}
            {move || save_ok.get().then(|| view! {
                <p class="text-[13px] text-[#2D7D46] mt-3">"Referral settings saved."</p>
            })}

            <button
                type="button"
                class="mt-3 px-3 py-1.5 text-[13px] rounded-lg bg-[#1A1A1A] text-white hover:opacity-90 disabled:opacity-50"
                disabled=move || saving.get()
                on:click=save
            >
                {move || if saving.get() { "Saving..." } else { "Save referral" }}
            </button>
        </section>
    }
}

#[component]
fn ReviewToolRow(
    item: ReviewQueueItem,
    run_review: ReviewHandler,
    reason_modal: RwSignal<Option<ReasonModalState>>,
    action_busy: RwSignal<bool>,
    referral_stats_refresh: RwSignal<u32>,
) -> impl IntoView {
    let tool = item.tool.clone();
    let slug = tool.slug.clone();
    let slug_href = slug.clone();
    let slug_for_approve = slug.clone();
    let slug_for_override = slug.clone();
    let slug_for_reject = slug.clone();
    let slug_for_needs_info = slug.clone();
    let slug_for_quarantine = slug.clone();
    let slug_for_verified = slug.clone();
    let slug_for_official = slug.clone();

    let has_url = tool.repo_url.is_some()
        || tool.homepage.is_some()
        || tool.npm_package.is_some()
        || tool.mcp_endpoint.is_some();
    let override_needed = needs_override(&item);
    let relevance_reasons = tool.crypto_relevance_reasons.join("; ");
    let install_reasons = tool.install_risk_reasons.join("; ");
    let last_commit = tool
        .last_commit_at
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "—".into());

    view! {
        <article class="rounded-xl border border-[#E5E5E5] bg-white p-4 md:p-5">
            <div class="flex flex-col gap-4">
                <div class="flex flex-col md:flex-row md:items-start md:justify-between gap-4">
                    <div class="min-w-0 flex-1">
                        <div class="flex flex-wrap items-center gap-2 mb-2">
                            <h2 class="text-[16px] font-semibold">{tool.name.clone()}</h2>
                            <span class="badge badge-neutral text-[12px]">{tool.tool_type.clone()}</span>
                            <span class="badge badge-neutral text-[12px]">{tool.source.clone()}</span>
                            <span class="badge badge-neutral text-[12px]">
                                {"lifecycle: "}{item.lifecycle_state.clone()}
                            </span>
                            <span class="badge badge-neutral text-[12px]">
                                {"claim: "}{item.claim_state.clone()}
                            </span>
                        </div>
                        <p class="text-[12px] text-[#999999] font-mono mb-3">{tool.slug.clone()}</p>

                        <dl class="grid gap-2 text-[13px]">
                            <div class="flex flex-wrap gap-x-4 gap-y-1">
                                <dt class="text-[#6B6B6B]">"Repo"</dt>
                                <dd class="font-mono truncate max-w-full">
                                    {tool.repo_url.clone().unwrap_or_else(|| "—".into())}
                                </dd>
                            </div>
                            <div class="flex flex-wrap gap-x-4 gap-y-1">
                                <dt class="text-[#6B6B6B]">"Homepage"</dt>
                                <dd class="font-mono truncate max-w-full">
                                    {tool.homepage.clone().unwrap_or_else(|| "—".into())}
                                </dd>
                            </div>
                            <div class="flex flex-wrap gap-x-4 gap-y-1">
                                <dt class="text-[#6B6B6B]">"npm"</dt>
                                <dd class="font-mono">{tool.npm_package.clone().unwrap_or_else(|| "—".into())}</dd>
                            </div>
                            <div class="flex flex-wrap gap-x-4 gap-y-1">
                                <dt class="text-[#6B6B6B]">"MCP endpoint"</dt>
                                <dd class="font-mono truncate max-w-full">
                                    {tool.mcp_endpoint.clone().unwrap_or_else(|| "—".into())}
                                </dd>
                            </div>
                            <div class="flex flex-wrap gap-x-4 gap-y-1">
                                <dt class="text-[#6B6B6B]">"Relevance"</dt>
                                <dd>
                                    {format!(
                                        "{} / {} — {}",
                                        tool.crypto_relevance_score,
                                        tool.relevance_status,
                                        if relevance_reasons.is_empty() { "no signals" } else { &relevance_reasons }
                                    )}
                                </dd>
                            </div>
                            <div class="flex flex-wrap gap-x-4 gap-y-1">
                                <dt class="text-[#6B6B6B]">"Install risk"</dt>
                                <dd>
                                    {format!(
                                        "{} — {}",
                                        tool.install_risk_level,
                                        if install_reasons.is_empty() { "no reasons" } else { &install_reasons }
                                    )}
                                </dd>
                            </div>
                            <div class="flex flex-wrap gap-x-4 gap-y-1">
                                <dt class="text-[#6B6B6B]">"Stars / last commit"</dt>
                                <dd>{format!("{} / {}", tool.stars, last_commit)}</dd>
                            </div>
                            {(!item.duplicate_candidates.is_empty()).then(|| {
                                let dupes = item.duplicate_candidates.clone();
                                view! {
                                    <div class="flex flex-wrap gap-x-4 gap-y-1">
                                        <dt class="text-[#6B6B6B]">"Duplicate candidates"</dt>
                                        <dd class="font-mono">
                                            {dupes
                                                .into_iter()
                                                .map(|d| format!("{} ({})", d.slug, d.name))
                                                .collect::<Vec<_>>()
                                                .join(", ")}
                                        </dd>
                                    </div>
                                }
                            })}
                        </dl>

                        {(!has_url).then(|| view! {
                            <p class="text-[12px] text-[#C0392B] mt-3">
                                "Missing trustworthy URL — add repo, homepage, npm package, or MCP endpoint before approval."
                            </p>
                        })}

                        <ReferralSettingsPanel
                            tool=tool.clone()
                            referral_stats_refresh=referral_stats_refresh
                        />
                    </div>

                    <div class="flex flex-wrap gap-2 shrink-0 max-w-[360px]">
                        <a
                            href=format!("/tools/{slug_href}")
                            class="px-3 py-1.5 text-[14px] rounded-lg border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                        >
                            "View"
                        </a>
                        {if override_needed {
                            view! {
                                <button
                                    type="button"
                                    class="px-3 py-1.5 text-[14px] rounded-lg bg-[#2D7D46] text-white hover:opacity-90 disabled:opacity-50"
                                    disabled=move || action_busy.get()
                                    on:click=move |_| {
                                        reason_modal.set(Some(ReasonModalState {
                                            slug: slug_for_override.clone(),
                                            action: "approved".into(),
                                            title: "Override and approve".into(),
                                            placeholder: "Override reason (required)".into(),
                                            confirm_label: "Confirm override approve".into(),
                                            confirm_class: "bg-[#2D7D46]".into(),
                                            override_mode: true,
                                        }));
                                    }
                                >
                                    "Override approve"
                                </button>
                            }.into_any()
                        } else {
                            view! {
                                <button
                                    type="button"
                                    class="px-3 py-1.5 text-[14px] rounded-lg bg-[#2D7D46] text-white hover:opacity-90 disabled:opacity-50"
                                    disabled=move || action_busy.get() || !has_url
                                    on:click={
                                        let run_review = run_review.clone();
                                        move |_| {
                                            run_review(ReviewToolPayload {
                                                slug: slug_for_approve.clone(),
                                                action: "approved".into(),
                                                reason: "Approved via admin review".into(),
                                                override_reason: None,
                                                expected_updated_at: None,
                                                snapshot_id: None,
                                                recommendation_id: None,
                                            });
                                        }
                                    }
                                >
                                    "Approve"
                                </button>
                            }.into_any()
                        }}
                        <button
                            type="button"
                            class="px-3 py-1.5 text-[14px] rounded-lg border border-[#C0392B] text-[#C0392B] hover:bg-[#C0392B]/5 disabled:opacity-50"
                            disabled=move || action_busy.get()
                            on:click=move |_| {
                                reason_modal.set(Some(ReasonModalState {
                                    slug: slug_for_reject.clone(),
                                    action: "rejected".into(),
                                    title: "Reject tool".into(),
                                    placeholder: "Rejection reason (required)".into(),
                                    confirm_label: "Confirm reject".into(),
                                    confirm_class: "bg-[#C0392B]".into(),
                                    override_mode: false,
                                }));
                            }
                        >
                            "Reject"
                        </button>
                        <button
                            type="button"
                            class="px-3 py-1.5 text-[14px] rounded-lg border border-[#E5E5E5] hover:bg-[#FAFAFA] disabled:opacity-50"
                            disabled=move || action_busy.get()
                            on:click=move |_| {
                                reason_modal.set(Some(ReasonModalState {
                                    slug: slug_for_needs_info.clone(),
                                    action: "needs_info".into(),
                                    title: "Request more information".into(),
                                    placeholder: "What information is needed?".into(),
                                    confirm_label: "Mark needs info".into(),
                                    confirm_class: "bg-[#1A1A1A]".into(),
                                    override_mode: false,
                                }));
                            }
                        >
                            "Needs info"
                        </button>
                        <button
                            type="button"
                            class="px-3 py-1.5 text-[14px] rounded-lg border border-[#C0392B] text-[#C0392B] hover:bg-[#C0392B]/5 disabled:opacity-50"
                            disabled=move || action_busy.get()
                            on:click=move |_| {
                                reason_modal.set(Some(ReasonModalState {
                                    slug: slug_for_quarantine.clone(),
                                    action: "quarantine".into(),
                                    title: "Quarantine listing".into(),
                                    placeholder: "Quarantine reason (required)".into(),
                                    confirm_label: "Confirm quarantine".into(),
                                    confirm_class: "bg-[#C0392B]".into(),
                                    override_mode: false,
                                }));
                            }
                        >
                            "Quarantine"
                        </button>
                        <button
                            type="button"
                            class="px-3 py-1.5 text-[14px] rounded-lg border border-[#E5E5E5] hover:bg-[#FAFAFA] disabled:opacity-50"
                            disabled=move || action_busy.get()
                            on:click=move |_| {
                                reason_modal.set(Some(ReasonModalState {
                                    slug: slug_for_verified.clone(),
                                    action: "mark_verified".into(),
                                    title: "Mark verified".into(),
                                    placeholder: "Verification evidence note (required)".into(),
                                    confirm_label: "Mark verified".into(),
                                    confirm_class: "bg-[#1A1A1A]".into(),
                                    override_mode: false,
                                }));
                            }
                        >
                            "Mark verified"
                        </button>
                        <button
                            type="button"
                            class="px-3 py-1.5 text-[14px] rounded-lg border border-[#E5E5E5] hover:bg-[#FAFAFA] disabled:opacity-50"
                            disabled=move || action_busy.get()
                            on:click=move |_| {
                                reason_modal.set(Some(ReasonModalState {
                                    slug: slug_for_official.clone(),
                                    action: "mark_official".into(),
                                    title: "Mark official".into(),
                                    placeholder: "Official evidence note (required)".into(),
                                    confirm_label: "Mark official".into(),
                                    confirm_class: "bg-[#1A1A1A]".into(),
                                    override_mode: false,
                                }));
                            }
                        >
                            "Mark official"
                        </button>
                    </div>
                </div>
            </div>
        </article>
    }
}
