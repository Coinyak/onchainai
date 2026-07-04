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
                normalized.push(payload);
            }
            "note" => {
                let text = node.text.unwrap_or_default();
                if text.chars().count() > MAX_NOTE_TEXT {
                    return Err(ApiError::BadRequest(format!(
                        "note text must be at most {MAX_NOTE_TEXT} characters"
                    )));
                }
                normalized.push(serde_json::json!({
                    "id": node.id,
                    "kind": "note",
                    "text": text,
                    "x": node.x,
                    "y": node.y,
                }));
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
                normalized.push(serde_json::json!({
                    "id": node.id,
                    "kind": "chain",
                    "chainId": chain_id,
                    "x": node.x,
                    "y": node.y,
                }));
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

        normalized.push(serde_json::json!({
            "id": edge.id,
            "fromId": from_id,
            "toId": to_id,
            "style": style,
            "color": edge.color.trim().to_ascii_uppercase(),
        }));
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
}
