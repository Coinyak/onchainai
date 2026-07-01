//! Stripe-style tool card for list views — badges, bookmark, upvote.

use crate::auth::session::has_access_token_cookie;
use crate::chains::{chain_fallback_label, chain_tags_for_tool, ChainTagView};
use crate::components::admin_context::AdminOnly;
use crate::components::chain_logo::ChainLogo;
use crate::components::copy_button::CopyButton;
use crate::components::highlighted_command::HighlightedCommand;
use crate::components::icons::LucideIcon;
use crate::components::login_modal::LoginModal;
use crate::components::tool_logo::ToolLogo;
use crate::discovery::compare_href;
use crate::models::Tool;
use crate::server::functions::{is_bookmarked, review_tool, set_bookmark, ReviewToolPayload};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Desktop tool card: show up to 5 chain tags, then "+N" (DESIGN.md / UI_UX_DESIGN.md).
const CHAINS_VISIBLE_DESKTOP: usize = 5;
/// Mobile tool card: show up to 3 chain tags, then "+N".
const CHAINS_VISIBLE_MOBILE: usize = 3;
const QUICK_VERIFY_REASON: &str = "Verified via public-card admin quick action.";
const QUICK_DEMOTE_REASON: &str = "Demoted via public-card admin quick action.";

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

fn install_risk_badge_class(risk: &str) -> &'static str {
    match risk {
        "low" => "badge badge-risk-low",
        "medium" => "badge badge-risk-medium",
        "high" => "badge badge-risk-high",
        "critical" => "badge badge-risk-critical",
        _ => "badge badge-neutral",
    }
}

fn install_risk_badge_label(risk: &str) -> &'static str {
    match risk {
        "low" => "Low risk",
        "medium" => "Medium risk",
        "high" => "High risk",
        "critical" => "Critical risk",
        _ => "Risk review",
    }
}

fn bookmark_action_label(starred: bool) -> &'static str {
    if starred {
        "Remove from Toolkit"
    } else {
        "Save to Toolkit"
    }
}

fn bookmark_icon_class(starred: bool) -> &'static str {
    if starred {
        "bookmark-icon is-filled"
    } else {
        "bookmark-icon"
    }
}

fn bookmark_icon_fill(starred: bool) -> &'static str {
    if starred {
        "currentColor"
    } else {
        "none"
    }
}

fn bookmark_icon(starred: bool) -> impl IntoView {
    view! {
        <LucideIcon
            name="star".to_string()
            class=bookmark_icon_class(starred)
            fill=bookmark_icon_fill(starred)
            stroke="currentColor"
        />
    }
}

fn status_badge_label(status: &str) -> &'static str {
    match status {
        "verified" => "Verified",
        "official" => "Official",
        _ => "Community",
    }
}

fn can_mark_verified(status: &str) -> bool {
    !matches!(status, "verified" | "official")
}

fn demote_action_for_status(status: &str) -> Option<&'static str> {
    match status {
        "verified" => Some("demote_verified"),
        "official" => Some("demote_official"),
        _ => None,
    }
}

fn confirm_quick_verify() -> bool {
    #[cfg(feature = "hydrate")]
    {
        web_sys::window()
            .and_then(|window| window.confirm_with_message("Mark this tool verified?").ok())
            .unwrap_or(false)
    }
    #[cfg(not(feature = "hydrate"))]
    {
        true
    }
}

fn confirm_quick_demote(status: &str) -> bool {
    #[cfg(feature = "hydrate")]
    {
        let message = match status {
            "verified" => "Remove verified status and demote to community?",
            "official" => "Remove official status and demote to community?",
            _ => return false,
        };
        web_sys::window()
            .and_then(|window| window.confirm_with_message(message).ok())
            .unwrap_or(false)
    }
    #[cfg(not(feature = "hydrate"))]
    {
        matches!(status, "verified" | "official")
    }
}

#[component]
fn AdminToolCardActions(slug: String, status: RwSignal<String>) -> impl IntoView {
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let review_href = RwSignal::new(format!("/admin/tools?slug={slug}"));
    let action_slug = RwSignal::new(slug.clone());

    view! {
        <AdminOnly>
            {move || {
                let review_href = review_href.get();
                view! {
                    <a
                        class="card-action-btn admin-card-action-link"
                        href=review_href
                        aria-label="Review or edit"
                        title="Review or edit"
                        on:click=|ev| ev.stop_propagation()
                    >
                        <LucideIcon name="pencil".to_string() class="card-action-icon"/>
                    </a>
                    <Show when=move || can_mark_verified(&status.get())>
                        <button
                            type="button"
                            class="card-action-btn admin-card-action-btn"
                            aria-label="Mark verified"
                            title="Mark verified"
                            disabled=move || busy.get()
                            on:click=move |ev| {
                                ev.stop_propagation();
                                if busy.get_untracked() {
                                    return;
                                }
                                if !confirm_quick_verify() {
                                    return;
                                }
                                busy.set(true);
                                error.set(None);
                                let slug = action_slug.get_untracked();
                                spawn_local(async move {
                                    let result = review_tool(ReviewToolPayload {
                                        slug,
                                        action: "mark_verified".into(),
                                        reason: QUICK_VERIFY_REASON.into(),
                                        override_reason: None,
                                        expected_updated_at: None,
                                        snapshot_id: None,
                                        recommendation_id: None,
                                    }).await;
                                    busy.set(false);
                                    match result {
                                        Ok(()) => status.set("verified".into()),
                                        Err(e) => error.set(Some(e.to_string())),
                                    }
                                });
                            }
                        >
                            <LucideIcon name="check".to_string() class="card-action-icon"/>
                        </button>
                    </Show>
                    <Show when=move || demote_action_for_status(&status.get()).is_some()>
                        <button
                            type="button"
                            class="card-action-btn admin-card-action-btn admin-card-action-btn-revoke"
                            aria-label="Revoke verified/official status"
                            title="Revoke verified/official status"
                            disabled=move || busy.get()
                            on:click=move |ev| {
                                ev.stop_propagation();
                                if busy.get_untracked() {
                                    return;
                                }
                                let current_status = status.get_untracked();
                                let Some(action) = demote_action_for_status(&current_status) else {
                                    return;
                                };
                                if !confirm_quick_demote(&current_status) {
                                    return;
                                }
                                busy.set(true);
                                error.set(None);
                                let slug = action_slug.get_untracked();
                                spawn_local(async move {
                                    let result = review_tool(ReviewToolPayload {
                                        slug,
                                        action: action.into(),
                                        reason: QUICK_DEMOTE_REASON.into(),
                                        override_reason: None,
                                        expected_updated_at: None,
                                        snapshot_id: None,
                                        recommendation_id: None,
                                    }).await;
                                    busy.set(false);
                                    match result {
                                        Ok(()) => status.set("community".into()),
                                        Err(e) => error.set(Some(e.to_string())),
                                    }
                                });
                            }
                        >
                            <LucideIcon name="x".to_string() class="card-action-icon"/>
                        </button>
                    </Show>
                    {move || error.get().map(|msg| view! {
                        <span class="sr-only" role="status">{msg.clone()}</span>
                        <span class="admin-card-error" role="alert">
                            <LucideIcon name="alert-circle".to_string() class="card-action-icon"/>
                            {msg}
                        </span>
                    })}
                }.into_any()
            }}
        </AdminOnly>
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
                            <ChainLogo
                                id=meta.id.to_string()
                                label=meta.label.to_string()
                                class="chain-logo chain-logo-tag"
                                size=20
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
    #[prop(optional)] initially_starred: bool,
    #[prop(optional)] on_bookmark_changed: Option<Callback<bool>>,
) -> impl IntoView {
    let slug = tool.slug.clone();
    let slug_for_bookmark_sync = slug.clone();
    let detail_href = format!("/tools/{slug}");
    let href = preview_href.unwrap_or(detail_href);

    let status = RwSignal::new(tool.status.clone());
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
    let risk_level = tool.install_risk_level.clone();
    let compare_url = compare_href(std::slice::from_ref(&slug));

    let show_login = RwSignal::new(false);
    let starred = RwSignal::new(initially_starred);

    Effect::new(move |_| {
        if !has_access_token_cookie() {
            return;
        }
        let slug_sync = slug_for_bookmark_sync.clone();
        spawn_local(async move {
            if let Ok(bookmarked) = is_bookmarked(slug_sync).await {
                starred.set(bookmarked);
            }
        });
    });

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
                                <span class=move || badge_class(&status.get())>
                                    {move || status_badge_label(&status.get())}
                                </span>
                                <span class=type_badge_class(&tool_type)>{tool_type.to_uppercase()}</span>
                                {if tool.claim_state == "claimed" {
                                    view! {
                                        <span class="badge badge-neutral">"Claimed by team"</span>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }}
                                {if !risk_level.is_empty() {
                                    let risk_level_for_label = risk_level.clone();
                                    view! {
                                        <span class=install_risk_badge_class(&risk_level)>
                                            {install_risk_badge_label(&risk_level_for_label)}
                                        </span>
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
                                    <HighlightedCommand text=install.clone()/>
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
                <AdminToolCardActions slug=slug.clone() status=status/>
                <button
                    type="button"
                    class="card-action-btn"
                    aria-label=move || bookmark_action_label(starred.get())
                    aria-pressed=move || if starred.get() { "true" } else { "false" }
                    title=move || bookmark_action_label(starred.get())
                    on:click=move |ev| {
                        ev.stop_propagation();
                        let slug_toggle = slug.clone();
                        if !has_access_token_cookie() {
                            show_login.set(true);
                            return;
                        }
                        spawn_local(async move {
                            let want_starred = !starred.get_untracked();
                            match set_bookmark(slug_toggle, want_starred).await {
                                Ok(now_starred) => {
                                    starred.set(now_starred);
                                    if let Some(callback) = on_bookmark_changed {
                                        callback.run(now_starred);
                                    }
                                }
                                Err(_) => show_login.set(true),
                            }
                        });
                    }
                >
                    {move || bookmark_icon(starred.get())}
                </button>
                <a
                    class="card-action-btn"
                    href=compare_url
                    aria-label="Compare"
                    title="Compare"
                    on:click=|ev| ev.stop_propagation()
                >
                    <LucideIcon name="arrow-left-right".to_string() class="card-action-icon"/>
                </a>
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
        assert_eq!(status_badge_label("verified"), "Verified");
        assert_eq!(status_badge_label("official"), "Official");
        assert_eq!(status_badge_label("community"), "Community");
        assert!(!can_mark_verified("verified"));
        assert!(!can_mark_verified("official"));
        assert!(can_mark_verified("community"));
        assert_eq!(
            demote_action_for_status("verified"),
            Some("demote_verified")
        );
        assert_eq!(
            demote_action_for_status("official"),
            Some("demote_official")
        );
        assert_eq!(demote_action_for_status("community"), None);
    }

    #[test]
    fn bookmark_labels_match_toolkit_state() {
        assert_eq!(bookmark_action_label(false), "Save to Toolkit");
        assert_eq!(bookmark_action_label(true), "Remove from Toolkit");
        assert_eq!(bookmark_icon_class(false), "bookmark-icon");
        assert_eq!(bookmark_icon_class(true), "bookmark-icon is-filled");
        assert_eq!(bookmark_icon_fill(false), "none");
        assert_eq!(bookmark_icon_fill(true), "currentColor");
    }

    #[test]
    fn install_risk_badges_match_trust_state() {
        assert_eq!(install_risk_badge_class("low"), "badge badge-risk-low");
        assert_eq!(
            install_risk_badge_class("medium"),
            "badge badge-risk-medium"
        );
        assert_eq!(install_risk_badge_class("high"), "badge badge-risk-high");
        assert_eq!(
            install_risk_badge_class("critical"),
            "badge badge-risk-critical"
        );
        assert_eq!(install_risk_badge_label("critical"), "Critical risk");
    }
}
