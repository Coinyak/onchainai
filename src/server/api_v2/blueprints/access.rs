//! Owned-blueprint DB helpers.
use axum::extract::State;
use uuid::Uuid;

use crate::AppState;

use super::super::error::ApiError;
use super::types::*;

pub(crate) async fn fetch_owned_blueprint(
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
    pub(crate) fn into_view(self) -> BlueprintView {
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
