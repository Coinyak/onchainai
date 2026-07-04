//! Database models — one struct per DB table.
//!
//! Each model derives `sqlx::FromRow` so rows can be decoded directly via
//! `query_as!`. Field types match the migrations exactly.
//!
//! Inline models (`Source`, `SiwxSession`, `SiteSettings`) live here rather
//! than in separate files because they are small and single-purpose.

pub mod category;
pub mod comment;
pub mod featured;
pub mod review;
pub mod submission;
pub mod tool;
pub mod user;

// Re-exports are the public API of the models module. They are unused until
// later milestones wire them into server functions and the MCP handler.
#[allow(unused_imports)]
pub use category::Category;
#[allow(unused_imports)]
pub use comment::{Bookmark, Comment, Upvote};
#[allow(unused_imports)]
pub use featured::FeaturedCard;
#[allow(unused_imports)]
pub use review::{
    official_link_display_label, OperatorVerdict, ReviewEntry, ReviewRun, ToolOfficialLink,
};
#[allow(unused_imports)]
pub use submission::{
    ToolClaimRequest, ToolReport, ToolSubmission, ToolSubmissionPayload, CLAIM_STATES,
    TOOL_REPORT_REASONS,
};
#[allow(unused_imports)]
pub use tool::Tool;
#[allow(unused_imports)]
pub use user::{Profile, ProfilePublic};

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A `sources` row — crawler status tracking.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Source {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub last_crawled_at: Option<DateTime<Utc>>,
    /// `pending` | `success` | `error`
    pub crawl_status: String,
    pub items_found: i32,
    pub error_message: Option<String>,
    pub schedule_minutes: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Footer navigation link stored in `site_settings.footer_links` JSONB.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct FooterLink {
    pub label: String,
    pub url: String,
}

/// A `siwx_sessions` row — server-side only, no client RLS policies.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct SiwxSession {
    pub id: Uuid,
    pub nonce: String,
    pub wallet_address: String,
    pub chain_id: String,
    pub message: String,
    pub signature: String,
    pub issued_at: DateTime<Utc>,
    pub expiration_time: DateTime<Utc>,
    pub used: bool,
    pub used_at: Option<DateTime<Utc>>,
    pub profile_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// A `site_settings` row — singleton (`id = 1`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct SiteSettings {
    pub id: i32,
    pub site_name: String,
    pub slogan: String,
    pub description: String,
    pub mcp_endpoint: String,
    pub search_keywords: Vec<String>,
    pub allow_free_registration: bool,
    pub require_tool_approval: bool,
    pub allow_x402_registration: bool,
    pub default_referral_bps: Option<i32>,
    pub default_referral_payout_address: Option<String>,
    pub x402_builder_code: Option<String>,
    pub mcp_premium_enabled: bool,
    pub mcp_premium_pay_to_address: Option<String>,
    pub mcp_premium_price: Option<String>,
    pub mcp_premium_network: String,
    pub mcp_premium_asset: Option<String>,
    pub mcp_premium_display_price: Option<String>,
    pub hero_title: Option<String>,
    pub hero_subtitle: Option<String>,
    pub about_content: Option<String>,
    #[cfg_attr(feature = "ssr", sqlx(json))]
    pub footer_links: Vec<FooterLink>,
    pub updated_at: DateTime<Utc>,
}

/// Strip operator-only referral config before serializing site settings to public clients.
pub fn sanitize_site_settings_for_public(mut settings: SiteSettings) -> SiteSettings {
    settings.default_referral_bps = None;
    settings.default_referral_payout_address = None;
    settings.x402_builder_code = None;
    settings.mcp_premium_pay_to_address = None;
    settings.mcp_premium_price = None;
    settings.mcp_premium_asset = None;
    settings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_serde_round_trip() {
        let now = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let s = Source {
            id: Uuid::nil(),
            name: "npm".into(),
            url: "https://registry.npmjs.org/".into(),
            last_crawled_at: Some(now),
            crawl_status: "success".into(),
            items_found: 42,
            error_message: None,
            schedule_minutes: 60,
            enabled: true,
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_string(&s).expect("serialize source");
        let back: Source = serde_json::from_str(&json).expect("deserialize source");
        assert_eq!(back.name, "npm");
        assert_eq!(back.items_found, 42);
    }

    #[test]
    fn siwx_session_serde_round_trip() {
        let now = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let s = SiwxSession {
            id: Uuid::nil(),
            nonce: "abc".into(),
            wallet_address: "0xabc".into(),
            chain_id: "1".into(),
            message: "msg".into(),
            signature: "0xsig".into(),
            issued_at: now,
            expiration_time: now,
            used: false,
            used_at: None,
            profile_id: None,
            created_at: now,
        };
        let json = serde_json::to_string(&s).expect("serialize siwx_session");
        let back: SiwxSession = serde_json::from_str(&json).expect("deserialize siwx_session");
        assert_eq!(back.chain_id, "1");
        assert!(!back.used);
    }

    #[test]
    fn site_settings_serde_round_trip() {
        let now = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let s = SiteSettings {
            id: 1,
            site_name: "OnchainAI".into(),
            slogan: "Crypto tools, unified.".into(),
            description: "desc".into(),
            mcp_endpoint: "npx mcp-remote www.onchain-ai.xyz/mcp".into(),
            search_keywords: vec!["mcp-server".into()],
            allow_free_registration: true,
            require_tool_approval: true,
            allow_x402_registration: false,
            default_referral_bps: Some(250),
            default_referral_payout_address: Some(
                "0x0000000000000000000000000000000000000000".into(),
            ),
            x402_builder_code: Some("onchainai".into()),
            mcp_premium_enabled: false,
            mcp_premium_pay_to_address: None,
            mcp_premium_price: None,
            mcp_premium_network: "eip155:8453".into(),
            mcp_premium_asset: None,
            mcp_premium_display_price: None,
            hero_title: Some("Find crypto tools".into()),
            hero_subtitle: Some("MCP, CLI, SDK, and more".into()),
            about_content: Some("About OnchainAI".into()),
            footer_links: vec![FooterLink {
                label: "GitHub".into(),
                url: "https://github.com".into(),
            }],
            updated_at: now,
        };
        let json = serde_json::to_string(&s).expect("serialize settings");
        let back: SiteSettings = serde_json::from_str(&json).expect("deserialize settings");
        assert_eq!(back.id, 1);
        assert!(back.allow_free_registration);
        assert_eq!(back.search_keywords, vec!["mcp-server".to_string()]);
        assert_eq!(back.default_referral_bps, Some(250));
        assert_eq!(back.x402_builder_code.as_deref(), Some("onchainai"));
    }

    #[test]
    fn sanitize_site_settings_for_public_strips_referral_fields() {
        let now = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let settings = SiteSettings {
            id: 1,
            site_name: "OnchainAI".into(),
            slogan: "slogan".into(),
            description: "desc".into(),
            mcp_endpoint: "npx mcp-remote www.onchain-ai.xyz/mcp".into(),
            search_keywords: vec![],
            allow_free_registration: true,
            require_tool_approval: true,
            allow_x402_registration: false,
            default_referral_bps: Some(250),
            default_referral_payout_address: Some(
                "0x0000000000000000000000000000000000000000".into(),
            ),
            x402_builder_code: Some("onchainai".into()),
            mcp_premium_enabled: true,
            mcp_premium_pay_to_address: Some(
                "0x0000000000000000000000000000000000000000".into(),
            ),
            mcp_premium_price: Some("$0.01".into()),
            mcp_premium_network: "eip155:8453".into(),
            mcp_premium_asset: None,
            mcp_premium_display_price: Some("$0.01/call".into()),
            hero_title: None,
            hero_subtitle: None,
            about_content: None,
            footer_links: vec![],
            updated_at: now,
        };
        let public = sanitize_site_settings_for_public(settings);
        assert_eq!(public.default_referral_bps, None);
        assert_eq!(public.default_referral_payout_address, None);
        assert_eq!(public.x402_builder_code, None);
        assert_eq!(public.mcp_premium_pay_to_address, None);
        assert_eq!(public.mcp_premium_price, None);
    }
}
