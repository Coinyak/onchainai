//! Server-side modules: Axum handlers, Leptos server functions, MCP handler.

pub mod functions;

#[cfg(feature = "ssr")]
pub mod mcp;
#[cfg(feature = "ssr")]
pub mod queries;