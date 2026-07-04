//! Abuse rate-limit policies for user actions and public endpoints.
//!
//! User-scoped limits use in-memory keyed governors (per process).
//! IP-scoped limits for Axum routes are configured in [`crate::build_app`].

use axum::http::request::Parts;
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::LazyLock;
use uuid::Uuid;

/// User may submit at most 5 tools per hour.
pub const SUBMIT_PER_HOUR: u32 = 5;
/// User may post at most 10 comments per minute.
pub const COMMENT_PER_MINUTE: u32 = 10;
/// User may toggle bookmarks at most 60 times per minute.
pub const BOOKMARK_PER_MINUTE: u32 = 60;
/// MCP clients may call at most 100 times per minute per IP.
pub const MCP_PER_MINUTE: u32 = 100;
/// Auth endpoints allow at most 5 attempts per minute per IP.
pub const AUTH_PER_MINUTE: u32 = 5;
/// Admin x402 manual re-probes: at most 10 per minute per admin.
pub const ADMIN_X402_VERIFY_PER_MINUTE: u32 = 10;
/// Agent token mint: at most 5 per hour per user.
pub const AGENT_TOKEN_MINT_PER_HOUR: u32 = 5;
/// Agent blueprint-node sync: at most 30 per minute per user.
pub const AGENT_BLUEPRINT_SYNC_PER_MINUTE: u32 = 30;
/// x402 self-listing probe previews: at most 10 per minute per user (outbound fetch).
pub const X402_PROBE_PER_MINUTE: u32 = 10;
/// General API traffic baseline (see [`crate::build_app`] — burst is 2× this, 5 req/s refill).
pub const GENERAL_PER_MINUTE: u32 = 60;

static SUBMIT_LIMITER: LazyLock<DefaultKeyedRateLimiter<Uuid>> =
    LazyLock::new(|| RateLimiter::dashmap(submit_quota()));
static COMMENT_LIMITER: LazyLock<DefaultKeyedRateLimiter<Uuid>> =
    LazyLock::new(|| RateLimiter::dashmap(comment_quota()));
static BOOKMARK_LIMITER: LazyLock<DefaultKeyedRateLimiter<Uuid>> =
    LazyLock::new(|| RateLimiter::dashmap(bookmark_quota()));
static MCP_IP_LIMITER: LazyLock<DefaultKeyedRateLimiter<String>> =
    LazyLock::new(|| RateLimiter::dashmap(mcp_ip_quota()));
static ADMIN_X402_VERIFY_LIMITER: LazyLock<DefaultKeyedRateLimiter<Uuid>> =
    LazyLock::new(|| RateLimiter::dashmap(admin_x402_verify_quota()));
static AGENT_TOKEN_MINT_LIMITER: LazyLock<DefaultKeyedRateLimiter<Uuid>> =
    LazyLock::new(|| RateLimiter::dashmap(agent_token_mint_quota()));
static AGENT_BLUEPRINT_SYNC_LIMITER: LazyLock<DefaultKeyedRateLimiter<Uuid>> =
    LazyLock::new(|| RateLimiter::dashmap(agent_blueprint_sync_quota()));
static X402_PROBE_LIMITER: LazyLock<DefaultKeyedRateLimiter<Uuid>> =
    LazyLock::new(|| RateLimiter::dashmap(x402_probe_quota()));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRateLimitAction {
    SubmitTool,
    CreateComment,
    ToggleBookmark,
    AdminX402Verify,
    AgentBlueprintSync,
    X402Probe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitExceeded {
    pub message: &'static str,
}

impl std::fmt::Display for RateLimitExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for RateLimitExceeded {}

pub fn submit_quota() -> Quota {
    Quota::per_hour(NonZeroU32::new(SUBMIT_PER_HOUR).expect("non-zero submit quota"))
}

pub fn comment_quota() -> Quota {
    Quota::per_minute(NonZeroU32::new(COMMENT_PER_MINUTE).expect("non-zero comment quota"))
}

pub fn bookmark_quota() -> Quota {
    Quota::per_minute(NonZeroU32::new(BOOKMARK_PER_MINUTE).expect("non-zero bookmark quota"))
}

pub fn mcp_ip_quota() -> Quota {
    Quota::per_minute(NonZeroU32::new(MCP_PER_MINUTE).expect("non-zero mcp quota"))
}

pub fn admin_x402_verify_quota() -> Quota {
    Quota::per_minute(
        NonZeroU32::new(ADMIN_X402_VERIFY_PER_MINUTE).expect("non-zero admin x402 verify quota"),
    )
}

pub fn agent_token_mint_quota() -> Quota {
    Quota::per_hour(
        NonZeroU32::new(AGENT_TOKEN_MINT_PER_HOUR).expect("non-zero agent token mint quota"),
    )
}

pub fn agent_blueprint_sync_quota() -> Quota {
    Quota::per_minute(
        NonZeroU32::new(AGENT_BLUEPRINT_SYNC_PER_MINUTE)
            .expect("non-zero agent blueprint sync quota"),
    )
}

pub fn x402_probe_quota() -> Quota {
    Quota::per_minute(NonZeroU32::new(X402_PROBE_PER_MINUTE).expect("non-zero x402 probe quota"))
}

/// Check a per-user rate limit before mutating state.
pub fn check_user_rate_limit(
    user_id: Uuid,
    action: UserRateLimitAction,
) -> Result<(), RateLimitExceeded> {
    let limiter = match action {
        UserRateLimitAction::SubmitTool => &SUBMIT_LIMITER,
        UserRateLimitAction::CreateComment => &COMMENT_LIMITER,
        UserRateLimitAction::ToggleBookmark => &BOOKMARK_LIMITER,
        UserRateLimitAction::AdminX402Verify => &ADMIN_X402_VERIFY_LIMITER,
        UserRateLimitAction::AgentBlueprintSync => &AGENT_BLUEPRINT_SYNC_LIMITER,
        UserRateLimitAction::X402Probe => &X402_PROBE_LIMITER,
    };
    limiter.check_key(&user_id).map_err(|_| RateLimitExceeded {
        message: "too many requests; try again later",
    })
}

/// Check agent token mint rate (device approve + manual mint).
pub fn check_agent_token_mint_limit(user_id: Uuid) -> Result<(), RateLimitExceeded> {
    AGENT_TOKEN_MINT_LIMITER
        .check_key(&user_id)
        .map_err(|_| RateLimitExceeded {
            message: "agent token mint limit exceeded; try again later",
        })
}

/// Check MCP per-IP limit inside the JSON-RPC handler.
pub fn check_mcp_ip_rate_limit(ip: &str) -> Result<(), RateLimitExceeded> {
    MCP_IP_LIMITER
        .check_key(&ip.to_string())
        .map_err(|_| RateLimitExceeded {
            message: "MCP rate limit exceeded; try again later",
        })
}

/// Best-effort client IP extraction for server functions and MCP.
pub fn client_ip_from_parts(parts: &Parts) -> String {
    if let Some(forwarded) = parts
        .headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(first) = forwarded.split(',').next() {
            let ip = first.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }

    if let Some(real_ip) = parts
        .headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return real_ip.to_string();
    }

    parts
        .extensions
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quota_constants_match_security_policy() {
        assert_eq!(SUBMIT_PER_HOUR, 5);
        assert_eq!(COMMENT_PER_MINUTE, 10);
        assert_eq!(BOOKMARK_PER_MINUTE, 60);
        assert_eq!(MCP_PER_MINUTE, 100);
        assert_eq!(AUTH_PER_MINUTE, 5);
        assert_eq!(GENERAL_PER_MINUTE, 60);
        assert_eq!(ADMIN_X402_VERIFY_PER_MINUTE, 10);
    }

    #[test]
    fn submit_limit_allows_five_then_blocks() {
        let user = Uuid::new_v4();
        for _ in 0..SUBMIT_PER_HOUR {
            assert!(check_user_rate_limit(user, UserRateLimitAction::SubmitTool).is_ok());
        }
        assert!(check_user_rate_limit(user, UserRateLimitAction::SubmitTool).is_err());
    }

    #[test]
    fn comment_limit_allows_ten_then_blocks() {
        let user = Uuid::new_v4();
        for _ in 0..COMMENT_PER_MINUTE {
            assert!(check_user_rate_limit(user, UserRateLimitAction::CreateComment).is_ok());
        }
        assert!(check_user_rate_limit(user, UserRateLimitAction::CreateComment).is_err());
    }

    #[test]
    fn bookmark_limit_allows_sixty_then_blocks() {
        let user = Uuid::new_v4();
        for _ in 0..BOOKMARK_PER_MINUTE {
            assert!(check_user_rate_limit(user, UserRateLimitAction::ToggleBookmark).is_ok());
        }
        assert!(check_user_rate_limit(user, UserRateLimitAction::ToggleBookmark).is_err());
    }

    #[test]
    fn mcp_ip_limit_isolated_per_ip() {
        let ip_a = "203.0.113.10".to_string();
        let ip_b = "203.0.113.11".to_string();
        for _ in 0..MCP_PER_MINUTE {
            assert!(check_mcp_ip_rate_limit(&ip_a).is_ok());
        }
        assert!(check_mcp_ip_rate_limit(&ip_a).is_err());
        assert!(check_mcp_ip_rate_limit(&ip_b).is_ok());
    }

    #[test]
    fn client_ip_prefers_forwarded_header() {
        let request = axum::http::Request::builder()
            .header("x-forwarded-for", "198.51.100.7, 10.0.0.1")
            .body(())
            .expect("request");
        let (parts, _) = request.into_parts();
        assert_eq!(client_ip_from_parts(&parts), "198.51.100.7");
    }
}
