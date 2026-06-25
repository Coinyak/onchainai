//! Stripe-style tool card for list views.

use crate::models::Tool;
use leptos::prelude::*;
use leptos_router::components::A;

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

fn type_badge_class(tool_type: &str) -> &'static str {
    match tool_type {
        "x402" => "badge badge-x402",
        _ => "badge badge-neutral",
    }
}

#[component]
pub fn ToolCard(
    tool: Tool,
    #[prop(optional)] preview_href: Option<String>,
) -> impl IntoView {
    let slug = tool.slug.clone();
    let detail_href = format!("/tools/{slug}");
    let href = preview_href.unwrap_or(detail_href);
    let mono = monogram(&tool.name);
    let status = tool.status.clone();
    let tool_type = tool.tool_type.clone();
    let chains = tool.chains.clone();
    let chain_preview: Vec<_> = chains.iter().take(5).cloned().collect();
    let extra_chains = chains.len().saturating_sub(5);
    let install = tool.install_command.clone().unwrap_or_default();
    let stars = tool.stars;
    let description = tool
        .description
        .clone()
        .unwrap_or_else(|| "No description.".into());

    view! {
        <article class="tool-card">
            <A href=href attr:class="tool-card-link no-underline text-inherit">
                <div class="tool-card-inner">
                    <div class="tool-logo" aria-hidden="true">
                        {mono}
                    </div>
                    <div class="tool-card-body">
                        <div class="tool-card-header">
                            <h3 class="tool-name">{tool.name.clone()}</h3>
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
                                <span class=type_badge_class(&tool_type)>{tool_type.to_uppercase()}</span>
                            </div>
                        </div>
                        <p class="tool-desc">{description}</p>
                        <div class="tool-meta">
                            <span class="tool-chains">
                                {chain_preview
                                    .into_iter()
                                    .map(|c| view! { <span class="chain-pill">{c}</span> })
                                    .collect_view()}
                                {if extra_chains > 0 {
                                    view! { <span class="chain-pill chain-more">{"+"}{extra_chains}</span> }
                                        .into_any()
                                } else {
                                    ().into_any()
                                }}
                            </span>
                            <span class="tool-stars">{"★ "}{stars}</span>
                        </div>
                        {if !install.is_empty() {
                            view! {
                                <div class="tool-install hidden md:flex">
                                    <code class="install-cmd">{install.clone()}</code>
                                    <button
                                        type="button"
                                        class="copy-btn"
                                        data-copy=install
                                    >
                                        "Copy"
                                    </button>
                                </div>
                            }
                            .into_any()
                        } else {
                            ().into_any()
                        }}
                    </div>
                </div>
            </A>
        </article>
    }
}