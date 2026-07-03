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
const COORD_MAX: i32 = 4000;
const MAX_NOTE_TEXT: usize = 2000;
const MAX_TITLE_LEN: usize = 200;

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
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateBlueprintBody {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    nodes: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct UpdateBlueprintBody {
    title: Option<String>,
    nodes: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct BlueprintNodeInput {
    id: String,
    kind: String,
    slug: Option<String>,
    text: Option<String>,
    x: i32,
    y: i32,
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
                normalized.push(serde_json::json!({
                    "id": node.id,
                    "kind": "tool",
                    "slug": slug,
                    "x": node.x,
                    "y": node.y,
                }));
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
            other => {
                return Err(ApiError::BadRequest(format!(
                    "node kind must be 'tool' or 'note', got '{other}'"
                )));
            }
        }
    }

    Ok(Value::Array(normalized))
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

    let row = sqlx::query_as::<_, BlueprintRow>(
        r#"
        INSERT INTO blueprints (user_id, title, nodes)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(user.id)
    .bind(&title)
    .bind(&nodes)
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

    if body.title.is_none() && body.nodes.is_none() {
        return Err(ApiError::BadRequest(
            "at least one of title or nodes is required".into(),
        ));
    }

    let existing = fetch_owned_blueprint(&state, id, user.id).await?;

    let title = if let Some(t) = body.title {
        validate_title(&t)?
    } else {
        existing.title
    };

    let nodes = if let Some(n) = body.nodes {
        validate_nodes(&n)?
    } else {
        existing.nodes
    };

    let row = sqlx::query_as::<_, BlueprintRow>(
        r#"
        UPDATE blueprints
        SET title = $3, nodes = $4
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(user.id)
    .bind(&title)
    .bind(&nodes)
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
            {"id": "n1", "kind": "tool", "slug": "  foo  ", "x": 10, "y": 20},
            {"id": "n2", "kind": "note", "text": "hello", "x": 0, "y": 0}
        ]);
        let result = validate_nodes(&nodes).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["slug"], "foo");
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
}
