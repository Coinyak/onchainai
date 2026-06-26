//! Server-side modules: Axum handlers, Leptos server functions, MCP handler.

pub mod functions;

#[cfg(feature = "ssr")]
pub mod mcp;
#[cfg(feature = "ssr")]
pub mod operator_harness;
#[cfg(feature = "ssr")]
pub mod queries;
#[cfg(feature = "ssr")]
pub mod rate_limit;
pub mod secret_redaction;
