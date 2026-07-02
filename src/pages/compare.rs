//! Public tool comparison route.

use crate::components::add_mcp_action::{AddMcpAction, AddMcpHrefSource, AddMcpVariant};
use crate::components::error_state::ErrorState;
use crate::components::official_links_list::OfficialLinksList;
use crate::components::site_shell::SiteShell;
use crate::components::skeleton::ToolCardSkeleton;
use crate::components::tool_logo::ToolLogo;
use crate::discovery::normalize_compare_slugs;
use crate::public_install_guide::tool_has_install_path;
use crate::server::functions::{compare_tools, ToolComparisonView};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

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

#[component]
fn CompareCard(row: ToolComparisonView, compare_slugs: Vec<String>) -> impl IntoView {
    let tool = row.tool.clone();
    let chains = if tool.chains.is_empty() {
        "Not listed".into()
    } else {
        tool.chains
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };
    let install_available = tool_has_install_path(&tool);
    let updated = tool
        .last_commit_at
        .map(|value| value.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| tool.updated_at.format("%Y-%m-%d").to_string());

    view! {
        <article class="compare-card">
            <header class="compare-card-header">
                <ToolLogo tool=tool.clone() class="compare-logo" img_class="tool-logo-img compare-logo-img"/>
                <div>
                    <h2>{tool.name.clone()}</h2>
                    <p>{tool.description.clone().unwrap_or_else(|| "No description.".into())}</p>
                </div>
            </header>
            <dl class="compare-facts">
                <div><dt>"Status"</dt><dd>{status_label(&tool.status)}</dd></div>
                <div><dt>"Type"</dt><dd>{tool.tool_type.to_uppercase()}</dd></div>
                <div><dt>"Chains"</dt><dd>{chains}</dd></div>
                <div><dt>"Install risk"</dt><dd>{risk_label(&tool.install_risk_level)}</dd></div>
                <div><dt>"Install/config"</dt><dd>{if install_available { "Available" } else { "Not listed" }}</dd></div>
                <div><dt>"Stars"</dt><dd>{tool.stars}</dd></div>
                <div><dt>"Recent activity"</dt><dd>{updated}</dd></div>
                <div><dt>"Claim"</dt><dd>{tool.claim_state.clone()}</dd></div>
                <div><dt>"Saved"</dt><dd>{if row.viewer_bookmarked { "In toolkit" } else { "Not saved" }}</dd></div>
            </dl>
            <section class="compare-trust">
                <h3>"Why trust this?"</h3>
                {if row.trust_facts.is_empty() {
                    view! { <p>"No verified evidence is listed yet."</p> }.into_any()
                } else {
                    view! {
                        <ul>
                            {row.trust_facts.into_iter().map(|fact| view! {
                                <li><strong>{fact.label}</strong><span>{fact.detail}</span></li>
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }}
                <OfficialLinksList links=row.official_links/>
            </section>
            <div class="compare-card-actions">
                <a href=format!("/tools/{}", tool.slug)>"Open details"</a>
                {if install_available {
                    view! {
                        <AddMcpAction
                            tool=tool.clone()
                            href_source=AddMcpHrefSource::CompareSlugs(compare_slugs.clone())
                            variant=AddMcpVariant::InlineButton
                        />
                    }.into_any()
                } else {
                    ().into_any()
                }}
                <a href="/toolkit">"Open toolkit"</a>
            </div>
        </article>
    }
}

#[component]
pub fn ComparePage() -> impl IntoView {
    let query = use_query_map();
    let retry = RwSignal::new(0u32);
    let slugs = Memo::new(move |_| {
        query.with(|q| {
            q.get("tools")
                .map(|raw| normalize_compare_slugs(raw.as_ref()))
                .unwrap_or_default()
        })
    });
    let comparison = Resource::new(
        move || (slugs.get(), retry.get()),
        |(slugs, _)| async move { compare_tools(slugs).await },
    );

    view! {
        <SiteShell>
            <div class="compare-page">
                <section class="compare-header">
                    <p class="dashboard-kicker">"Compare Tools"</p>
                    <h1>"Choose the safest fit for your agent stack"</h1>
                    <p>"Compare up to 3 public tools by trust evidence, install risk, links, chains, and saved state."</p>
                    <a href="/tools" class="compare-browse-link">"Browse tools"</a>
                </section>
                <Suspense fallback=|| view! { <ToolCardSkeleton/> }>
                    {move || match comparison.get() {
                        Some(Ok(rows)) if rows.is_empty() => view! {
                            <section class="compare-empty">
                                <h2>"No tools selected"</h2>
                                <p>"Use Compare on tool cards or open this route with up to 3 slugs."</p>
                                <a href="/tools" class="compare-primary-link">"Find tools"</a>
                            </section>
                        }.into_any(),
                        Some(Ok(rows)) => {
                            let compare_slugs = slugs.get();
                            view! {
                                <div class="compare-grid">
                                    {rows.into_iter().map(|row| view! {
                                        <CompareCard row=row compare_slugs=compare_slugs.clone()/>
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }
                        Some(Err(error)) => view! {
                            <ErrorState
                                message=format!("Compare failed: {error}")
                                on_retry=move || retry.update(|n| *n = n.wrapping_add(1))
                            />
                        }.into_any(),
                        None => view! { <ToolCardSkeleton/> }.into_any(),
                    }}
                </Suspense>
            </div>
        </SiteShell>
    }
}
