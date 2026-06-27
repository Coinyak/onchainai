//! Claim status timeline for submit/claim flow.

use leptos::prelude::*;

#[derive(Clone)]
pub struct ClaimStep {
    pub label: String,
    pub description: String,
}

#[component]
pub fn ClaimStatusTimeline(
    steps: Vec<ClaimStep>,
    #[prop(optional)] active_index: Option<usize>,
) -> impl IntoView {
    let active = active_index.unwrap_or(0);

    view! {
        <section class="rounded-xl border border-[#E5E5E5] bg-[#FAFAFA] p-4 mb-6">
            <h3 class="text-[14px] font-semibold mb-4">"Claim review status"</h3>
            <ol class="space-y-3">
                {steps.into_iter().enumerate().map(|(idx, step)| {
                    let is_active = idx == active;
                    let is_done = idx < active;
                    let dot_class = if is_done {
                        "bg-[#2D7D46]"
                    } else if is_active {
                        "bg-[#E76F00]"
                    } else {
                        "bg-[#E5E5E5]"
                    };
                    view! {
                        <li class="flex gap-3 items-start">
                            <span class=format!("mt-1.5 w-2.5 h-2.5 rounded-full shrink-0 {dot_class}")></span>
                            <div>
                                <p class=format!(
                                    "text-[14px] {}",
                                    if is_active { "font-semibold" } else { "font-medium" }
                                )>
                                    {step.label}
                                </p>
                                <p class="text-[12px] text-[#6B6B6B] mt-0.5">{step.description}</p>
                            </div>
                        </li>
                    }
                }).collect_view()}
            </ol>
        </section>
    }
}

pub fn default_claim_steps() -> Vec<ClaimStep> {
    vec![
        ClaimStep {
            label: "Submitted".into(),
            description: "Your claim and proof materials are on file.".into(),
        },
        ClaimStep {
            label: "Under review".into(),
            description: "Operators verify GitHub, website, and X links independently.".into(),
        },
        ClaimStep {
            label: "Needs more proof".into(),
            description: "You may be asked for backlinks or ownership evidence.".into(),
        },
        ClaimStep {
            label: "Claim approved".into(),
            description: "Verified links may show official labels on the public listing.".into(),
        },
    ]
}
