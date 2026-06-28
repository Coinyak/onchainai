//! Admin tool review workbench — queue rail, timeline, decision panel.

use crate::components::admin_review_decision_panel::{
    AdminReviewDecisionPanel, ReasonModalTrigger,
};
use crate::components::admin_review_timeline::AdminReviewTimeline;
use crate::pages::admin::admin_page_shell;
use crate::server::functions::{
    get_admin_tool_workbench, get_admin_workbench_summary, get_referral_dashboard_stats,
    list_review_queue, review_tool, AdminToolWorkbenchView, ReferralDashboardStats,
    ReviewQueueItem, ReviewToolPayload,
};
use crate::workbench::derive_selected_slug;
use crate::workbench::WorkbenchSummaryCard;
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
            if crate::server::functions::REVIEW_QUEUES.contains(&queue.as_str()) {
                active_queue.set(queue);
            }
        }
    });

    let refresh = RwSignal::new(0u32);
    let workbench_refresh = RwSignal::new(0u32);
    let referral_stats_refresh = RwSignal::new(0u32);

    let summary = Resource::new(
        move || (refresh.get(), workbench_refresh.get()),
        |_| async move { get_admin_workbench_summary().await },
    );
    let queue_items = Resource::new(
        move || (active_queue.get(), refresh.get()),
        |(queue, _)| async move { list_review_queue(queue, 50).await },
    );
    let referral_stats = Resource::new(
        move || referral_stats_refresh.get(),
        |_| async move { get_referral_dashboard_stats().await },
    );

    let selected_slug = Memo::new(move |_| {
        let selected_param = query.get().get("selected").map(|s| s.to_string());
        let slugs: Vec<String> = queue_items
            .get()
            .and_then(|res| res.ok())
            .map(|items| items.into_iter().map(|i| i.tool.slug).collect())
            .unwrap_or_default();
        derive_selected_slug(selected_param.as_deref(), &slugs)
    });

    let workbench = Resource::new(
        move || (selected_slug.get(), workbench_refresh.get()),
        |(slug, _)| async move {
            match slug {
                Some(s) => get_admin_tool_workbench(s).await.map(Some),
                None => Ok(None),
            }
        },
    );

    let reason_modal = RwSignal::new(None::<ReasonModalState>);
    let reason_trigger = RwSignal::new(None::<ReasonModalTrigger>);
    Effect::new(move |_| {
        if let Some(trigger) = reason_trigger.get() {
            reason_modal.set(Some(ReasonModalState {
                slug: trigger.slug,
                action: trigger.action,
                title: trigger.title,
                placeholder: trigger.placeholder,
                confirm_label: trigger.confirm_label,
                confirm_class: "bg-[#1A1A1A]".into(),
            }));
        }
    });
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
                    workbench_refresh.update(|n| *n = n.wrapping_add(1));
                }
                Err(e) => action_error.set(Some(e.to_string())),
            }
        });
    });

    let run_review_panel = run_review.clone();
    let run_review_modal = run_review.clone();

    view! {
        <div class="px-4 md:px-6 py-8 max-w-[1400px] mx-auto">
            <div class="mb-6">
                <h1 class="text-[20px] font-semibold tracking-tight">"Review Workbench"</h1>
                <p class="text-[#6B6B6B] text-[14px] mt-1">
                    "One candidate at a time — read the timeline, verify links, then decide."
                </p>
            </div>

            <Suspense fallback=|| view! { <SummaryRailSkeleton/> }>
                {move || summary.get().map(|res| match res {
                    Ok(data) => view! { <SummaryRail cards=data.cards/> }.into_any(),
                    Err(e) => view! {
                        <p class="mb-4 text-[14px] text-[#C0392B]">{e.to_string()}</p>
                    }.into_any(),
                })}
            </Suspense>

            <Suspense fallback=|| view! {
                <div class="mb-6 grid grid-cols-2 md:grid-cols-4 gap-3">
                    <ReferralStatSkeleton/>
                    <ReferralStatSkeleton/>
                    <ReferralStatSkeleton/>
                    <ReferralStatSkeleton/>
                </div>
            }>
                {move || referral_stats.get().map(|res| match res {
                    Ok(stats) => view! { <ReferralStatsBar stats=stats/> }.into_any(),
                    Err(_) => ().into_any(),
                })}
            </Suspense>

            {move || action_error.get().map(|msg| view! {
                <p class="mb-4 text-[14px] text-[#C0392B] border border-[#C0392B]/30 rounded-lg px-4 py-2 bg-[#C0392B]/5">
                    {msg}
                </p>
            })}

            <div class="grid grid-cols-1 xl:grid-cols-[220px_minmax(0,1fr)_320px] gap-4">
                <QueueRail
                    tabs=QUEUE_TABS
                    active_queue=active_queue
                    queue_items=queue_items
                    selected_slug=selected_slug
                />

                <div class="min-w-0 space-y-4">
                    <Suspense fallback=|| view! {
                        <p class="text-[14px] text-[#6B6B6B]">"Loading review timeline..."</p>
                    }>
                        {move || workbench.get().map(|res| match res {
                            Ok(Some(view)) => view! {
                                <SelectedToolHeader view=view.clone()/>
                                <AdminReviewTimeline
                                    entries=view.timeline.clone()
                                    verdicts=view.verdicts.clone()
                                />
                            }.into_any(),
                            Ok(None) => view! {
                                <div class="rounded-xl border border-[#E5E5E5] bg-[#FAFAFA] px-6 py-16 text-center">
                                    <p class="text-[16px] font-medium mb-2">"Select a candidate"</p>
                                    <p class="text-[#6B6B6B] text-[14px]">
                                        "Choose a tool from the queue rail to review."
                                    </p>
                                </div>
                            }.into_any(),
                            Err(e) => view! {
                                <p class="text-[14px] text-[#C0392B]">{e.to_string()}</p>
                            }.into_any(),
                        })}
                    </Suspense>
                </div>

                <Suspense fallback=|| view! {
                    <aside class="rounded-xl border border-[#E5E5E5] bg-[#FAFAFA] p-4 h-48 animate-pulse"/>
                }>
                    {move || workbench.get().map(|res| match res {
                        Ok(Some(view)) => view! {
                            <AdminReviewDecisionPanel
                                tool=view.tool.clone()
                                trust=view.trust.clone()
                                links=view.official_links.clone()
                                official_promotion_allowed=view.official_promotion_allowed
                                run_review=run_review_panel.clone()
                                on_open_reason=reason_trigger
                                action_busy=action_busy
                                on_links_updated=workbench_refresh
                            />
                        }.into_any(),
                        _ => ().into_any(),
                    })}
                </Suspense>
            </div>

            {move || reason_modal.get().map(|state| view! {
                <ReasonModal
                    state=state
                    run_review=run_review_modal.clone()
                    reason_modal=reason_modal
                    action_busy=action_busy
                />
            })}
        </div>
    }
}

#[component]
fn SummaryRail(cards: Vec<WorkbenchSummaryCard>) -> impl IntoView {
    view! {
        <div class="mb-6 grid grid-cols-2 md:grid-cols-4 gap-3">
            {cards.into_iter().map(|card| {
                let href = card.queue.as_ref().map(|q| format!("/admin/tools?queue={q}"));
                view! {
                    <div class="rounded-lg border border-[#E5E5E5] bg-white px-4 py-3">
                        <p class="text-[12px] text-[#6B6B6B]">{card.label.clone()}</p>
                        <p class="text-[20px] font-semibold mt-1">{card.count}</p>
                        {href.map(|h| view! {
                            <a href=h class="text-[11px] text-[#6B6B6B] hover:underline mt-1 inline-block">
                                "View queue"
                            </a>
                        })}
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

#[component]
fn SummaryRailSkeleton() -> impl IntoView {
    view! {
        <div class="mb-6 grid grid-cols-2 md:grid-cols-4 gap-3">
            {(0..4).map(|_| view! {
                <div class="rounded-lg border border-[#E5E5E5] bg-[#FAFAFA] px-4 py-3 h-[72px]"/>
            }).collect_view()}
        </div>
    }
}

#[component]
fn QueueRail(
    tabs: &'static [QueueTab],
    active_queue: RwSignal<String>,
    queue_items: Resource<Result<Vec<ReviewQueueItem>, ServerFnError>>,
    selected_slug: Memo<Option<String>>,
) -> impl IntoView {
    view! {
        <nav class="rounded-xl border border-[#E5E5E5] bg-white p-3 space-y-4">
            <div>
                <p class="text-[12px] font-medium text-[#6B6B6B] uppercase tracking-wide mb-2">"Queues"</p>
                <ul class="space-y-1">
                    {tabs.iter().map(|tab| {
                        let tab_id = tab.id;
                        let href = format!("/admin/tools?queue={tab_id}");
                        view! {
                            <li>
                                <a
                                    href=href
                                    class=move || {
                                        if active_queue.get() == tab_id {
                                            "block px-2 py-1.5 rounded-lg text-[13px] font-medium bg-[#1A1A1A] text-white no-underline"
                                        } else {
                                            "block px-2 py-1.5 rounded-lg text-[13px] font-medium hover:bg-[#FAFAFA] no-underline text-inherit"
                                        }
                                    }
                                >
                                    {tab.label}
                                </a>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            </div>

            <div>
                <p class="text-[12px] font-medium text-[#6B6B6B] uppercase tracking-wide mb-2">"In queue"</p>
                <Suspense fallback=|| view! {
                    <p class="text-[13px] text-[#6B6B6B]">"Loading..."</p>
                }>
                    {move || queue_items.get().map(|res| match res {
                        Ok(items) if items.is_empty() => view! {
                            <p class="text-[13px] text-[#6B6B6B]">"Empty"</p>
                        }.into_any(),
                        Ok(items) => view! {
                            <ul class="space-y-1 max-h-[420px] overflow-y-auto">
                                {items.into_iter().map(|item| {
                                    let slug = item.tool.slug.clone();
                                    let name = item.tool.name.clone();
                                    let queue = active_queue.get_untracked();
                                    let href = format!("/admin/tools?queue={queue}&selected={slug}");
                                    let is_selected = selected_slug.get().as_deref() == Some(slug.as_str());
                                    view! {
                                        <li>
                                            <a
                                                href=href
                                                class=if is_selected {
                                                    "block px-2 py-2 rounded-lg text-[13px] bg-[#FAFAFA] border border-[#E5E5E5] no-underline text-inherit"
                                                } else {
                                                    "block px-2 py-2 rounded-lg text-[13px] hover:bg-[#FAFAFA] no-underline text-inherit"
                                                }
                                            >
                                                <span class="font-medium block truncate">{name}</span>
                                                <span class="text-[11px] text-[#6B6B6B] font-mono">{slug}</span>
                                            </a>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any(),
                        Err(e) => view! {
                            <p class="text-[13px] text-[#C0392B]">{e.to_string()}</p>
                        }.into_any(),
                    })}
                </Suspense>
            </div>
        </nav>
    }
}

#[component]
fn SelectedToolHeader(view: AdminToolWorkbenchView) -> impl IntoView {
    let tool = view.tool.clone();
    view! {
        <header class="rounded-xl border border-[#E5E5E5] bg-white px-4 py-3">
            <div class="flex flex-wrap items-center gap-2">
                <h2 class="text-[16px] font-semibold">{tool.name.clone()}</h2>
                <span class="badge badge-neutral text-[12px]">{tool.tool_type.clone()}</span>
                <span class="badge badge-neutral text-[12px]">{"claim: "}{tool.claim_state.clone()}</span>
            </div>
            <p class="text-[12px] font-mono text-[#6B6B6B] mt-1">{tool.slug.clone()}</p>
            <a
                href=format!("/tools/{}", tool.slug)
                class="text-[13px] text-[#6B6B6B] hover:underline mt-2 inline-block"
                target="_blank"
            >
                "View public listing"
            </a>
        </header>
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
        <div class="modal-overlay">
            <div class="w-full max-w-md rounded-xl bg-white border border-[#E5E5E5] shadow-lg p-5" role="dialog">
                <h3 class="text-[16px] font-semibold mb-2">{state.title.clone()}</h3>
                <p class="text-[14px] text-[#6B6B6B] mb-4 font-mono">{slug.clone()}</p>
                <textarea
                    class="w-full min-h-[96px] rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px]"
                    placeholder=state.placeholder.clone()
                    prop:value=move || reason.get()
                    on:input=move |ev| reason.set(event_target_value(&ev))
                />
                <div class="flex justify-end gap-2 mt-4">
                    <button
                        type="button"
                        class="px-3 py-1.5 text-[14px] rounded-lg border border-[#E5E5E5]"
                        on:click=move |_| reason_modal.set(None)
                    >
                        "Cancel"
                    </button>
                    <button
                        type="button"
                        class=format!("px-3 py-1.5 text-[14px] rounded-lg text-white {}", state.confirm_class)
                        disabled=move || action_busy.get() || reason.get().trim().is_empty()
                        on:click=move |_| {
                            let text = reason.get().trim().to_string();
                            if text.is_empty() { return; }
                            let payload = ReviewToolPayload {
                                slug: slug.clone(),
                                action: action.clone(),
                                reason: text,
                                override_reason: None,
                                expected_updated_at: None,
                                snapshot_id: None,
                                recommendation_id: None,
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
