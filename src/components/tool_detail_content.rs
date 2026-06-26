//! Shared tool detail body — install tabs, trust, chains, links.

use crate::components::copy_button::CopyButton;
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

fn claude_config(install: &str) -> String {
    format!(
        "{{\"mcpServers\":{{\"tool\":{{\"command\":\"sh\",\"args\":[\"-c\",\"{install}\"]}}}}}}"
    )
}

fn cursor_config(install: &str) -> String {
    format!("// Add to Cursor MCP settings:\n{install}")
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
    let active_tab = RwSignal::new("generic".to_string());

    let claude = claude_config(&install);
    let cursor = cursor_config(&install);
    let last_commit = tool
        .last_commit_at
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "—".into());

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
                        {if tool.pricing == "x402" {
                            view! { <span class="badge badge-x402">"x402"</span> }.into_any()
                        } else {
                            ().into_any()
                        }}
                    </div>
                    <div class="detail-tags">
                        <span class="tag-pill">{tool.function.clone()}</span>
                        <span class="tag-pill">{tool.asset_class.clone()}</span>
                        <span class="tag-pill">{tool.actor.clone()}</span>
                    </div>
                </div>
            </header>
            <p class="detail-desc">{desc}</p>
            <div class="detail-meta detail-meta-wrap">
                <span>{"★ "}{tool.stars}</span>
                {if !tool.chains.is_empty() {
                    view! {
                        <span class="tool-chains chains-wrap">
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
                            <button
                                type="button"
                                class=move || if active_tab.get() == "generic" { "install-tab active" } else { "install-tab" }
                                on:click=move |_| active_tab.set("generic".into())
                            >
                                "Generic"
                            </button>
                            <button
                                type="button"
                                class=move || if active_tab.get() == "claude" { "install-tab active" } else { "install-tab" }
                                on:click=move |_| active_tab.set("claude".into())
                            >
                                "Claude"
                            </button>
                            <button
                                type="button"
                                class=move || if active_tab.get() == "cursor" { "install-tab active" } else { "install-tab" }
                                on:click=move |_| active_tab.set("cursor".into())
                            >
                                "Cursor"
                            </button>
                        </div>
                        {move || {
                            let tab = active_tab.get();
                            let text = if tab == "claude" {
                                claude.clone()
                            } else if tab == "cursor" {
                                cursor.clone()
                            } else {
                                install.clone()
                            };
                            view! {
                                <div class="tool-install">
                                    <code class="install-cmd">
                                        <span class="install-prefix">"$ "</span>{text.clone()}
                                    </code>
                                    <CopyButton text=text/>
                                </div>
                            }
                        }}
                    </section>
                }
                .into_any()
            } else {
                ().into_any()
            }}
            <section class="links-section">
                <h3 class="install-heading">"Links"</h3>
                <ul class="trust-list">
                    {if let Some(url) = tool.repo_url.clone() {
                        view! {
                            <li>
                                <a href=url target="_blank" rel="noopener" class="external-link">"Repository"</a>
                            </li>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                    {if let Some(url) = tool.homepage.clone() {
                        view! {
                            <li>
                                <a href=url target="_blank" rel="noopener" class="external-link">"Homepage"</a>
                            </li>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                    {if let Some(url) = tool.source_url.clone() {
                        view! {
                            <li>
                                <a href=url target="_blank" rel="noopener" class="external-link">"Source listing"</a>
                            </li>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                </ul>
            </section>
            <section class="trust-section">
                <h3 class="install-heading">"Trust"</h3>
                <ul class="trust-list">
                    <li>"✓ Source: "{tool.source.clone()}</li>
                    {if let Some(team) = tool.official_team.clone() {
                        view! { <li>"✓ Official team: "{team}</li> }.into_any()
                    } else {
                        ().into_any()
                    }}
                    <li>"✓ Stars: "{tool.stars}</li>
                    <li>"✓ Last commit: "{last_commit}</li>
                    {if tool.status == "verified" || tool.status == "official" {
                        view! { <li>"✓ Badge verified by OnchainAI"</li> }.into_any()
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
