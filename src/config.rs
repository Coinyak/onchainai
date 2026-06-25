//! Configuration — environment variable loading and DB pool setup.
//!
//! All required env vars are loaded at startup via [`Config::from_env`].
//! Missing required vars produce clear [`anyhow`] errors with the var name.
//! The Postgres connection pool is built via [`setup_db`] using
//! [`PgPoolOptions`](sqlx::postgres::PgPoolOptions).

use std::env;

/// Canonical production hostname (no scheme).
pub const CANONICAL_DOMAIN: &str = "www.onchain-ai.xyz";

/// Canonical site origin for CORS, OAuth, and SIWX URI binding.
pub const SITE_ORIGIN: &str = "https://www.onchain-ai.xyz";

/// Default MCP install command shown in UI and site settings.
pub const MCP_ENDPOINT_CMD: &str = "npx mcp-remote www.onchain-ai.xyz/mcp";

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Postgres connection string (Supabase).
    pub database_url: String,
    /// Supabase project URL (for Auth).
    pub supabase_url: String,
    /// Supabase anon public key.
    pub supabase_anon_key: String,
    /// Supabase service role key (server only — never expose to client).
    pub supabase_service_key: String,
    /// GitHub OAuth client id (Supabase provider).
    pub github_client_id: String,
    /// GitHub OAuth client secret (server only).
    pub github_client_secret: String,
    /// SIWX domain bound to signed messages.
    pub siwx_domain: String,
    /// SIWX session TTL in seconds.
    pub siwx_session_ttl: i64,
    /// JWT signing/verification secret (server only).
    pub jwt_secret: String,
    /// Optional GitHub personal access token for crawler star sync.
    pub github_api_token: Option<String>,
    /// HTTP server bind port.
    pub port: u16,
}

impl Config {
    /// Load configuration from environment variables via `dotenvy`.
    ///
    /// Required variables that are missing or empty produce an error
    /// naming the variable so the operator knows what to fill in.
    pub fn from_env() -> anyhow::Result<Self> {
        // dotenvy is invoked in main before this; re-loading is harmless.
        let _ = dotenvy::dotenv();

        let database_url = required("DATABASE_URL")?;
        let supabase_url = required("SUPABASE_URL")?;
        let supabase_anon_key = required("SUPABASE_ANON_KEY")?;
        let supabase_service_key = required("SUPABASE_SERVICE_KEY")?;
        let github_client_id = required("GITHUB_CLIENT_ID")?;
        let github_client_secret = required("GITHUB_CLIENT_SECRET")?;
        let siwx_domain = required("SIWX_DOMAIN")?;
        let jwt_secret = required("JWT_SECRET")?;

        let siwx_session_ttl = env::var("SIWX_SESSION_TTL")
            .ok()
            .map(|s| s.parse::<i64>())
            .transpose()
            .map_err(|e| anyhow::anyhow!("SIWX_SESSION_TTL is not a valid integer: {e}"))?
            .unwrap_or(86_400);

        let github_api_token = env::var("GITHUB_API_TOKEN").ok().filter(|s| !s.is_empty());

        let port = env::var("PORT")
            .ok()
            .map(|s| s.parse::<u16>())
            .transpose()
            .map_err(|e| anyhow::anyhow!("PORT is not a valid u16: {e}"))?
            .unwrap_or(3000);

        Ok(Self {
            database_url,
            supabase_url,
            supabase_anon_key,
            supabase_service_key,
            github_client_id,
            github_client_secret,
            siwx_domain,
            siwx_session_ttl,
            jwt_secret,
            github_api_token,
            port,
        })
    }
}

/// Read a required environment variable, erroring if missing or empty.
fn required(key: &str) -> anyhow::Result<String> {
    env::var(key)
        .map(|v| v.trim().to_owned())
        .map_err(|_| anyhow::anyhow!("missing required environment variable: {key}"))
        .and_then(|v| {
            if v.is_empty() {
                Err(anyhow::anyhow!(
                    "environment variable {key} is set but empty"
                ))
            } else {
                Ok(v)
            }
        })
}

/// Initialize the Postgres connection pool from `database_url`.
///
/// Uses [`PgPoolOptions`](sqlx::postgres::PgPoolOptions) with a modest
/// connection limit. Returns an error (not a panic) on a bad URL.
pub async fn setup_db(database_url: &str) -> anyhow::Result<sqlx::PgPool> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .map_err(|e| anyhow::anyhow!("failed to connect to Postgres: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_domain_constants() {
        assert_eq!(CANONICAL_DOMAIN, "www.onchain-ai.xyz");
        assert_eq!(SITE_ORIGIN, "https://www.onchain-ai.xyz");
        assert!(MCP_ENDPOINT_CMD.contains("www.onchain-ai.xyz"));
        assert!(!MCP_ENDPOINT_CMD.contains("onchainai.xyz"));
    }

    #[test]
    fn missing_required_var_produces_error() {
        // Ensure the var is unset for this test.
        unsafe { env::remove_var("ONCHAINAI_TEST_MISSING_VAR") }
        let res = required("ONCHAINAI_TEST_MISSING_VAR");
        assert!(res.is_err());
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("ONCHAINAI_TEST_MISSING_VAR"),
            "error should name the missing var: {msg}"
        );
    }

    #[test]
    fn empty_required_var_produces_error() {
        unsafe { env::set_var("ONCHAINAI_TEST_EMPTY_VAR", "") }
        let res = required("ONCHAINAI_TEST_EMPTY_VAR");
        assert!(res.is_err());
        unsafe { env::remove_var("ONCHAINAI_TEST_EMPTY_VAR") }
    }

    #[test]
    fn present_required_var_returns_trimmed() {
        unsafe { env::set_var("ONCHAINAI_TEST_PRESENT_VAR", "  hello  ") }
        let res = required("ONCHAINAI_TEST_PRESENT_VAR");
        assert_eq!(res.unwrap(), "hello");
        unsafe { env::remove_var("ONCHAINAI_TEST_PRESENT_VAR") }
    }

    #[tokio::test]
    async fn setup_db_rejects_invalid_url() {
        // An invalid URL must produce an error, not a panic.
        let res = setup_db("not-a-valid-postgres-url").await;
        assert!(
            res.is_err(),
            "setup_db should return Err for an invalid URL"
        );
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("Postgres"),
            "error should mention Postgres: {msg}"
        );
    }

    #[tokio::test]
    async fn setup_db_rejects_unreachable_host() {
        // A syntactically valid URL pointing at nothing should fail quickly.
        // Use a short acquire timeout so the test does not hang.
        let res = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_millis(200))
            .connect("postgresql://nobody@127.0.0.1:1/none")
            .await;
        assert!(res.is_err(), "connecting to a dead port should fail");
    }
}
