//! MCP Bearer resolution for Agent Sync tools.

use sqlx::PgPool;

use crate::server::agent_sync::{resolve_bearer, AgentAuth};

pub async fn agent_from_authorization(
    pool: &PgPool,
    authorization: Option<&str>,
) -> Option<AgentAuth> {
    resolve_bearer(pool, authorization).await
}
