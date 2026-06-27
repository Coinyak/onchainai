//! Public trust facts — explainable labels without raw scores.

use crate::trust_verification::TrustFact;
use leptos::prelude::*;

#[component]
pub fn ToolTrustFacts(facts: Vec<TrustFact>) -> impl IntoView {
    let rendered = facts.clone();
    view! {
        {(!rendered.is_empty()).then(|| {
            let items = rendered.clone();
            view! {
                <section class="rounded-xl border border-[#E5E5E5] bg-[#FAFAFA] p-4 mt-6">
                    <h3 class="text-[14px] font-semibold mb-3">"Why this looks trustworthy"</h3>
                    <ul class="space-y-2">
                        {items.into_iter().map(|fact| view! {
                            <li class="text-[14px]">
                                <span class="font-medium">{fact.label.clone()}</span>
                                <span class="text-[#6B6B6B]">" — " {fact.detail.clone()}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                </section>
            }
        })}
    }
}
