//! Operator decision panel — right sticky column of the admin workbench.

use crate::models::{Tool, ToolOfficialLink};
use crate::server::functions::{ReviewToolPayload, VerifyOfficialLinkPayload};
use crate::trust_verification::TrustVerificationResult;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;

type ReviewHandler = Arc<dyn Fn(ReviewToolPayload) + Send + Sync>;

#[derive(Clone)]
pub struct ReasonModalTrigger {
    pub slug: String,
    pub action: String,
    pub title: String,
    pub placeholder: String,
    pub confirm_label: String,
}

#[component]
pub fn AdminReviewDecisionPanel(
    tool: Tool,
    trust: TrustVerificationResult,
    links: Vec<ToolOfficialLink>,
    official_promotion_allowed: bool,
    #[allow(unused_variables)] run_review: ReviewHandler,
    on_open_reason: RwSignal<Option<ReasonModalTrigger>>,
    action_busy: RwSignal<bool>,
    on_links_updated: RwSignal<u32>,
) -> impl IntoView {
    let slug = tool.slug.clone();
    let gaps = trust.evidence_gaps.clone();
    let facts = trust.trust_facts.clone();
    let official_title = if official_promotion_allowed {
        "Strong proof present — operator approval still required"
    } else {
        "Requires claimed status and two strongly verified official links"
    };

    view! {
        <aside class="xl:sticky xl:top-4 space-y-4">
            <section class="rounded-xl border border-[#E5E5E5] bg-white p-4">
                <h3 class="text-[14px] font-semibold mb-2">{tool.name.clone()}</h3>
                <p class="text-[12px] font-mono text-[#6B6B6B] mb-3">{tool.slug.clone()}</p>
                <dl class="space-y-2 text-[13px]">
                    <div class="flex justify-between gap-2">
                        <dt class="text-[#6B6B6B]">"Status"</dt>
                        <dd class="font-medium">{tool.status.clone()}</dd>
                    </div>
                    <div class="flex justify-between gap-2">
                        <dt class="text-[#6B6B6B]">"Claim"</dt>
                        <dd class="font-medium">{tool.claim_state.clone()}</dd>
                    </div>
                    <div class="flex justify-between gap-2">
                        <dt class="text-[#6B6B6B]">"Trust score"</dt>
                        <dd class="font-medium font-mono">{trust.total_score}</dd>
                    </div>
                </dl>
            </section>

            <section class="rounded-xl border border-[#E5E5E5] bg-[#FAFAFA] p-4">
                <h3 class="text-[14px] font-semibold mb-3">"Official links"</h3>
                {if links.is_empty() {
                    view! {
                        <p class="text-[13px] text-[#6B6B6B]">"No official links recorded yet."</p>
                    }.into_any()
                } else {
                    view! {
                        <ul class="space-y-3">
                            {links.into_iter().map(|link| {
                                let link_id = link.id;
                                let link_type = link.link_type.clone();
                                let href = link.url.clone();
                                let display_url = link.url.clone();
                                let status = link.verification_status.clone();
                                let strength = link.evidence_strength.clone();
                                let refresh = on_links_updated;

                                view! {
                                    <li class="border border-[#E5E5E5] rounded-lg p-3 bg-white">
                                        <p class="text-[12px] font-medium uppercase tracking-wide text-[#6B6B6B]">
                                            {link_type.clone()}
                                        </p>
                                        <a
                                            href=href
                                            class="text-[13px] font-mono break-all text-[#1A1A1A] hover:underline"
                                            target="_blank"
                                            rel="noopener noreferrer"
                                        >
                                            {display_url}
                                        </a>
                                        <p class="text-[12px] text-[#6B6B6B] mt-1">
                                            {status.clone()} " · " {strength.clone()} " evidence"
                                        </p>
                                        <div class="flex flex-wrap gap-1 mt-2">
                                            <button
                                                type="button"
                                                class="px-2 py-1 text-[11px] rounded border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                                                on:click=move |_| {
                                                    spawn_local(async move {
                                                        let payload = VerifyOfficialLinkPayload {
                                                            link_id,
                                                            verification_status: "verified".into(),
                                                            evidence_strength: "strong".into(),
                                                            official_badge_allowed: true,
                                                            verification_method: Some("operator_review".into()),
                                                            notes: None,
                                                        };
                                                        if crate::server::functions::verify_tool_official_link(payload).await.is_ok() {
                                                            refresh.update(|n| *n = n.wrapping_add(1));
                                                        }
                                                    });
                                                }
                                            >
                                                "Verify"
                                            </button>
                                            <button
                                                type="button"
                                                class="px-2 py-1 text-[11px] rounded border border-[#C0392B]/30 text-[#C0392B] hover:bg-[#C0392B]/5"
                                                on:click=move |_| {
                                                    spawn_local(async move {
                                                        let payload = VerifyOfficialLinkPayload {
                                                            link_id,
                                                            verification_status: "rejected".into(),
                                                            evidence_strength: "weak".into(),
                                                            official_badge_allowed: false,
                                                            verification_method: None,
                                                            notes: Some("Rejected by operator".into()),
                                                        };
                                                        if crate::server::functions::verify_tool_official_link(payload).await.is_ok() {
                                                            refresh.update(|n| *n = n.wrapping_add(1));
                                                        }
                                                    });
                                                }
                                            >
                                                "Reject"
                                            </button>
                                        </div>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }}
            </section>

            <section class="rounded-xl border border-[#E5E5E5] bg-[#FAFAFA] p-4">
                <h3 class="text-[14px] font-semibold mb-3">"Trust facts"</h3>
                {if facts.is_empty() {
                    view! { <p class="text-[13px] text-[#6B6B6B]">"No positive trust facts yet."</p> }.into_any()
                } else {
                    view! {
                        <ul class="space-y-2">
                            {facts.into_iter().map(|fact| view! {
                                <li class="text-[13px]">
                                    <span class="font-medium">{fact.label}</span>
                                    <span class="text-[#6B6B6B]">" — " {fact.detail}</span>
                                </li>
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }}
            </section>

            <section class="rounded-xl border border-[#E5E5E5] bg-white p-4">
                <h3 class="text-[14px] font-semibold mb-2">"Next proof needed"</h3>
                {if gaps.is_empty() {
                    view! { <p class="text-[13px] text-[#2D7D46]">"No critical gaps flagged."</p> }.into_any()
                } else {
                    view! {
                        <ul class="space-y-1">
                            {gaps.into_iter().map(|gap| view! {
                                <li class="text-[13px] text-[#6B6B6B]">"· " {gap}</li>
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }}
                <p class="text-[12px] text-[#6B6B6B] mt-2">
                    "Suggested: " {trust.suggested_action.clone()}
                </p>
            </section>

            <section class="rounded-xl border border-[#E5E5E5] bg-white p-4">
                <h3 class="text-[14px] font-semibold mb-3">"Actions"</h3>
                <div class="flex flex-col gap-2">
                    <ActionButton
                        label="Approve community"
                        class="bg-[#2D7D46] text-white"
                        slug=slug.clone()
                        action="approved"
                        title="Approve for community listing"
                        placeholder="Approval reason"
                        confirm="Confirm approve"
                        on_open_reason=on_open_reason
                        action_busy=action_busy
                    />
                    <ActionButton
                        label="Request claim proof"
                        class="border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                        slug=slug.clone()
                        action="needs_info"
                        title="Request claim proof"
                        placeholder="What proof is needed?"
                        confirm="Send request"
                        on_open_reason=on_open_reason
                        action_busy=action_busy
                    />
                    <ActionButton
                        label="Mark verified"
                        class="border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                        slug=slug.clone()
                        action="mark_verified"
                        title="Mark verified"
                        placeholder="Verification evidence"
                        confirm="Mark verified"
                        on_open_reason=on_open_reason
                        action_busy=action_busy
                    />
                    {
                        let slug_official = slug.clone();
                        view! {
                    <button
                        type="button"
                        class="px-3 py-2 text-[13px] rounded-lg border border-[#E5E5E5] hover:bg-[#FAFAFA] disabled:opacity-40 disabled:cursor-not-allowed"
                        disabled=move || action_busy.get() || !official_promotion_allowed
                        title=official_title
                        on:click=move |_| {
                            on_open_reason.set(Some(ReasonModalTrigger {
                                slug: slug_official.clone(),
                                action: "mark_official".into(),
                                title: "Mark official".into(),
                                placeholder: "Official evidence note (required)".into(),
                                confirm_label: "Mark official".into(),

                            }));
                        }
                    >
                        "Mark official"
                    </button>
                        }.into_any()
                    }
                    <ActionButton
                        label="Quarantine"
                        class="border border-[#C0392B] text-[#C0392B] hover:bg-[#C0392B]/5"
                        slug=slug.clone()
                        action="quarantine"
                        title="Quarantine listing"
                        placeholder="Quarantine reason"
                        confirm="Confirm quarantine"
                        on_open_reason=on_open_reason
                        action_busy=action_busy
                    />
                </div>
            </section>
        </aside>
    }
}

#[component]
fn ActionButton(
    label: &'static str,
    class: &'static str,
    slug: String,
    action: &'static str,
    title: &'static str,
    placeholder: &'static str,
    confirm: &'static str,
    on_open_reason: RwSignal<Option<ReasonModalTrigger>>,
    action_busy: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=format!("px-3 py-2 text-[13px] rounded-lg disabled:opacity-50 {class}")
            disabled=move || action_busy.get()
            on:click=move |_| {
                on_open_reason.set(Some(ReasonModalTrigger {
                    slug: slug.clone(),
                    action: action.into(),
                    title: title.into(),
                    placeholder: placeholder.into(),
                    confirm_label: confirm.into(),
                }));
            }
        >
            {label}
        </button>
    }
}
