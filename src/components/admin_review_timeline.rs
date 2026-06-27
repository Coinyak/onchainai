//! Operator review timeline — center column of the admin workbench.

use crate::models::{OperatorVerdict, ReviewEntry};
use leptos::prelude::*;

#[component]
pub fn AdminReviewTimeline(
    entries: Vec<ReviewEntry>,
    verdicts: Vec<OperatorVerdict>,
) -> impl IntoView {
    let has_content = !entries.is_empty() || !verdicts.is_empty();

    view! {
        <section class="rounded-xl border border-[#E5E5E5] bg-white p-5 min-h-[320px]">
            <h2 class="text-[16px] font-semibold mb-1">"Review timeline"</h2>
            <p class="text-[12px] text-[#6B6B6B] mb-4">
                "Agent recommendations and operator notes — human verdict is final."
            </p>

            {if !has_content {
                view! {
                    <p class="text-[14px] text-[#6B6B6B] py-8 text-center">
                        "No review entries yet. External agents can log runs via the operator harness."
                    </p>
                }.into_any()
            } else {
                view! {
                    <div class="space-y-4">
                        {verdicts.into_iter().map(|verdict| {
                            let action = verdict.action.clone();
                            let note = verdict.note.clone().unwrap_or_else(|| "No note".into());
                            let created = verdict.created_at.format("%Y-%m-%d %H:%M").to_string();
                            view! {
                                <article class="border-l-2 border-[#2D7D46] pl-4 py-1">
                                    <p class="text-[12px] text-[#6B6B6B]">
                                        "operator verdict · " {created}
                                    </p>
                                    <p class="text-[14px] font-medium mt-1">{action}</p>
                                    <p class="text-[14px] text-[#6B6B6B] mt-1">{note}</p>
                                </article>
                            }
                        }).collect_view()}

                        {entries.into_iter().map(|entry| {
                            let role = entry.role.clone();
                            let agent = entry.agent_label.clone().unwrap_or_else(|| "system".into());
                            let rationale = entry.rationale.clone().unwrap_or_else(|| "No rationale".into());
                            let action = entry.recommended_action.clone().unwrap_or_else(|| "—".into());
                            let created = entry.created_at.format("%Y-%m-%d %H:%M").to_string();
                            let confidence = entry.confidence.map(|c| format!("{:.0}%", c * 100.0));
                            let missing: Vec<String> = entry.missing_proofs_json
                                .as_array()
                                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                                .unwrap_or_default();

                            view! {
                                <article class="border-l-2 border-[#E5E5E5] pl-4 py-1">
                                    <p class="text-[12px] text-[#6B6B6B]">
                                        {role} " · " {agent} " · " {created}
                                    </p>
                                    <p class="text-[14px] mt-1">{rationale}</p>
                                    <p class="text-[13px] text-[#6B6B6B] mt-1">
                                        "Recommended: " <span class="font-medium">{action}</span>
                                        {confidence.map(|c| view! {
                                            <span class="ml-2">"(" {c} " confidence)"</span>
                                        })}
                                    </p>
                                    {(!missing.is_empty()).then(|| view! {
                                        <p class="text-[12px] text-[#6B6B6B] mt-1">
                                            "Missing proof: " {missing.join(", ")}
                                        </p>
                                    })}
                                </article>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </section>
    }
}
