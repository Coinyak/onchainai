//! Session cookies and JWT verification for Supabase access tokens.

use leptos::server_fn::ServerFnError;
use uuid::Uuid;

pub const ACCESS_TOKEN_COOKIE: &str = "onchainai_access_token";
pub const PKCE_VERIFIER_COOKIE: &str = "onchainai_pkce_verifier";
pub const GITHUB_STATE_COOKIE: &str = "onchainai_github_state";

/// True when auth cookies must include `Secure` (production HTTPS).
pub fn cookie_secure_for_domain(siwx_domain: &str) -> bool {
    !siwx_domain.contains("localhost")
}

/// Whether the SSR shell will inject the WASM hydration bundle.
///
/// Matches [`crate::app::shell`] so wallet buttons can fall back to plain
/// links when interactive handlers are unavailable.
pub fn ssr_hydration_available() -> bool {
    #[cfg(feature = "ssr")]
    {
        let bundle = std::path::Path::new("target/site/pkg/onchainai.js").exists();
        match std::env::var("LEPTOS_HYDRATION").as_deref() {
            Ok("0") | Ok("false") | Ok("FALSE") => false,
            Ok("1") | Ok("true") | Ok("TRUE") => bundle,
            _ => bundle,
        }
    }
    #[cfg(not(feature = "ssr"))]
    {
        cfg!(feature = "hydrate")
    }
}

/// Authenticated user resolved from JWT + profiles row.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionUser {
    pub id: Uuid,
    pub nickname: Option<String>,
    pub is_admin: bool,
    pub auth_method: String,
}

/// Parse `Cookie` header value and extract the named cookie.
pub fn cookie_value<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    cookie_header.split(';').find_map(|part| {
        let part = part.trim();
        let (key, value) = part.split_once('=')?;
        if key == name {
            Some(value)
        } else {
            None
        }
    })
}

#[derive(Debug)]
pub enum AuthSessionError {
    InvalidToken,
    ProfileMissing,
    Banned,
    Database(String),
}

impl std::fmt::Display for AuthSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "invalid session"),
            Self::ProfileMissing => write!(f, "profile not found"),
            Self::Banned => write!(f, "account suspended"),
            Self::Database(msg) => write!(f, "database error: {msg}"),
        }
    }
}

impl From<AuthSessionError> for ServerFnError {
    fn from(e: AuthSessionError) -> Self {
        ServerFnError::new(e.to_string())
    }
}

/// Maps session resolution for optional reads (`get_current_user`, `is_bookmarked`, …).
/// Invalid, expired, or missing sessions become `Ok(None)` instead of API errors.
pub fn optional_session_result(
    result: Result<Option<SessionUser>, AuthSessionError>,
) -> Result<Option<SessionUser>, ServerFnError> {
    match result {
        Ok(user) => Ok(user),
        Err(AuthSessionError::Database(msg)) => Err(ServerFnError::new(msg)),
        Err(_) => Ok(None),
    }
}

#[cfg(feature = "ssr")]
#[path = "session_ssr.rs"]
mod session_ssr;

#[cfg(feature = "ssr")]
pub use session_ssr::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_cookie() {
        let header = "foo=bar; onchainai_access_token=abc123; baz=qux";
        assert_eq!(cookie_value(header, ACCESS_TOKEN_COOKIE), Some("abc123"));
        assert_eq!(cookie_value(header, "missing"), None);
    }

    #[test]
    fn optional_session_result_treats_invalid_token_as_anonymous() {
        let out = optional_session_result(Err(AuthSessionError::InvalidToken)).expect("no error");
        assert!(out.is_none());
    }

    #[test]
    fn optional_session_result_propagates_database_errors() {
        let err = optional_session_result(Err(AuthSessionError::Database("down".into())))
            .expect_err("db error");
        assert!(err.to_string().contains("down"));
    }

    #[test]
    fn cookie_secure_for_production_domain() {
        assert!(cookie_secure_for_domain("www.onchain-ai.xyz"));
        assert!(!cookie_secure_for_domain("localhost:3000"));
    }
}
