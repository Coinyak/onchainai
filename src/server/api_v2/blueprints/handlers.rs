//! Blueprint CRUD HTTP handlers.
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use uuid::Uuid;

use crate::AppState;

use super::super::auth::require_user_from;
use super::super::error::ApiError;
use super::access::fetch_owned_blueprint;
use super::export::agent_export_blueprint;
use super::types::*;
use super::validate::{
    node_ids_from_value, prune_edges_for_nodes, validate_edges, validate_nodes, validate_title,
};

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
