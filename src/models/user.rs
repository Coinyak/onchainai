//! Profile model — maps the `profiles` table.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A user profile row from the `profiles` table.
///
/// `id` mirrors `auth.users(id)` on Supabase. Public exposure should go
/// through the `profiles_public` view (nickname + avatar + auth_method only)
/// to avoid leaking email, wallet address, or GitHub username.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Profile {
    pub id: Uuid,
    pub nickname: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    /// `github` | `email` | `siwx`
    pub auth_method: String,
    /// EVM/Solana address (siwx only).
    pub wallet_address: Option<String>,
    /// `'1'` (EVM) | `'solana'` (siwx only).
    pub chain_id: Option<String>,
    pub is_admin: bool,
    pub is_banned: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public-facing profile projection — never exposes PII.
///
/// Mirrors the `profiles_public` view created in `002_auth.sql`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ProfilePublic {
    pub id: Uuid,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub auth_method: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_serde_round_trip() {
        let now = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let p = Profile {
            id: Uuid::nil(),
            nickname: Some("alice".into()),
            bio: None,
            avatar_url: None,
            auth_method: "github".into(),
            wallet_address: None,
            chain_id: None,
            is_admin: false,
            is_banned: false,
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_string(&p).expect("serialize profile");
        let back: Profile = serde_json::from_str(&json).expect("deserialize profile");
        assert_eq!(back.auth_method, "github");
        assert_eq!(back.nickname.as_deref(), Some("alice"));
    }
}
