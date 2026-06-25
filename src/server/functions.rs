//! Leptos server functions — public API used by pages and components.
//!
//! These functions are auto-registered by the Leptos runtime and are
//! available to both server-rendered and hydrated components. Placeholder
//! endpoints for the website-core milestone; implementations will be filled
//! in by subsequent feature work.

use leptos::prelude::*;

/// Returns the most recently added tools.
#[server(GetRecentTools, "/api")]
pub async fn get_recent_tools(_limit: i64) -> Result<Vec<crate::models::Tool>, ServerFnError> {
    // Placeholder: will query the DB in the home-page feature.
    Ok(vec![])
}

/// Returns all categories with tool counts.
#[server(GetCategories, "/api")]
pub async fn get_categories() -> Result<Vec<(crate::models::Category, i64)>, ServerFnError> {
    // Placeholder: will query the DB in the home-page feature.
    Ok(vec![])
}

/// Searches tools using Postgres full-text search.
#[server(SearchTools, "/api")]
pub async fn search_tools(
    _query: String,
    _function: Option<String>,
    _chain: Option<String>,
) -> Result<Vec<crate::models::Tool>, ServerFnError> {
    // Placeholder: will query the DB in the tools-list feature.
    Ok(vec![])
}
