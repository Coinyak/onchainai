//! Button-first entry point for per-tool MCP / agent install flow.

use crate::components::icons::LucideIcon;
use crate::models::Tool;
use crate::public_install_guide::{
    add_mcp_action_label, add_mcp_href, add_mcp_href_from_compare, tool_has_install_path,
};
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AddMcpVariant {
    #[default]
    CardIcon,
    InlineButton,
    DetailPrimary,
}

/// How to build the add-mode navigation href for a tool action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AddMcpHrefSource {
    QueryBase(String),
    CompareSlugs(Vec<String>),
}

/// Resolve the shipped add-mode href for a tool slug.
pub fn resolve_add_mcp_href(source: &AddMcpHrefSource, slug: &str) -> String {
    match source {
        AddMcpHrefSource::QueryBase(base) => add_mcp_href(base, slug),
        AddMcpHrefSource::CompareSlugs(slugs) => add_mcp_href_from_compare(slugs, slug),
    }
}

#[component]
pub fn AddMcpAction(
    tool: Tool,
    href_source: AddMcpHrefSource,
    #[prop(optional)] variant: AddMcpVariant,
) -> impl IntoView {
    let slug = tool.slug.clone();
    let href = resolve_add_mcp_href(&href_source, &slug);
    let label = add_mcp_action_label(&tool);
    let has_path = tool_has_install_path(&tool);

    let action_label = label.unwrap_or("Add MCP");

    if !has_path {
        return match variant {
            AddMcpVariant::CardIcon => ().into_any(),
            AddMcpVariant::InlineButton | AddMcpVariant::DetailPrimary => view! {
                <span class="add-mcp-disabled" aria-disabled="true">"No install listed"</span>
            }
            .into_any(),
        };
    }

    match variant {
        AddMcpVariant::CardIcon => view! {
            <a
                href=href
                class="card-action-btn add-mcp-action"
                aria-label=action_label
                title=action_label
                on:click=|ev| ev.stop_propagation()
            >
                <LucideIcon name="plug".to_string() class="card-action-icon"/>
            </a>
        }
        .into_any(),
        AddMcpVariant::InlineButton => view! {
            <a href=href class="add-mcp-inline-btn" on:click=|ev| ev.stop_propagation()>
                {action_label}
            </a>
        }
        .into_any(),
        AddMcpVariant::DetailPrimary => view! {
            <a href=href class="add-mcp-primary-btn" on:click=|ev| ev.stop_propagation()>
                {action_label}
            </a>
        }
        .into_any(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::default_review_fields;

    fn sample_tool(tool_type: &str, install: Option<&str>) -> Tool {
        let review = default_review_fields();
        Tool {
            id: uuid::Uuid::new_v4(),
            name: "Sample".into(),
            slug: "sample".into(),
            description: None,
            function: "dev-tool".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: tool_type.into(),
            repo_url: None,
            homepage: None,
            npm_package: None,
            install_command: install.map(str::to_string),
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
            install_risk_level: "low".into(),
            install_risk_reasons: vec![],
            requires_secret: false,
            safe_copy_command: install.map(str::to_string),
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
    fn add_mcp_action_label_mcp_vs_cli() {
        let mcp = sample_tool("mcp", Some("npx @a/mcp"));
        let cli = sample_tool("cli", Some("npm i @a/cli"));
        assert_eq!(add_mcp_action_label(&mcp), Some("Add MCP"));
        assert_eq!(add_mcp_action_label(&cli), Some("Use with agent"));
    }

    #[test]
    fn query_base_href_includes_intent_and_preserves_filters() {
        let href = resolve_add_mcp_href(
            &AddMcpHrefSource::QueryBase("/tools?type=mcp&chain=base".into()),
            "bridge-mcp",
        );
        assert!(href.contains("selected=bridge-mcp"));
        assert!(href.contains("intent=add-mcp"));
        assert!(href.contains("type=mcp"));
    }

    #[test]
    fn query_base_href_replaces_existing_selected_param() {
        let href = resolve_add_mcp_href(
            &AddMcpHrefSource::QueryBase("/tools?type=mcp&selected=old-tool&intent=add-mcp".into()),
            "new-tool",
        );
        assert!(href.contains("selected=new-tool"));
        assert_eq!(href.matches("selected=").count(), 1);
        assert!(href.contains("intent=add-mcp"));
    }

    #[test]
    fn detail_primary_shows_disabled_when_no_install_path() {
        let tool = sample_tool("mcp", None);
        assert!(!tool_has_install_path(&tool));
    }

    #[test]
    fn compare_href_source_matches_add_mcp_href_from_compare() {
        let slugs = vec!["aave".into(), "uniswap".into()];
        let href =
            resolve_add_mcp_href(&AddMcpHrefSource::CompareSlugs(slugs.clone()), "zapper-mcp");
        assert_eq!(href, add_mcp_href_from_compare(&slugs, "zapper-mcp"));
    }
}
