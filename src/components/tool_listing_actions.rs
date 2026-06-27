//! Report listing and claim request UI on tool detail pages.

use crate::components::login_modal::LoginModal;
use crate::models::{Tool, TOOL_REPORT_REASONS};
use crate::server::functions::{
    get_current_user, report_tool, request_tool_claim, ClaimToolInput, ReportToolInput,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn ToolListingActions(tool: Tool) -> impl IntoView {
    let show_login = RwSignal::new(false);
    let show_report = RwSignal::new(false);
    let show_claim = RwSignal::new(false);
    let action_msg = RwSignal::new(None::<String>);
    let action_err = RwSignal::new(None::<String>);
    let report_busy = RwSignal::new(false);
    let claim_busy = RwSignal::new(false);

    let report_reason = RwSignal::new("scam_phishing".to_string());
    let report_details = RwSignal::new(String::new());
    let claim_note = RwSignal::new(String::new());
    let claim_email = RwSignal::new(String::new());

    let slug_report = tool.slug.clone();
    let slug_claim = tool.slug.clone();
    let claim_state = tool.claim_state.clone();

    view! {
        <LoginModal show=show_login/>
        <section id="listing-actions" class="listing-actions mt-8 border-t border-[#E5E5E5] pt-6">
            <h3 class="text-[16px] font-semibold mb-2">"Listing actions"</h3>
            <p class="text-[13px] text-[#6B6B6B] mb-4 leading-relaxed">
                "This is an unofficial community listing until a project claims or verifies it. "
                "Report issues or request ownership verification."
            </p>
            <div class="flex flex-wrap gap-2">
                <button
                    type="button"
                    class="h-9 px-3 rounded-lg border border-[#E5E5E5] text-[13px] bg-white hover:bg-[#FAFAFA]"
                    on:click=move |_| {
                        action_err.set(None);
                        action_msg.set(None);
                        show_report.set(true);
                    }
                >
                    "Report listing"
                </button>
                {if claim_state == "unclaimed" || claim_state == "revoked" {
                    view! {
                        <button
                            type="button"
                            class="h-9 px-3 rounded-lg border border-[#E5E5E5] text-[13px] bg-white hover:bg-[#FAFAFA]"
                            on:click=move |_| {
                                action_err.set(None);
                                action_msg.set(None);
                                show_claim.set(true);
                            }
                        >
                            "Claim this listing"
                        </button>
                    }.into_any()
                } else if claim_state == "claim_pending" {
                    view! {
                        <span class="text-[13px] text-[#6B6B6B] px-2 py-1">"Claim pending review"</span>
                    }.into_any()
                } else if claim_state == "claimed" {
                    view! {
                        <span class="text-[13px] text-[#1A7F4B] px-2 py-1">"Claimed by project"</span>
                    }.into_any()
                } else {
                    view! {
                        <span class="text-[13px] text-[#6B6B6B] px-2 py-1">{"Claim state: "}{claim_state.clone()}</span>
                    }.into_any()
                }}
            </div>
            {move || action_msg.get().map(|m| view! {
                <p class="text-[13px] text-[#1A7F4B] mt-3" role="status">{m}</p>
            })}
            {move || action_err.get().map(|m| view! {
                <p class="text-[13px] text-[#C0392B] mt-3" role="alert">{m}</p>
            })}
        </section>

        {move || show_report.get().then(|| {
            let slug_report = slug_report.clone();
            view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40" on:click=move |_| show_report.set(false)>
                    <div
                        class="w-full max-w-md rounded-xl bg-white border border-[#E5E5E5] shadow-lg p-6"
                        role="dialog"
                        aria-labelledby="report-title"
                        on:click=|ev| ev.stop_propagation()
                    >
                        <h4 id="report-title" class="text-[16px] font-semibold mb-3">"Report listing"</h4>
                        <label class="block text-[13px] font-medium mb-1">"Reason"</label>
                        <select
                            class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[13px] bg-white mb-3"
                            prop:value=move || report_reason.get()
                            on:change=move |ev| report_reason.set(event_target_value(&ev))
                        >
                            {TOOL_REPORT_REASONS.iter().map(|(value, label)| view! {
                                <option value=*value>{*label}</option>
                            }).collect_view()}
                        </select>
                        <label class="block text-[13px] font-medium mb-1">"Details (optional)"</label>
                        <textarea
                            class="w-full min-h-[80px] px-3 py-2 border border-[#E5E5E5] rounded-lg text-[13px] mb-4"
                            placeholder="What is wrong with this listing?"
                            prop:value=move || report_details.get()
                            on:input=move |ev| report_details.set(event_target_value(&ev))
                        />
                        <div class="flex gap-2 justify-end">
                            <button type="button" class="h-9 px-3 rounded-lg border border-[#E5E5E5] text-[13px]" on:click=move |_| show_report.set(false)>"Cancel"</button>
                            <button
                                type="button"
                                class="h-9 px-3 rounded-lg bg-[#E76F00] text-white text-[13px] disabled:opacity-50"
                                disabled=move || report_busy.get()
                                on:click=move |_| {
                                    report_busy.set(true);
                                    let slug_report = slug_report.clone();
                                    let reason = report_reason.get_untracked();
                                    let details = report_details.get_untracked();
                                    spawn_local(async move {
                                        match get_current_user().await {
                                            Ok(Some(_)) => {
                                                let input = ReportToolInput {
                                                    slug: slug_report,
                                                    reason,
                                                    details: if details.trim().is_empty() { None } else { Some(details) },
                                                };
                                                match report_tool(input).await {
                                                    Ok(_) => {
                                                        action_msg.set(Some("Report submitted. Operators will review.".into()));
                                                        action_err.set(None);
                                                        show_report.set(false);
                                                    }
                                                    Err(e) => action_err.set(Some(e.to_string())),
                                                }
                                            }
                                            Ok(None) => show_login.set(true),
                                            Err(e) => action_err.set(Some(e.to_string())),
                                        }
                                        report_busy.set(false);
                                    });
                                }
                            >
                                "Submit report"
                            </button>
                        </div>
                    </div>
                </div>
            }
        })}

        {move || show_claim.get().then(|| {
            let slug_claim = slug_claim.clone();
            view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40" on:click=move |_| show_claim.set(false)>
                    <div
                        class="w-full max-w-md rounded-xl bg-white border border-[#E5E5E5] shadow-lg p-6"
                        role="dialog"
                        aria-labelledby="claim-title"
                        on:click=|ev| ev.stop_propagation()
                    >
                        <h4 id="claim-title" class="text-[16px] font-semibold mb-2">"Claim listing"</h4>
                        <p class="text-[13px] text-[#6B6B6B] mb-4 leading-relaxed">
                            "Request verification as the official project. Operators review before claim is approved."
                        </p>
                        <label class="block text-[13px] font-medium mb-1">"Verification note"</label>
                        <textarea
                            class="w-full min-h-[100px] px-3 py-2 border border-[#E5E5E5] rounded-lg text-[13px] mb-3"
                            placeholder="How can operators verify you represent this project?"
                            prop:value=move || claim_note.get()
                            on:input=move |ev| claim_note.set(event_target_value(&ev))
                        />
                        <label class="block text-[13px] font-medium mb-1">"Contact email (optional)"</label>
                        <input
                            type="email"
                            class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[13px] mb-4"
                            prop:value=move || claim_email.get()
                            on:input=move |ev| claim_email.set(event_target_value(&ev))
                        />
                        <div class="flex gap-2 justify-end">
                            <button type="button" class="h-9 px-3 rounded-lg border border-[#E5E5E5] text-[13px]" on:click=move |_| show_claim.set(false)>"Cancel"</button>
                            <button
                                type="button"
                                class="h-9 px-3 rounded-lg bg-[#E76F00] text-white text-[13px] disabled:opacity-50"
                                disabled=move || claim_busy.get()
                                on:click=move |_| {
                                    claim_busy.set(true);
                                    let slug_claim = slug_claim.clone();
                                    let note = claim_note.get_untracked();
                                    let email = claim_email.get_untracked();
                                    spawn_local(async move {
                                        match get_current_user().await {
                                            Ok(Some(_)) => {
                                                let input = ClaimToolInput {
                                                    slug: slug_claim,
                                                    verification_note: note,
                                                    contact_email: if email.trim().is_empty() { None } else { Some(email) },
                                                    team_name: None,
                                                    github_url: None,
                                                    website_url: None,
                                                    x_url: None,
                                                    proof_links: vec![],
                                                };
                                                match request_tool_claim(input).await {
                                                    Ok(_) => {
                                                        action_msg.set(Some("Claim request submitted (claim_pending).".into()));
                                                        action_err.set(None);
                                                        show_claim.set(false);
                                                    }
                                                    Err(e) => action_err.set(Some(e.to_string())),
                                                }
                                            }
                                            Ok(None) => show_login.set(true),
                                            Err(e) => action_err.set(Some(e.to_string())),
                                        }
                                        claim_busy.set(false);
                                    });
                                }
                            >
                                "Request claim"
                            </button>
                        </div>
                    </div>
                </div>
            }
        })}
    }
}
