//! Shared tool detail body used by detail page, preview panel, and bottom sheet.

use crate::models::Tool;
use leptos::prelude::*;

fn monogram(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

fn badge_class(status: &str) -> &'static str {
    match status {
        "verified" => "badge badge-verified",
        "official" => "badge badge-official",
        _ => "badge badge-neutral",
    }
}

#[component]
pub fn ToolDetailContent(
    tool: Tool,
    #[prop(optional)] compact: bool,
    #[prop(optional)] full_page_href: Option<String>,
) -> impl IntoView {
    let install = tool.install_command.clone().unwrap_or_default();
    let desc = tool
        .description
        .clone()
        .unwrap_or_else(|| "No description.".into());
    let mono = monogram(&tool.name);
    let status = tool.status.clone();
    let tool_type = tool.tool_type.clone();

    view! {
        <div class=if compact { "detail-content compact" } else { "detail-content" }>
            <header class="detail-header">
                <div class="detail-logo" aria-hidden="true">{mono}</div>
                <div class="detail-header-text">
                    <h2 class="detail-title">{tool.name.clone()}</h2>
                    <div class="tool-badges">
                        <span class=badge_class(&status)>
                            {if status == "verified" {
                                "Verified"
                            } else if status == "official" {
                                "Official"
                            } else {
                                "Community"
                            }}
                        </span>
                        <span class="badge badge-neutral">{tool_type.to_uppercase()}</span>
                    </div>
                    <div class="detail-tags">
                        <span class="tag-pill">{tool.function.clone()}</span>
                        <span class="tag-pill">{tool.asset_class.clone()}</span>
                        <span class="tag-pill">{tool.actor.clone()}</span>
                    </div>
                </div>
            </header>
            <p class="detail-desc">{desc}</p>
            <div class="detail-meta">
                <span>{"★ "}{tool.stars}</span>
                {if !tool.chains.is_empty() {
                    view! {
                        <span class="tool-chains">
                            {tool.chains
                                .iter()
                                .map(|c| view! { <span class="chain-pill">{c.clone()}</span> })
                                .collect_view()}
                        </span>
                    }
                    .into_any()
                } else {
                    ().into_any()
                }}
            </div>
            {if !install.is_empty() {
                view! {
                    <section class="install-section">
                        <h3 class="install-heading">"Install"</h3>
                        <div class="install-tabs">
                            <span class="install-tab active">"Generic"</span>
                        </div>
                        <div class="tool-install">
                            <code class="install-cmd">{install.clone()}</code>
                            <button type="button" class="copy-btn" data-copy=install>
                                "Copy"
                            </button>
                        </div>
                    </section>
                }
                .into_any()
            } else {
                ().into_any()
            }}
            <section class="trust-section">
                <h3 class="install-heading">"Trust"</h3>
                <ul class="trust-list">
                    <li>"Source: "{tool.source.clone()}</li>
                    {if tool.repo_url.is_some() {
                        view! { <li>"Repository linked"</li> }.into_any()
                    } else {
                        ().into_any()
                    }}
                    {if tool.status == "verified" || tool.status == "official" {
                        view! { <li>"Badge verified by OnchainAI"</li> }.into_any()
                    } else {
                        ().into_any()
                    }}
                </ul>
            </section>
            {if let Some(href) = full_page_href {
                view! {
                    <a href=href class="full-page-link">"View full page"</a>
                }
                .into_any()
            } else {
                ().into_any()
            }}
        </div>
    }
}