//! MCP server (rmcp handler) — stub.
//!
//! The 4-tool MCP server is implemented in the mcp-server milestone.
//! When `search_tools` / `get_tool_detail` / `list_categories` are wired here,
//! they MUST filter with `crate::server::queries::TOOLS_APPROVED_WHERE` so only
//! publicly approved tools are exposed to agents (same bar as Leptos server fns).