//! Blueprints REST API (`/api/v2/blueprints`).

mod access;
mod export;
mod handlers;
mod types;
mod validate;

pub use handlers::router;

// Bring helpers into this module so unit tests can call them via `super::*`.
#[cfg(test)]
use export::{
    build_agent_export_markdown, build_flow_section, build_order_section, collect_tool_slugs,
    parse_export_nodes,
};
#[cfg(test)]
use types::{
    ExportNode, ToolExportMeta, MAX_TITLE_LEN, NODE_MAX_STEP, NODE_MAX_W, NODE_MIN_H, NODE_MIN_W,
};
#[cfg(test)]
use validate::{node_ids_from_value, validate_edges, validate_nodes, validate_title};

#[cfg(test)]
mod tests;
