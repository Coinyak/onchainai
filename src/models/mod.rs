//! Database models — one struct per DB table.
//!
//! Each model derives `sqlx::FromRow` so rows can be decoded directly via
//! `query_as!`. Field types match the migrations exactly.
//!
//! Inline models (`Source`, `SiwxSession`, `SiteSettings`) live here rather
//! than in separate files because they are small and single-purpose.

pub mod category;
pub mod comment;
pub mod tool;
pub mod user;

// Re-exports are the public API of the models module. They are unused until
// later milestones wire them into server functions and the MCP handler.
#[allow(unused_imports)]
pub use category::Category;
#[allow(unused_imports)]
pub use comment::{Bookmark, Comment, Upvote};
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub updated_at: DateTime<Utc>,
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
            updated_at: now,
        };
        let json = serde_json::to_string(&s).expect("serialize settings");
        let back: SiteSettings = serde_json::from_str(&json).expect("deserialize settings");
        assert_eq!(back.id, 1);
        assert!(back.allow_free_registration);
        assert_eq!(back.search_keywords, vec!["mcp-server".to_string()]);
    }
}
