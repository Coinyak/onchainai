//! GitHub source crawler — stub.
//!
//! Topics search, star sync, and self-register added in crawler milestone.

pub async fn run_once(_pool: &sqlx::PgPool) {
    tracing::info!("github topics crawl stub — not yet implemented");
}

/// Sync GitHub stars for existing tools (every 30min).
pub async fn sync_stars(_pool: &sqlx::PgPool) {
    tracing::info!("github star sync stub — not yet implemented");
}
