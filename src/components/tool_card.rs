//! Stripe-style tool card for list views — badges, bookmark, upvote.

use crate::chains::{chain_fallback_label, chain_logo_path, chain_tags_for_tool, ChainTagView};
use crate::components::copy_button::CopyButton;
use crate::components::login_modal::LoginModal;
use crate::components::tool_logo::ToolLogo;
use crate::models::Tool;
use crate::server::functions::{get_current_user, toggle_bookmark};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Desktop tool card: show up to 5 chain tags, then "+N" (DESIGN.md / UI_UX_DESIGN.md).
const CHAINS_VISIBLE_DESKTOP: usize = 5;
/// Mobile tool card: show up to 3 chain tags, then "+N".
const CHAINS_VISIBLE_MOBILE: usize = 3;

fn badge_class(status: &str) -> &'static str {
    match status {
        "verified" => "badge badge-verified",
        "official" => "badge badge-official",
        "community" => "badge badge-community",
        _ => "badge badge-neutral",
    }
}

fn type_badge_class(tool_type: &str) -> &'static str {
    match tool_type {
        "x402" => "badge badge-x402",
        _ => "badge badge-neutral",
    }
}

fn render_chain_tags(
    preview: Vec<ChainTagView>,
    extra: usize,
    class: &'static str,
) -> impl IntoView {
    view! {
        <span class=class>
            {preview
                .into_iter()
                .map(|tag| {
                    if let Some(meta) = tag.meta {
                        view! {
                            <img
                                class="chain-logo chain-logo-tag"
                                src=chain_logo_path(meta.id)
                                alt=meta.label
                                title=meta.label
                                width="20"
                                height="20"
                            />
                        }
                        .into_any()
                    } else {
                        let label = chain_fallback_label(&tag.raw);
                        let title = tag.raw.clone();
                        view! {
                            <span class="chain-pill" title=title>{label}</span>
                        }
                        .into_any()
                    }
                })
                .collect_view()}
            {if extra > 0 {
                view! {
                    <span class="chain-pill chain-more" title=format!("{extra} more chains")>
                        {"+"}{extra}
                    </span>
                }
                .into_any()
            } else {
                ().into_any()
            }}
        </span>
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
    let (chain_desktop, extra_desktop) = chain_tags_for_tool(&chains, CHAINS_VISIBLE_DESKTOP);
    let (chain_mobile, extra_mobile) = chain_tags_for_tool(&chains, CHAINS_VISIBLE_MOBILE);
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
                                {if tool.claim_state == "claimed" {
                                    view! {
                                        <span class="badge badge-neutral">"Claimed by team"</span>
                                    }.into_any()
                                } else if tool.install_risk_level == "low" && !install.is_empty() {
                                    view! {
                                        <span class="badge badge-neutral">"Verified install"</span>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }}
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
                            {render_chain_tags(chain_desktop, extra_desktop, "tool-chains tool-chains-desktop")}
                            {render_chain_tags(chain_mobile, extra_mobile, "tool-chains tool-chains-mobile")}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::chain_tags_for_tool;

    #[test]
    fn tool_card_chain_limits_match_design() {
        let chains: Vec<String> = vec![
            "bitcoin".into(),
            "ethereum".into(),
            "base".into(),
            "solana".into(),
            "arbitrum".into(),
            "optimism".into(),
        ];
        let (desktop, extra_desktop) = chain_tags_for_tool(&chains, CHAINS_VISIBLE_DESKTOP);
        assert_eq!(desktop.len(), 5);
        assert_eq!(extra_desktop, 1);

        let (mobile, extra_mobile) = chain_tags_for_tool(&chains, CHAINS_VISIBLE_MOBILE);
        assert_eq!(mobile.len(), 3);
        assert_eq!(extra_mobile, 3);
    }

    #[test]
    fn chain_pill_label_abbreviates_long_values() {
        assert_eq!(chain_fallback_label("hyperliquid"), "HYPE");
        assert_eq!(chain_fallback_label("eth"), "ETH");
        assert_eq!(chain_fallback_label("binance-smart-chain"), "BINA");
    }

    #[test]
    fn badge_classes_match_design() {
        assert_eq!(badge_class("verified"), "badge badge-verified");
        assert_eq!(badge_class("official"), "badge badge-official");
        assert_eq!(badge_class("community"), "badge badge-community");
        assert_eq!(type_badge_class("x402"), "badge badge-x402");
        assert_eq!(type_badge_class("mcp"), "badge badge-neutral");
    }
}
