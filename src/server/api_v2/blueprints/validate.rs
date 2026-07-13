//! Blueprint graph validation and node/edge normalization.
use serde_json::Value;

use super::types::*;
use super::super::error::ApiError;

pub(crate) fn clamp_dim(value: Option<i32>, min: i32, max: i32) -> Option<i32> {
    value.map(|v| v.clamp(min, max))
}

/// Clamp an optional 1-based step badge into [1, NODE_MAX_STEP]; drop non-positive.
pub(crate) fn normalize_step(value: Option<i32>) -> Option<i32> {
    value.filter(|v| *v >= 1).map(|v| v.min(NODE_MAX_STEP))
}

/// Normalize a steps array: dedupe, sort, clamp to [1, NODE_MAX_STEP], cap at max per node.
pub(crate) fn normalize_steps(values: Option<&[i32]>) -> Vec<i32> {
    let Some(values) = values else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for &v in values {
        if v < 1 {
            continue;
        }
        let clamped = v.min(NODE_MAX_STEP);
        if seen.insert(clamped) {
            result.push(clamped);
        }
        if result.len() >= NODE_MAX_STEPS_PER_NODE {
            break;
        }
    }
    result.sort();
    result
}

/// Merge legacy `step` (single) into `steps` (array) for backward compatibility.
pub(crate) fn merge_step_and_steps(step: Option<i32>, steps: Option<&[i32]>) -> Vec<i32> {
    let mut merged = normalize_steps(steps);
    if let Some(s) = normalize_step(step) {
        if !merged.contains(&s) {
            merged.push(s);
            merged.sort();
            merged.truncate(NODE_MAX_STEPS_PER_NODE);
        }
    }
    merged
}

/// Trim an optional edge label and cap its length; `None`/empty -> `None`.
pub(crate) fn normalize_edge_label(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(MAX_EDGE_LABEL_LEN).collect())
}

/// Attach clamped custom width/height to a node payload when provided.
pub(crate) fn apply_node_size(payload: &mut Value, w: Option<i32>, h: Option<i32>) {
    if let Some(w) = clamp_dim(w, NODE_MIN_W, NODE_MAX_W) {
        payload["w"] = serde_json::json!(w);
    }
    if let Some(h) = clamp_dim(h, NODE_MIN_H, NODE_MAX_H) {
        payload["h"] = serde_json::json!(h);
    }
}

/// Attach a normalized steps array to a node payload when provided.
pub(crate) fn apply_node_steps(payload: &mut Value, steps: &[i32]) {
    if !steps.is_empty() {
        payload["steps"] = serde_json::json!(steps);
    }
}

pub(crate) fn normalize_tool_node_chains(
    chains: Option<&[String]>,
    idx: usize,
) -> Result<Vec<String>, ApiError> {
    let Some(values) = chains else {
        return Ok(Vec::new());
    };
    if values.len() > MAX_TOOL_NODE_CHAINS {
        return Err(ApiError::BadRequest(format!(
            "tool node at index {idx} accepts at most {MAX_TOOL_NODE_CHAINS} chains"
        )));
    }

    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw in values {
        let chain_id = raw.trim().to_lowercase();
        if chain_id.is_empty() {
            continue;
        }
        if chain_id.chars().count() > MAX_CHAIN_ID_LEN {
            return Err(ApiError::BadRequest(format!(
                "tool node chain id must be at most {MAX_CHAIN_ID_LEN} characters"
            )));
        }
        if !chain_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(ApiError::BadRequest(
                "tool node chain id contains invalid characters".into(),
            ));
        }
        if seen.insert(chain_id.clone()) {
            normalized.push(chain_id);
        }
    }
    Ok(normalized)
}

pub(crate) fn validate_title(title: &str) -> Result<String, ApiError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Ok("Untitled blueprint".into());
    }
    if trimmed.chars().count() > MAX_TITLE_LEN {
        return Err(ApiError::BadRequest(format!(
            "blueprint title must be at most {MAX_TITLE_LEN} characters"
        )));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn validate_nodes(nodes: &Value) -> Result<Value, ApiError> {
    let arr = nodes
        .as_array()
        .ok_or_else(|| ApiError::BadRequest("nodes must be a JSON array".into()))?;

    if arr.len() > MAX_NODES {
        return Err(ApiError::BadRequest(format!(
            "blueprints accept at most {MAX_NODES} nodes"
        )));
    }

    let mut normalized = Vec::with_capacity(arr.len());
    for (idx, item) in arr.iter().enumerate() {
        let node: BlueprintNodeInput = serde_json::from_value(item.clone())
            .map_err(|e| ApiError::BadRequest(format!("invalid node at index {idx}: {e}")))?;

        if node.id.trim().is_empty() {
            return Err(ApiError::BadRequest(format!(
                "node at index {idx} requires a non-empty id"
            )));
        }

        if !(0..=COORD_MAX).contains(&node.x) || !(0..=COORD_MAX).contains(&node.y) {
            return Err(ApiError::BadRequest(format!(
                "node coordinates must be between 0 and {COORD_MAX}"
            )));
        }

        match node.kind.as_str() {
            "tool" => {
                let slug = node.slug.as_deref().unwrap_or("").trim();
                if slug.is_empty() {
                    return Err(ApiError::BadRequest(format!(
                        "tool node at index {idx} requires a slug"
                    )));
                }
                let chains = normalize_tool_node_chains(node.chains.as_deref(), idx)?;
                let steps = merge_step_and_steps(node.step, node.steps.as_deref());
                let mut payload = serde_json::json!({
                    "id": node.id,
                    "kind": "tool",
                    "slug": slug,
                    "x": node.x,
                    "y": node.y,
                });
                if !chains.is_empty() {
                    payload["chains"] = serde_json::json!(chains);
                }
                apply_node_size(&mut payload, node.w, node.h);
                apply_node_steps(&mut payload, &steps);
                normalized.push(payload);
            }
            "note" => {
                let text = node.text.unwrap_or_default();
                if text.chars().count() > MAX_NOTE_TEXT {
                    return Err(ApiError::BadRequest(format!(
                        "note text must be at most {MAX_NOTE_TEXT} characters"
                    )));
                }
                let steps = merge_step_and_steps(node.step, node.steps.as_deref());
                let mut payload = serde_json::json!({
                    "id": node.id,
                    "kind": "note",
                    "text": text,
                    "x": node.x,
                    "y": node.y,
                });
                apply_node_size(&mut payload, node.w, node.h);
                apply_node_steps(&mut payload, &steps);
                normalized.push(payload);
            }
            "chain" => {
                let chain_id = node.chain_id.as_deref().unwrap_or("").trim();
                if chain_id.is_empty() {
                    return Err(ApiError::BadRequest(format!(
                        "chain node at index {idx} requires a chainId"
                    )));
                }
                if chain_id.chars().count() > MAX_CHAIN_ID_LEN {
                    return Err(ApiError::BadRequest(format!(
                        "chainId must be at most {MAX_CHAIN_ID_LEN} characters"
                    )));
                }
                let steps = merge_step_and_steps(node.step, node.steps.as_deref());
                let mut payload = serde_json::json!({
                    "id": node.id,
                    "kind": "chain",
                    "chainId": chain_id,
                    "x": node.x,
                    "y": node.y,
                });
                apply_node_steps(&mut payload, &steps);
                normalized.push(payload);
            }
            other => {
                return Err(ApiError::BadRequest(format!(
                    "node kind must be 'tool', 'note', or 'chain', got '{other}'"
                )));
            }
        }
    }

    Ok(Value::Array(normalized))
}

pub(crate) fn is_valid_edge_color(color: &str) -> bool {
    let color = color.trim();
    if color.len() != 7 || !color.starts_with('#') {
        return false;
    }
    color[1..].chars().all(|c| c.is_ascii_hexdigit())
}

pub(crate) fn validate_edges(edges: &Value, node_ids: &[String]) -> Result<Value, ApiError> {
    let arr = edges
        .as_array()
        .ok_or_else(|| ApiError::BadRequest("edges must be a JSON array".into()))?;

    if arr.len() > MAX_EDGES {
        return Err(ApiError::BadRequest(format!(
            "blueprints accept at most {MAX_EDGES} edges"
        )));
    }

    let node_set: std::collections::HashSet<&str> = node_ids.iter().map(String::as_str).collect();
    let mut normalized = Vec::with_capacity(arr.len());

    for (idx, item) in arr.iter().enumerate() {
        let edge: BlueprintEdgeInput = serde_json::from_value(item.clone())
            .map_err(|e| ApiError::BadRequest(format!("invalid edge at index {idx}: {e}")))?;

        if edge.id.trim().is_empty() {
            return Err(ApiError::BadRequest(format!(
                "edge at index {idx} requires a non-empty id"
            )));
        }

        let from_id = edge.from_id.trim();
        let to_id = edge.to_id.trim();
        if from_id.is_empty() || to_id.is_empty() {
            return Err(ApiError::BadRequest(format!(
                "edge at index {idx} requires fromId and toId"
            )));
        }
        if from_id == to_id {
            return Err(ApiError::BadRequest(format!(
                "edge at index {idx} cannot connect a node to itself"
            )));
        }
        if !node_set.contains(from_id) || !node_set.contains(to_id) {
            return Err(ApiError::BadRequest(format!(
                "edge at index {idx} references unknown nodes"
            )));
        }

        let style = edge.style.trim();
        if style != "solid" && style != "arrow" {
            return Err(ApiError::BadRequest(format!(
                "edge style must be 'solid' or 'arrow', got '{style}'"
            )));
        }

        if !is_valid_edge_color(&edge.color) {
            return Err(ApiError::BadRequest(format!(
                "edge at index {idx} requires a #RRGGBB color"
            )));
        }

        let mut payload = serde_json::json!({
            "id": edge.id,
            "fromId": from_id,
            "toId": to_id,
            "style": style,
            "color": edge.color.trim().to_ascii_uppercase(),
        });
        if edge.dashed.unwrap_or(false) {
            payload["dashed"] = serde_json::json!(true);
        }
        if let Some(label) = normalize_edge_label(edge.label.as_deref()) {
            payload["label"] = serde_json::json!(label);
        }
        normalized.push(payload);
    }

    Ok(Value::Array(normalized))
}

pub(crate) fn node_ids_from_value(nodes: &Value) -> Result<Vec<String>, ApiError> {
    let arr = nodes
        .as_array()
        .ok_or_else(|| ApiError::BadRequest("nodes must be a JSON array".into()))?;
    Ok(arr
        .iter()
        .filter_map(|item| item.get("id").and_then(|v| v.as_str()).map(str::to_string))
        .collect())
}

pub(crate) fn prune_edges_for_nodes(edges: &Value, node_ids: &[String]) -> Result<Value, ApiError> {
    let node_set: std::collections::HashSet<&str> = node_ids.iter().map(String::as_str).collect();
    let arr = edges
        .as_array()
        .ok_or_else(|| ApiError::BadRequest("edges must be a JSON array".into()))?;
    let pruned: Vec<Value> = arr
        .iter()
        .filter(|item| {
            let from_ok = item
                .get("fromId")
                .and_then(|v| v.as_str())
                .is_some_and(|id| node_set.contains(id));
            let to_ok = item
                .get("toId")
                .and_then(|v| v.as_str())
                .is_some_and(|id| node_set.contains(id));
            from_ok && to_ok
        })
        .cloned()
        .collect();
    Ok(Value::Array(pruned))
}
