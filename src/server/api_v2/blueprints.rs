//! Stack Blueprint endpoints — authenticated, owner-scoped.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use std::collections::{HashMap, HashSet};

use crate::config::SITE_ORIGIN;
use crate::AppState;

use super::auth::require_user_from;
use super::error::ApiError;

const MAX_BLUEPRINTS_PER_USER: i64 = 20;
const MAX_NODES: usize = 120;
const MAX_EDGES: usize = 120;
const COORD_MAX: i32 = 4000;
const MAX_NOTE_TEXT: usize = 2000;
const MAX_TITLE_LEN: usize = 200;
const MAX_CHAIN_ID_LEN: usize = 64;
const MAX_TOOL_NODE_CHAINS: usize = 8;
const MAX_EDGE_LABEL_LEN: usize = 40;
const NODE_MIN_W: i32 = 160;
const NODE_MAX_W: i32 = 520;
const NODE_MIN_H: i32 = 72;
const NODE_MAX_H: i32 = 420;
const NODE_MAX_STEP: i32 = 99;
const NODE_MAX_STEPS_PER_NODE: usize = 8;
const AGENT_EXPORT_FILENAME: &str = "blueprint-agent.md";

const AGENT_EXPORT_TASK_TEMPLATE: &str = r#"## Your task

1. Read the attached blueprint PNG together with this prompt (export PNG separately from the editor Share dock).
2. For each slug in ## Tools, call OnchainAI MCP `get_install_guide` (platform: {platform}).
3. Summarize install risk; do not install critical-risk tools.
4. When ## Order is present, treat it as the owner's step sequence; otherwise follow ## Flow. If you edited Flow/Order, prefer the user's wording.
5. Ask before changing my toolkit or installing anything."#;

fn db_internal(action: &str, err: impl std::fmt::Display) -> ApiError {
    tracing::error!("blueprint {action} failed: {err}");
    ApiError::Internal(format!("blueprint {action} failed"))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v2/blueprints",
            get(list_blueprints).post(create_blueprint),
        )
        .route(
            "/api/v2/blueprints/{id}",
            get(get_blueprint)
                .put(update_blueprint)
                .delete(delete_blueprint),
        )
        .route(
            "/api/v2/blueprints/{id}/agent-export",
            get(agent_export_blueprint),
        )
        .with_state(state)
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct BlueprintRow {
    id: Uuid,
    user_id: Uuid,
    title: String,
    nodes: Value,
    edges: Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct BlueprintListRow {
    id: Uuid,
    title: String,
    node_count: i32,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
struct BlueprintView {
    id: Uuid,
    title: String,
    nodes: Value,
    edges: Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateBlueprintBody {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    nodes: Option<Value>,
    #[serde(default)]
    edges: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct UpdateBlueprintBody {
    title: Option<String>,
    nodes: Option<Value>,
    edges: Option<Value>,
}

#[derive(Debug, Serialize)]
struct AgentExportResponse {
    title: String,
    markdown: String,
    slugs: Vec<String>,
    filename: String,
}

#[derive(Debug, Clone)]
struct ExportNode {
    id: String,
    kind: String,
    slug: Option<String>,
    chain_id: Option<String>,
    text: Option<String>,
    chains: Vec<String>,
    steps: Vec<i32>,
}

#[derive(Debug, Clone)]
struct ToolExportMeta {
    name: String,
    install_risk_level: String,
}

#[derive(Debug, Deserialize)]
struct BlueprintNodeInput {
    id: String,
    kind: String,
    slug: Option<String>,
    #[serde(rename = "chainId")]
    chain_id: Option<String>,
    text: Option<String>,
    #[serde(default)]
    chains: Option<Vec<String>>,
    x: i32,
    y: i32,
    #[serde(default)]
    w: Option<i32>,
    #[serde(default)]
    h: Option<i32>,
    #[serde(default)]
    step: Option<i32>,
    #[serde(default)]
    steps: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
struct BlueprintEdgeInput {
    id: String,
    #[serde(rename = "fromId")]
    from_id: String,
    #[serde(rename = "toId")]
    to_id: String,
    style: String,
    color: String,
    #[serde(default)]
    dashed: Option<bool>,
    #[serde(default)]
    label: Option<String>,
}

/// Clamp an optional dimension into [min, max]; `None` stays `None` (default size).
fn clamp_dim(value: Option<i32>, min: i32, max: i32) -> Option<i32> {
    value.map(|v| v.clamp(min, max))
}

/// Clamp an optional 1-based step badge into [1, NODE_MAX_STEP]; drop non-positive.
fn normalize_step(value: Option<i32>) -> Option<i32> {
    value.filter(|v| *v >= 1).map(|v| v.min(NODE_MAX_STEP))
}

/// Normalize a steps array: dedupe, sort, clamp to [1, NODE_MAX_STEP], cap at max per node.
fn normalize_steps(values: Option<&[i32]>) -> Vec<i32> {
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
fn merge_step_and_steps(step: Option<i32>, steps: Option<&[i32]>) -> Vec<i32> {
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
fn normalize_edge_label(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(MAX_EDGE_LABEL_LEN).collect())
}

/// Attach clamped custom width/height to a node payload when provided.
fn apply_node_size(payload: &mut Value, w: Option<i32>, h: Option<i32>) {
    if let Some(w) = clamp_dim(w, NODE_MIN_W, NODE_MAX_W) {
        payload["w"] = serde_json::json!(w);
    }
    if let Some(h) = clamp_dim(h, NODE_MIN_H, NODE_MAX_H) {
        payload["h"] = serde_json::json!(h);
    }
}

/// Attach a normalized steps array to a node payload when provided.
fn apply_node_steps(payload: &mut Value, steps: &[i32]) {
    if !steps.is_empty() {
        payload["steps"] = serde_json::json!(steps);
    }
}

fn normalize_tool_node_chains(
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

fn validate_title(title: &str) -> Result<String, ApiError> {
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

fn validate_nodes(nodes: &Value) -> Result<Value, ApiError> {
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

fn is_valid_edge_color(color: &str) -> bool {
    let color = color.trim();
    if color.len() != 7 || !color.starts_with('#') {
        return false;
    }
    color[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn validate_edges(edges: &Value, node_ids: &[String]) -> Result<Value, ApiError> {
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

fn node_ids_from_value(nodes: &Value) -> Result<Vec<String>, ApiError> {
    let arr = nodes
        .as_array()
        .ok_or_else(|| ApiError::BadRequest("nodes must be a JSON array".into()))?;
    Ok(arr
        .iter()
        .filter_map(|item| item.get("id").and_then(|v| v.as_str()).map(str::to_string))
        .collect())
}

fn prune_edges_for_nodes(edges: &Value, node_ids: &[String]) -> Result<Value, ApiError> {
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

async fn list_blueprints(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<BlueprintListRow>>, ApiError> {
    let user = require_user_from(&state, &headers).await?;

    let rows = sqlx::query_as::<_, BlueprintListRow>(
        r#"
        SELECT
            id,
            title,
            COALESCE(jsonb_array_length(nodes), 0)::int AS node_count,
            updated_at
        FROM blueprints
        WHERE user_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| db_internal("list", e))?;

    Ok(Json(rows))
}

async fn create_blueprint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateBlueprintBody>,
) -> Result<Json<BlueprintView>, ApiError> {
    let user = require_user_from(&state, &headers).await?;

    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM blueprints WHERE user_id = $1")
        .bind(user.id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| db_internal("count", e))?;

    if count >= MAX_BLUEPRINTS_PER_USER {
        return Err(ApiError::BadRequest(format!(
            "you can save at most {MAX_BLUEPRINTS_PER_USER} blueprints"
        )));
    }

    let title = validate_title(body.title.as_deref().unwrap_or("Untitled blueprint"))?;
    let nodes = validate_nodes(&body.nodes.unwrap_or_else(|| Value::Array(vec![])))?;
    let node_ids = node_ids_from_value(&nodes)?;
    let edges = validate_edges(
        &body.edges.unwrap_or_else(|| Value::Array(vec![])),
        &node_ids,
    )?;

    let row = sqlx::query_as::<_, BlueprintRow>(
        r#"
        INSERT INTO blueprints (user_id, title, nodes, edges)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(user.id)
    .bind(&title)
    .bind(&nodes)
    .bind(&edges)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| db_internal("create", e))?;

    Ok(Json(row.into_view()))
}

async fn get_blueprint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<BlueprintView>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    let row = fetch_owned_blueprint(&state, id, user.id).await?;
    Ok(Json(row.into_view()))
}

async fn update_blueprint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateBlueprintBody>,
) -> Result<Json<BlueprintView>, ApiError> {
    let user = require_user_from(&state, &headers).await?;

    if body.title.is_none() && body.nodes.is_none() && body.edges.is_none() {
        return Err(ApiError::BadRequest(
            "at least one of title, nodes, or edges is required".into(),
        ));
    }

    let existing = fetch_owned_blueprint(&state, id, user.id).await?;

    let title = if let Some(t) = body.title {
        validate_title(&t)?
    } else {
        existing.title
    };

    let nodes_updated = body.nodes.is_some();
    let nodes = if let Some(n) = body.nodes {
        validate_nodes(&n)?
    } else {
        existing.nodes
    };

    let node_ids = node_ids_from_value(&nodes)?;
    let edges = if let Some(e) = body.edges {
        validate_edges(&e, &node_ids)?
    } else if nodes_updated {
        prune_edges_for_nodes(&existing.edges, &node_ids)?
    } else {
        existing.edges
    };

    let row = sqlx::query_as::<_, BlueprintRow>(
        r#"
        UPDATE blueprints
        SET title = $3, nodes = $4, edges = $5
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(user.id)
    .bind(&title)
    .bind(&nodes)
    .bind(&edges)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| db_internal("update", e))?
    .ok_or_else(|| ApiError::NotFound("blueprint not found".into()))?;

    Ok(Json(row.into_view()))
}

fn parse_export_nodes(nodes: &Value) -> Vec<ExportNode> {
    let Some(arr) = nodes.as_array() else {
        return Vec::new();
    };

    arr.iter()
        .filter_map(|item| {
            let id = item.get("id")?.as_str()?.trim().to_string();
            if id.is_empty() {
                return None;
            }
            let kind = item.get("kind")?.as_str()?.to_string();
            let slug = item
                .get("slug")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let chain_id = item
                .get("chainId")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let text = item
                .get("text")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let chains = item
                .get("chains")
                .and_then(|v| v.as_array())
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|chain| chain.as_str().map(str::trim))
                        .filter(|chain| !chain.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let steps: Vec<i32> = item
                .get("steps")
                .and_then(|v| v.as_array())
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|s| s.as_i64().map(|v| v as i32))
                        .filter(|s| *s >= 1)
                        .take(NODE_MAX_STEPS_PER_NODE)
                        .collect()
                })
                .unwrap_or_default();
            // Fall back to legacy `step` field
            let steps = if steps.is_empty() {
                item.get("step")
                    .and_then(|v| v.as_i64())
                    .map(|v| vec![v as i32])
                    .unwrap_or_default()
            } else {
                steps
            };

            Some(ExportNode {
                id,
                kind,
                slug,
                chain_id,
                text,
                chains,
                steps,
            })
        })
        .collect()
}

fn collect_tool_slugs(nodes: &Value) -> Vec<String> {
    let mut slugs = Vec::new();
    let mut seen = HashSet::new();
    for node in parse_export_nodes(nodes) {
        if node.kind == "tool" {
            if let Some(slug) = node.slug {
                if seen.insert(slug.clone()) {
                    slugs.push(slug);
                }
            }
        }
    }
    slugs
}

fn node_flow_label(node: &ExportNode) -> String {
    match node.kind.as_str() {
        "tool" => node.slug.clone().unwrap_or_else(|| node.id.clone()),
        "note" => {
            let text = node.text.as_deref().unwrap_or("").trim();
            if text.is_empty() {
                "note".into()
            } else if text.chars().count() > 48 {
                let truncated: String = text.chars().take(48).collect();
                format!("note: {truncated}…")
            } else {
                format!("note: {text}")
            }
        }
        "chain" => format!("chain: {}", node.chain_id.as_deref().unwrap_or("unknown")),
        _ => node.id.clone(),
    }
}

struct FlowEdge {
    from: String,
    to: String,
    label: Option<String>,
    dashed: bool,
}

fn flow_label_for(node_map: &HashMap<&str, &ExportNode>, id: &str) -> String {
    node_map
        .get(id)
        .map(|node| node_flow_label(node))
        .unwrap_or_else(|| id.to_string())
}

/// Follow a maximal simple path from `start_idx`, stopping at branch/merge
/// points (nodes whose in- or out-degree is not exactly 1) or a revisited edge.
fn walk_flow_segment(
    start_idx: usize,
    flow_edges: &[FlowEdge],
    out_edges: &HashMap<String, Vec<usize>>,
    in_deg: &HashMap<String, usize>,
    out_deg: &HashMap<String, usize>,
    node_map: &HashMap<&str, &ExportNode>,
    visited: &mut [bool],
) -> String {
    let mut line = flow_label_for(node_map, &flow_edges[start_idx].from);
    let mut cur = start_idx;
    loop {
        visited[cur] = true;
        let edge = &flow_edges[cur];
        if edge.dashed {
            match &edge.label {
                Some(label) => line.push_str(&format!(" →({label}, dashed) ")),
                None => line.push_str(" -[dashed]→ "),
            }
        } else {
            match &edge.label {
                Some(label) => line.push_str(&format!(" →({label}) ")),
                None => line.push_str(" → "),
            }
        }
        line.push_str(&flow_label_for(node_map, &edge.to));

        let internal = in_deg.get(&edge.to).copied().unwrap_or(0) == 1
            && out_deg.get(&edge.to).copied().unwrap_or(0) == 1;
        if internal {
            if let Some(next) = out_edges
                .get(&edge.to)
                .and_then(|list| list.first().copied())
            {
                if !visited[next] {
                    cur = next;
                    continue;
                }
            }
        }
        break;
    }
    line
}

fn build_flow_section(nodes: &[ExportNode], edges: &Value) -> String {
    let node_map: HashMap<&str, &ExportNode> =
        nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let edge_arr = edges.as_array().cloned().unwrap_or_default();

    // Keep only edges whose endpoints both resolve, preserving insertion order.
    let mut flow_edges: Vec<FlowEdge> = Vec::new();
    for edge in edge_arr {
        let from = edge
            .get("fromId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let to = edge
            .get("toId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if from.is_empty() || to.is_empty() {
            continue;
        }
        if !node_map.contains_key(from) || !node_map.contains_key(to) {
            continue;
        }
        let label = edge
            .get("label")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let dashed = edge
            .get("dashed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        flow_edges.push(FlowEdge {
            from: from.to_string(),
            to: to.to_string(),
            label,
            dashed,
        });
    }

    if flow_edges.is_empty() {
        return "(no flow edges defined)".into();
    }

    let mut in_deg: HashMap<String, usize> = HashMap::new();
    let mut out_deg: HashMap<String, usize> = HashMap::new();
    let mut out_edges: HashMap<String, Vec<usize>> = HashMap::new();
    for node in nodes {
        in_deg.entry(node.id.clone()).or_insert(0);
        out_deg.entry(node.id.clone()).or_insert(0);
    }
    for (idx, edge) in flow_edges.iter().enumerate() {
        *out_deg.entry(edge.from.clone()).or_insert(0) += 1;
        *in_deg.entry(edge.to.clone()).or_insert(0) += 1;
        in_deg.entry(edge.from.clone()).or_insert(0);
        out_deg.entry(edge.to.clone()).or_insert(0);
        out_edges.entry(edge.from.clone()).or_default().push(idx);
    }

    // Deterministic output: sort out-edges by target label, then id, then index.
    for indices in out_edges.values_mut() {
        indices.sort_by(|&a, &b| {
            flow_label_for(&node_map, &flow_edges[a].to)
                .cmp(&flow_label_for(&node_map, &flow_edges[b].to))
                .then_with(|| flow_edges[a].to.cmp(&flow_edges[b].to))
                .then_with(|| a.cmp(&b))
        });
    }

    let mut visited = vec![false; flow_edges.len()];
    let mut lines: Vec<String> = Vec::new();

    // 1. Segments that begin at a junction (source/sink/branch/merge).
    let mut junctions: Vec<String> = out_edges.keys().cloned().collect();
    junctions.sort_by(|a, b| {
        flow_label_for(&node_map, a)
            .cmp(&flow_label_for(&node_map, b))
            .then_with(|| a.cmp(b))
    });
    for from in &junctions {
        let is_junction = in_deg.get(from).copied().unwrap_or(0) != 1
            || out_deg.get(from).copied().unwrap_or(0) != 1;
        if !is_junction {
            continue;
        }
        let idx_list = out_edges.get(from).cloned().unwrap_or_default();
        for idx in idx_list {
            if !visited[idx] {
                lines.push(walk_flow_segment(
                    idx,
                    &flow_edges,
                    &out_edges,
                    &in_deg,
                    &out_deg,
                    &node_map,
                    &mut visited,
                ));
            }
        }
    }

    // 2. Remaining edges belong to pure cycles with no junction — emit them too.
    for idx in 0..flow_edges.len() {
        if !visited[idx] {
            lines.push(walk_flow_segment(
                idx,
                &flow_edges,
                &out_edges,
                &in_deg,
                &out_deg,
                &node_map,
                &mut visited,
            ));
        }
    }

    // 3. Nodes touching no edge, listed on their own for completeness.
    let touched: HashSet<&str> = flow_edges
        .iter()
        .flat_map(|edge| [edge.from.as_str(), edge.to.as_str()])
        .collect();
    let mut orphans: Vec<String> = nodes
        .iter()
        .filter(|node| !touched.contains(node.id.as_str()))
        .map(node_flow_label)
        .collect();
    orphans.sort();
    lines.extend(orphans);

    if lines.is_empty() {
        "(no flow edges defined)".into()
    } else {
        lines
            .iter()
            .map(|line| format!("- {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn build_order_section(nodes: &[ExportNode]) -> String {
    // Expand each node's steps into (node, step) pairs, then sort globally.
    let mut stepped: Vec<(&ExportNode, i32)> = Vec::new();
    for node in nodes {
        for &step in &node.steps {
            stepped.push((node, step));
        }
    }
    if stepped.is_empty() {
        return String::new();
    }
    stepped.sort_by(|(a, step_a), (b, step_b)| step_a.cmp(step_b).then_with(|| a.id.cmp(&b.id)));
    stepped
        .into_iter()
        .map(|(node, step)| format!("- {step}. {} ({})", node_flow_label(node), node.kind))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_agent_export_markdown(
    title: &str,
    nodes: &Value,
    edges: &Value,
    tool_meta: &HashMap<String, ToolExportMeta>,
    platform: &str,
) -> String {
    let export_nodes = parse_export_nodes(nodes);
    let mut markdown = format!("# {title}\n\n");
    markdown.push_str(&format!(
        "Read the attached blueprint image together with this prompt. \
For each tool below, call OnchainAI MCP `get_install_guide` (platform: {platform}) \
before installing.\n\n",
    ));

    markdown.push_str("## Tools\n\n");
    let mut tool_nodes: Vec<(usize, &ExportNode)> = export_nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.kind == "tool")
        .collect();
    tool_nodes.sort_by(|(idx_a, a), (idx_b, b)| {
        let a_min = a.steps.first().copied();
        let b_min = b.steps.first().copied();
        match (a_min, b_min) {
            (Some(step_a), Some(step_b)) => step_a.cmp(&step_b).then(idx_a.cmp(idx_b)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => idx_a.cmp(idx_b),
        }
    });
    if tool_nodes.is_empty() {
        markdown.push_str("(none)\n\n");
    } else {
        for (_, node) in tool_nodes {
            let slug = node.slug.as_deref().unwrap_or("unknown");
            let display_name = tool_meta
                .get(slug)
                .map(|meta| meta.name.as_str())
                .unwrap_or(slug);
            let chains = if node.chains.is_empty() {
                "none specified".to_string()
            } else {
                node.chains.join(", ")
            };
            let step_badges: String = if node.steps.is_empty() {
                String::new()
            } else {
                let badges: Vec<String> = node.steps.iter().map(|s| format!("#{s}")).collect();
                format!(" {}", badges.join(" "))
            };
            markdown.push_str(&format!("### {display_name}{step_badges}\n"));
            markdown.push_str(&format!("- Slug: `{slug}`\n"));
            markdown.push_str(&format!("- Chains: {chains}\n"));
            if let Some(meta) = tool_meta.get(slug) {
                markdown.push_str(&format!("- Install risk: {}\n", meta.install_risk_level));
            }
            markdown.push_str(&format!("- Page: {SITE_ORIGIN}/tools/{slug}\n"));
            markdown.push_str(&format!(
                "- MCP: `get_install_guide({{ slug: \"{slug}\", platform: \"{platform}\" }})`\n\n"
            ));
        }
    }

    markdown.push_str("## Notes\n\n");
    let note_texts: Vec<String> = export_nodes
        .iter()
        .filter(|node| node.kind == "note")
        .filter_map(|node| node.text.as_deref())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .collect();
    if note_texts.is_empty() {
        markdown.push_str("(none)\n\n");
    } else {
        for text in note_texts {
            markdown.push_str(&format!("- {text}\n"));
        }
        markdown.push('\n');
    }

    let order_section = build_order_section(&export_nodes);
    if !order_section.is_empty() {
        markdown.push_str("## Order\n\n");
        markdown.push_str(&order_section);
        markdown.push_str("\n\n");
    }

    markdown.push_str("## Flow\n\n");
    markdown.push_str(&build_flow_section(&export_nodes, edges));
    markdown.push_str("\n\n");
    markdown.push_str(&AGENT_EXPORT_TASK_TEMPLATE.replace("{platform}", platform));
    markdown
}

async fn fetch_tool_export_meta(
    state: &AppState,
    slugs: &[String],
) -> Result<HashMap<String, ToolExportMeta>, ApiError> {
    if slugs.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(sqlx::FromRow)]
    struct ToolExportRow {
        slug: String,
        name: String,
        install_risk_level: String,
    }

    let rows = sqlx::query_as::<_, ToolExportRow>(
        r#"
        SELECT slug, name, install_risk_level
        FROM tools
        WHERE slug = ANY($1)
          AND approval_status = 'approved'
        "#,
    )
    .bind(slugs)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| db_internal("tool lookup", e))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.slug,
                ToolExportMeta {
                    name: row.name,
                    install_risk_level: row.install_risk_level,
                },
            )
        })
        .collect())
}

async fn agent_export_blueprint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<AgentExportResponse>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    let row = fetch_owned_blueprint(&state, id, user.id).await?;
    let slugs = collect_tool_slugs(&row.nodes);
    let tool_meta = fetch_tool_export_meta(&state, &slugs).await?;

    // Default platform is generic; cursor/claude also accepted.
    let platform = headers
        .get("x-blueprint-platform")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("generic");
    let platform = match platform {
        "cursor" => "cursor",
        "claude" => "claude",
        _ => "generic",
    };

    let markdown =
        build_agent_export_markdown(&row.title, &row.nodes, &row.edges, &tool_meta, platform);

    Ok(Json(AgentExportResponse {
        title: row.title,
        markdown,
        slugs,
        filename: AGENT_EXPORT_FILENAME.into(),
    }))
}

async fn delete_blueprint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let user = require_user_from(&state, &headers).await?;

    let result = sqlx::query("DELETE FROM blueprints WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&state.pool)
        .await
        .map_err(|e| db_internal("delete", e))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("blueprint not found".into()));
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn fetch_owned_blueprint(
    state: &AppState,
    id: Uuid,
    user_id: Uuid,
) -> Result<BlueprintRow, ApiError> {
    sqlx::query_as::<_, BlueprintRow>("SELECT * FROM blueprints WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| db_internal("load", e))?
        .ok_or_else(|| ApiError::NotFound("blueprint not found".into()))
}

impl BlueprintRow {
    fn into_view(self) -> BlueprintView {
        BlueprintView {
            id: self.id,
            title: self.title,
            nodes: self.nodes,
            edges: self.edges,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_title_defaults_empty_to_untitled() {
        assert_eq!(validate_title("  ").unwrap(), "Untitled blueprint");
    }

    #[test]
    fn validate_title_rejects_overlong_input() {
        let long = "a".repeat(MAX_TITLE_LEN + 1);
        assert!(validate_title(&long).is_err());
    }

    #[test]
    fn validate_nodes_normalizes_tool_and_note() {
        let nodes = json!([
            {"id": "n1", "kind": "tool", "slug": "  foo  ", "chains": ["Base", "base"], "x": 10, "y": 20},
            {"id": "n2", "kind": "note", "text": "hello", "x": 0, "y": 0}
        ]);
        let result = validate_nodes(&nodes).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["slug"], "foo");
        assert_eq!(arr[0]["chains"], json!(["base"]));
    }

    #[test]
    fn validate_nodes_rejects_invalid_kind() {
        let nodes = json!([{"id": "n1", "kind": "widget", "x": 0, "y": 0}]);
        assert!(validate_nodes(&nodes).is_err());
    }

    #[test]
    fn validate_nodes_rejects_out_of_range_coordinates() {
        let nodes = json!([{"id": "n1", "kind": "note", "text": "", "x": -1, "y": 0}]);
        assert!(validate_nodes(&nodes).is_err());
    }

    #[test]
    fn validate_nodes_normalizes_chain() {
        let nodes = json!([
            {"id": "c1", "kind": "chain", "chainId": "  ethereum  ", "x": 8, "y": 16}
        ]);
        let result = validate_nodes(&nodes).unwrap();
        assert_eq!(result[0]["chainId"], "ethereum");
    }

    #[test]
    fn validate_edges_accepts_solid_and_arrow() {
        let nodes = json!([
            {"id": "a", "kind": "tool", "slug": "foo", "x": 0, "y": 0},
            {"id": "b", "kind": "note", "text": "", "x": 40, "y": 40}
        ]);
        let node_ids = node_ids_from_value(&validate_nodes(&nodes).unwrap()).unwrap();
        let edges = json!([
            {
                "id": "e1",
                "fromId": "a",
                "toId": "b",
                "style": "arrow",
                "color": "#E76F00"
            }
        ]);
        let result = validate_edges(&edges, &node_ids).unwrap();
        assert_eq!(result[0]["style"], "arrow");
    }

    #[test]
    fn validate_edges_rejects_unknown_nodes() {
        let edges = json!([
            {
                "id": "e1",
                "fromId": "missing",
                "toId": "also-missing",
                "style": "solid",
                "color": "#1A1A1A"
            }
        ]);
        assert!(validate_edges(&edges, &[]).is_err());
    }

    #[test]
    fn build_agent_export_markdown_includes_tools_notes_flow_and_task() {
        let nodes = json!([
            {"id": "t1", "kind": "tool", "slug": "uniswap", "chains": ["base"], "x": 0, "y": 0},
            {"id": "n1", "kind": "note", "text": "Start here", "x": 40, "y": 40}
        ]);
        let edges = json!([
            {
                "id": "e1",
                "fromId": "t1",
                "toId": "n1",
                "style": "arrow",
                "color": "#E76F00"
            }
        ]);
        let mut tool_meta = HashMap::new();
        tool_meta.insert(
            "uniswap".into(),
            ToolExportMeta {
                name: "Uniswap".into(),
                install_risk_level: "low".into(),
            },
        );

        let markdown =
            build_agent_export_markdown("My Stack", &nodes, &edges, &tool_meta, "cursor");

        assert!(markdown.starts_with("# My Stack\n"));
        assert!(markdown.contains("get_install_guide"));
        assert!(markdown.contains("### Uniswap"));
        assert!(markdown.contains("- Slug: `uniswap`"));
        assert!(markdown.contains("- Chains: base"));
        assert!(markdown.contains("- Install risk: low"));
        assert!(markdown.contains(&format!("{SITE_ORIGIN}/tools/uniswap")));
        assert!(markdown.contains("## Notes"));
        assert!(markdown.contains("- Start here"));
        assert!(markdown.contains("## Flow"));
        assert!(markdown.contains("uniswap → note: Start here"));
        assert!(markdown.contains("## Your task"));
        assert!(markdown.contains("do not install critical-risk tools"));
        assert!(markdown.contains("export PNG separately from the editor Share dock"));
    }

    #[test]
    fn build_order_section_lists_stepped_nodes_sorted() {
        let nodes = parse_export_nodes(&json!([
            {"id": "t2", "kind": "tool", "slug": "beta", "x": 0, "y": 0, "step": 2},
            {"id": "n1", "kind": "note", "text": "check", "x": 0, "y": 0, "step": 1},
            {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0, "step": 3}
        ]));

        let order = build_order_section(&nodes);

        assert_eq!(
            order,
            "- 1. note: check (note)\n- 2. beta (tool)\n- 3. alpha (tool)"
        );
    }

    #[test]
    fn build_order_section_tiebreaks_duplicate_steps_by_node_id() {
        let nodes = parse_export_nodes(&json!([
            {"id": "t2", "kind": "tool", "slug": "beta", "x": 0, "y": 0, "step": 1},
            {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0, "step": 1}
        ]));

        let order = build_order_section(&nodes);

        assert_eq!(order, "- 1. alpha (tool)\n- 1. beta (tool)");
    }

    #[test]
    fn build_order_section_returns_empty_without_steps() {
        let nodes = parse_export_nodes(&json!([
            {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0}
        ]));

        assert!(build_order_section(&nodes).is_empty());
    }

    #[test]
    fn build_order_section_supports_multi_step_nodes() {
        let nodes = parse_export_nodes(&json!([
            {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0, "steps": [1, 7]},
            {"id": "t2", "kind": "tool", "slug": "beta", "x": 0, "y": 0, "steps": [3]}
        ]));

        let order = build_order_section(&nodes);

        assert_eq!(
            order,
            "- 1. alpha (tool)\n- 3. beta (tool)\n- 7. alpha (tool)"
        );
    }

    #[test]
    fn build_agent_export_markdown_includes_step_badges_in_tool_heading() {
        let nodes = json!([
            {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0, "steps": [1, 7]}
        ]);
        let markdown =
            build_agent_export_markdown("Stack", &nodes, &json!([]), &HashMap::new(), "generic");

        assert!(markdown.contains("### alpha #1 #7"));
        assert!(markdown.contains("platform: \"generic\""));
    }

    #[test]
    fn build_agent_export_markdown_includes_order_section_after_notes() {
        let nodes = json!([
            {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0, "step": 1},
            {"id": "n1", "kind": "note", "text": "memo", "x": 0, "y": 0}
        ]);
        let markdown =
            build_agent_export_markdown("Stack", &nodes, &json!([]), &HashMap::new(), "cursor");

        let notes_pos = markdown.find("## Notes").unwrap();
        let order_pos = markdown.find("## Order").unwrap();
        let flow_pos = markdown.find("## Flow").unwrap();
        assert!(notes_pos < order_pos);
        assert!(order_pos < flow_pos);
        assert!(markdown.contains("- 1. alpha (tool)"));
        assert!(markdown.contains("treat it as the owner's step sequence"));
    }

    #[test]
    fn build_agent_export_markdown_sorts_tools_by_step_then_canvas_order() {
        let nodes = json!([
            {"id": "t1", "kind": "tool", "slug": "first", "x": 0, "y": 0},
            {"id": "t2", "kind": "tool", "slug": "second", "x": 10, "y": 0, "step": 2},
            {"id": "t3", "kind": "tool", "slug": "third", "x": 20, "y": 0, "step": 1},
            {"id": "t4", "kind": "tool", "slug": "fourth", "x": 30, "y": 0}
        ]);
        let markdown =
            build_agent_export_markdown("Stack", &nodes, &json!([]), &HashMap::new(), "cursor");

        let third_pos = markdown.find("### third").unwrap();
        let second_pos = markdown.find("### second").unwrap();
        let first_pos = markdown.find("### first").unwrap();
        let fourth_pos = markdown.find("### fourth").unwrap();
        assert!(third_pos < second_pos);
        assert!(second_pos < first_pos);
        assert!(first_pos < fourth_pos);
    }

    #[test]
    fn build_flow_section_annotates_dashed_edges() {
        let nodes = parse_export_nodes(&json!([
            {"id": "a", "kind": "tool", "slug": "alpha", "x": 0, "y": 0},
            {"id": "b", "kind": "tool", "slug": "beta", "x": 40, "y": 0},
            {"id": "c", "kind": "tool", "slug": "gamma", "x": 80, "y": 0}
        ]));
        let edges = json!([
            {"id": "e1", "fromId": "a", "toId": "b", "style": "arrow", "color": "#1A1A1A",
             "dashed": true, "label": "optional"},
            {"id": "e2", "fromId": "b", "toId": "c", "style": "arrow", "color": "#1A1A1A",
             "dashed": true}
        ]);

        let flow = build_flow_section(&nodes, &edges);

        assert!(flow.contains("alpha →(optional, dashed) beta"));
        assert!(flow.contains("beta -[dashed]→ gamma"));
    }

    #[test]
    fn build_flow_section_lists_edges_when_cycle_detected() {
        let nodes = parse_export_nodes(&json!([
            {"id": "a", "kind": "tool", "slug": "alpha", "x": 0, "y": 0},
            {"id": "b", "kind": "tool", "slug": "beta", "x": 40, "y": 40}
        ]));
        let edges = json!([
            {"id": "e1", "fromId": "a", "toId": "b", "style": "solid", "color": "#1A1A1A"},
            {"id": "e2", "fromId": "b", "toId": "a", "style": "solid", "color": "#1A1A1A"}
        ]);

        let flow = build_flow_section(&nodes, &edges);

        assert!(flow.contains("alpha → beta"));
        assert!(flow.contains("beta → alpha"));
    }

    #[test]
    fn collect_tool_slugs_deduplicates_in_order() {
        let nodes = json!([
            {"id": "t1", "kind": "tool", "slug": "foo", "x": 0, "y": 0},
            {"id": "t2", "kind": "tool", "slug": "bar", "x": 10, "y": 10},
            {"id": "t3", "kind": "tool", "slug": "foo", "x": 20, "y": 20}
        ]);

        assert_eq!(collect_tool_slugs(&nodes), vec!["foo", "bar"]);
    }

    #[test]
    fn validate_nodes_clamps_size_and_step() {
        let nodes = json!([
            {"id": "t1", "kind": "tool", "slug": "foo", "x": 0, "y": 0,
             "w": 9000, "h": 10, "step": 250},
            {"id": "n1", "kind": "note", "text": "", "x": 8, "y": 8, "w": 300, "h": 200},
            {"id": "c1", "kind": "chain", "chainId": "base", "x": 0, "y": 0, "w": 400, "step": 2}
        ]);
        let result = validate_nodes(&nodes).unwrap();
        let arr = result.as_array().unwrap();
        // Tool: width clamped down to max, height clamped up to min, step migrated to steps array capped.
        assert_eq!(arr[0]["w"], json!(NODE_MAX_W));
        assert_eq!(arr[0]["h"], json!(NODE_MIN_H));
        assert_eq!(arr[0]["steps"], json!([NODE_MAX_STEP]));
        // Note keeps in-range size.
        assert_eq!(arr[1]["w"], json!(300));
        assert_eq!(arr[1]["h"], json!(200));
        // Chain never carries a size, but does carry steps (migrated from legacy step).
        assert!(arr[2].get("w").is_none());
        assert_eq!(arr[2]["steps"], json!([2]));
    }

    #[test]
    fn validate_edges_preserves_dashed_and_label() {
        let nodes = json!([
            {"id": "a", "kind": "tool", "slug": "foo", "x": 0, "y": 0},
            {"id": "b", "kind": "note", "text": "", "x": 40, "y": 40}
        ]);
        let node_ids = node_ids_from_value(&validate_nodes(&nodes).unwrap()).unwrap();
        let edges = json!([
            {"id": "e1", "fromId": "a", "toId": "b", "style": "arrow",
             "color": "#E76F00", "dashed": true, "label": "  swap to Base  "}
        ]);
        let result = validate_edges(&edges, &node_ids).unwrap();
        assert_eq!(result[0]["dashed"], json!(true));
        assert_eq!(result[0]["label"], json!("swap to Base"));
    }

    #[test]
    fn validate_edges_omits_falsey_dashed_and_empty_label() {
        let nodes = json!([
            {"id": "a", "kind": "tool", "slug": "foo", "x": 0, "y": 0},
            {"id": "b", "kind": "note", "text": "", "x": 40, "y": 40}
        ]);
        let node_ids = node_ids_from_value(&validate_nodes(&nodes).unwrap()).unwrap();
        let edges = json!([
            {"id": "e1", "fromId": "a", "toId": "b", "style": "solid",
             "color": "#1A1A1A", "dashed": false, "label": "   "}
        ]);
        let result = validate_edges(&edges, &node_ids).unwrap();
        assert!(result[0].get("dashed").is_none());
        assert!(result[0].get("label").is_none());
    }

    #[test]
    fn build_flow_section_splits_at_branch_points() {
        let nodes = parse_export_nodes(&json!([
            {"id": "hub", "kind": "tool", "slug": "gateway", "x": 0, "y": 0},
            {"id": "base", "kind": "chain", "chainId": "base", "x": 40, "y": 0},
            {"id": "bnb", "kind": "chain", "chainId": "bsc", "x": 40, "y": 40}
        ]));
        let edges = json!([
            {"id": "e1", "fromId": "hub", "toId": "base", "style": "arrow", "color": "#1A1A1A"},
            {"id": "e2", "fromId": "hub", "toId": "bnb", "style": "arrow", "color": "#1A1A1A",
             "label": "swap"}
        ]);

        let flow = build_flow_section(&nodes, &edges);
        let lines: Vec<&str> = flow.lines().collect();

        // Branch point produces one line per outgoing branch, not a single chain.
        assert_eq!(lines.len(), 2);
        assert!(flow.contains("gateway → chain: base"));
        assert!(flow.contains("gateway →(swap) chain: bsc"));
    }

    #[test]
    fn build_flow_section_keeps_linear_path_on_one_line() {
        let nodes = parse_export_nodes(&json!([
            {"id": "a", "kind": "tool", "slug": "alpha", "x": 0, "y": 0},
            {"id": "b", "kind": "tool", "slug": "beta", "x": 40, "y": 0},
            {"id": "c", "kind": "tool", "slug": "gamma", "x": 80, "y": 0}
        ]));
        let edges = json!([
            {"id": "e1", "fromId": "a", "toId": "b", "style": "arrow", "color": "#1A1A1A"},
            {"id": "e2", "fromId": "b", "toId": "c", "style": "arrow", "color": "#1A1A1A"}
        ]);

        let flow = build_flow_section(&nodes, &edges);

        assert_eq!(flow, "- alpha → beta → gamma");
    }
}
