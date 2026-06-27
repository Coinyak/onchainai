//! Stripe-style tool card for list views — badges, bookmark, upvote.

use crate::chains::chain_tags_show_all;
use crate::components::copy_button::CopyButton;
use crate::components::login_modal::LoginModal;
use crate::components::tool_logo::ToolLogo;
use crate::models::Tool;
use crate::server::functions::{get_current_user, toggle_bookmark};
use leptos::prelude::*;
use leptos::task::spawn_local;

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
    #[prop(optional)] is_selected: bool,
    #[prop(optional)] comment_count: i64,
) -> impl IntoView {
    let slug = tool.slug.clone();
    let detail_href = format!("/tools/{slug}");
    let href = preview_href.unwrap_or(detail_href);

    let status = tool.status.clone();
    let tool_type = tool.tool_type.clone();
    let chains = tool.chains.clone();
    let (chain_preview, extra_chains) = chain_tags_show_all(&chains);
    let install = tool.install_command.clone().unwrap_or_default();
    let stars = tool.stars;
    let description = tool
        .description
        .clone()
        .unwrap_or_else(|| "No description.".into());
    let team = tool
        .official_team
        .clone()
        .unwrap_or_else(|| tool.source.clone());
    let time_ago = tool
        .last_commit_at
        .map(|t| {
            let now = chrono::Utc::now();
            let diff = now.signed_duration_since(t);
            if diff.num_days() > 0 {
                format!("{}d ago", diff.num_days())
            } else if diff.num_hours() > 0 {
                format!("{}h ago", diff.num_hours())
            } else {
                "today".to_string()
            }
        })
        .unwrap_or_else(|| "—".into());
    let license = tool.license.clone().unwrap_or_default();

    let show_login = RwSignal::new(false);
    let starred = RwSignal::new(false);
    view! {
        <LoginModal show=show_login/>
        <article class=if is_selected { "tool-card is-selected" } else { "tool-card" }>
            <a href=href class="tool-card-link no-underline text-inherit">
                <div class="tool-card-inner">
                    <ToolLogo tool=tool.clone()/>
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
                        <div class="tool-source-line">
                            <span class="tool-team">{team.clone()}</span>
                            <span class="tool-meta-sep">"·"</span>
                            <span class="tool-time">{time_ago.clone()}</span>
                            {if !license.is_empty() {
                                view! {
                                    <span class="tool-meta-sep">"·"</span>
                                    <span class="tool-license">{license.clone()}</span>
                                }.into_any()
                            } else {
                                ().into_any()
                            }}
                        </div>
                        <div class="tool-meta">
                            <span class="tool-chains">
                                {chain_preview.into_iter().map(|tag| {
                                    if let Some(meta) = tag.meta {
                                        view! {
                                            <img
                                                class="chain-logo chain-logo-tag"
                                                src=meta.logo
                                                alt=meta.label
                                                title=meta.label
                                            />
                                        }.into_any()
                                    } else {
                                        view! { <span class="chain-pill">{tag.raw}</span> }.into_any()
                                    }
                                }).collect_view()}
                                {if extra_chains > 0 {
                                    view! { <span class="chain-pill chain-more">{"+"}{extra_chains}</span> }
                                        .into_any()
                                } else {
                                    ().into_any()
                                }}
                            </span>
                            <span class="tool-meta-sep">"·"</span>
                            <span class="tool-stars">{"★ "}{stars}</span>
                            <span class="tool-meta-sep">"·"</span>
                            <span class="tool-comments">"comments "{comment_count}</span>
                        </div>
                        {if !install.is_empty() {
                            view! {
                                <div class="tool-install hidden md:flex">
                                    <code class="install-cmd">
                                        <span class="install-prefix">"$ "</span>{install.clone()}
                                    </code>
                                    <CopyButton text=install/>
                                </div>
                            }
                            .into_any()
                        } else {
                            ().into_any()
                        }}
                    </div>
                </div>
            </a>
            <div class="tool-card-actions">
                <button
                    type="button"
                    class="card-action-btn"
                    aria-label="Toggle bookmark"
                    on:click=move |ev| {
                        ev.stop_propagation();
                        let slug_toggle = slug.clone();
                        spawn_local(async move {
                            match get_current_user().await {
                                Ok(Some(_)) => {
                                    if let Ok(now_starred) = toggle_bookmark(slug_toggle).await {
                                        starred.set(now_starred);
                                    }
                                }
                                _ => show_login.set(true),
                            }
                        });
                    }
                >
                    {move || if starred.get() { "★" } else { "☆" }}
                </button>
            </div>
        </article>
    }
}
