//! Server-side authorization helpers (admin routes and mutations).

use crate::auth::session::{session_from_parts, SessionUser};
use leptos::server_fn::ServerFnError;
use sqlx::PgPool;

/// Authorization failure — map to generic messages (no IDOR hints).
#[derive(Debug)]
pub enum AuthError {
    Unauthorized,
    Forbidden,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(f, "sign in required"),
            Self::Forbidden => write!(f, "not found"),
        }
    }
}

impl From<AuthError> for ServerFnError {
    fn from(e: AuthError) -> Self {
        ServerFnError::new(e.to_string())
    }
}

/// Require an authenticated admin session. Returns the session user on success.
pub async fn require_admin(
    parts: &axum::http::request::Parts,
    pool: &PgPool,
    jwt_secret: &str,
    issuer: &str,
) -> Result<SessionUser, AuthError> {
    let user = session_from_parts(parts, pool, jwt_secret, issuer)
        .await
        .map_err(|_| AuthError::Forbidden)?
        .ok_or(AuthError::Forbidden)?;

    if !user.is_admin {
        return Err(AuthError::Forbidden);
    }

    Ok(user)
}

/// Require any authenticated session (social mutations, bookmarks, etc.).
pub async fn require_user(
    parts: &axum::http::request::Parts,
    pool: &PgPool,
    jwt_secret: &str,
    issuer: &str,
) -> Result<SessionUser, AuthError> {
    session_from_parts(parts, pool, jwt_secret, issuer)
        .await
        .map_err(|_| AuthError::Unauthorized)?
        .ok_or(AuthError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_error_messages_are_generic() {
        assert_eq!(AuthError::Forbidden.to_string(), "not found");
    }
}
