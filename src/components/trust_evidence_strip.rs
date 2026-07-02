//! Compact trust evidence shown before install copy.

use crate::models::{official_link_display_label, Tool, ToolOfficialLink};
use leptos::prelude::*;

fn risk_label(risk: &str) -> &'static str {
    match risk {
        "low" => "Low",
        "medium" => "Medium",
        "high" => "High",
        "critical" => "Critical",
        _ => "Review",
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "official" => "Official",
        "verified" => "Verified",
        _ => "Community",
    }
}

fn claim_label(claim_state: &str) -> &'static str {
    match claim_state {
        "claimed" => "Claimed by team",
        "claim_pending" => "Claim pending review",
        "revoked" => "Claim revoked",
        _ => "Unclaimed",
    }
}

fn format_short_date(at: Option<chrono::DateTime<chrono::Utc>>) -> String {
    at.map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "—".into())
}

#[component]
pub fn TrustEvidenceStrip(
    tool: Tool,
    #[prop(optional)] official_links: Vec<ToolOfficialLink>,
) -> impl IntoView {
    let risk = tool.install_risk_level.clone();
    let status = tool.status.clone();
    let claim = tool.claim_state.clone();
    let last_reviewed = format_short_date(tool.last_reviewed_at);
    let last_commit = format_short_date(tool.last_commit_at);
    let has_official_links = !official_links.is_empty();

    view! {
        <section class="trust-evidence-strip" aria-label="Trust evidence">
            <div class="trust-evidence-grid">
                <div>
                    <span class="trust-summary-label">"Install risk"</span>
                    <strong>{risk_label(&risk)}</strong>
                </div>
                <div>
                    <span class="trust-summary-label">"Status"</span>
                    <strong>{status_label(&status)}</strong>
                </div>
                <div>
                    <span class="trust-summary-label">"Claim"</span>
                    <strong>{claim_label(&claim)}</strong>
                </div>
                <div>
                    <span class="trust-summary-label">"Recent activity"</span>
                    <strong>{last_commit.clone()}</strong>
                </div>
                <div>
                    <span class="trust-summary-label">"Last reviewed"</span>
                    <strong>{last_reviewed}</strong>
                </div>
            </div>
            {if has_official_links {
                view! {
                    <div class="trust-evidence-links">
                        {official_links.into_iter().map(|link| {
                            let label = official_link_display_label(&link);
                            let href = link.url.clone();
                            view! {
                                <a href=href target="_blank" rel="noopener noreferrer">{label}</a>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            } else {
                view! {
                    <p class="trust-summary-gap">"No verified official links are listed yet."</p>
                }.into_any()
            }}
        </section>
    }
}
