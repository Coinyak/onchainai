//! Agent Sync — token mint, device flow, toolkit upsert for coding-tool clients.

mod blueprint;
mod device;
mod sync_tool;
mod tokens;
mod types;

pub use blueprint::{agent_session_title, save_stack_to_blueprint, sync_blueprint_node};
pub use device::{device_approve, device_poll, device_start};
pub use sync_tool::{link_required_payload, sync_tool};
pub use tokens::{
    count_active_tokens, has_active_link, hash_token, list_tokens, mint_token, resolve_bearer,
    revoke_token,
};
pub use types::*;

#[cfg(test)]
mod tests {
    use super::blueprint::{next_agent_tool_node_coords, slug_on_canvas};
    use super::tokens::hash_token;
    use super::types::TOKEN_PREFIX;
    use serde_json::json;

    #[test]
    fn hash_token_is_stable_hex() {
        let h = hash_token("oai_ag_test");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn token_prefix_constant() {
        assert!(TOKEN_PREFIX.starts_with("oai_ag_"));
    }

    #[test]
    fn next_agent_tool_node_coords_stacks_below_last_tool() {
        let nodes = json!([
            {"id": "a", "kind": "tool", "slug": "foo", "x": 40, "y": 40},
            {"id": "b", "kind": "note", "text": "hi", "x": 200, "y": 200}
        ]);
        let (x, y) = next_agent_tool_node_coords(&nodes);
        assert_eq!(x, 40);
        assert_eq!(y, 112);
    }

    #[test]
    fn next_agent_tool_node_coords_defaults_when_empty() {
        let (x, y) = next_agent_tool_node_coords(&json!([]));
        assert_eq!(x, 40);
        assert_eq!(y, 40);
    }

    #[test]
    fn slug_on_canvas_detects_tool_slug() {
        let nodes = json!([{"id": "a", "kind": "tool", "slug": "foo", "x": 0, "y": 0}]);
        assert!(slug_on_canvas(&nodes, "foo"));
        assert!(!slug_on_canvas(&nodes, "bar"));
    }
}
