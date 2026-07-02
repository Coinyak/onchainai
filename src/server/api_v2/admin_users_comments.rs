//! Admin user and comment moderation endpoints.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::server::functions::{AdminCommentView, AdminUserView};
use crate::server::secret_redaction::redact_secrets;
use crate::AppState;

use super::auth::require_admin_from;
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/admin/users", get(list_admin_users))
        .route("/api/v2/admin/users/{id}/ban", put(set_user_banned))
        .route("/api/v2/admin/users/{id}/admin", put(set_user_admin))
        .route("/api/v2/admin/users/{id}", delete(delete_user))
        .route("/api/v2/admin/comments", get(list_admin_comments))
        .route("/api/v2/admin/comments/{id}", delete(delete_admin_comment))
        .route(
            "/api/v2/admin/comments/{id}/ban-author",
            delete(delete_comment_and_ban_user),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    query: Option<String>,
    #[serde(default = "default_list_limit")]
    limit: i64,
}

fn default_list_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
struct BanBody {
    banned: bool,
}

#[derive(Debug, Deserialize)]
struct AdminBody {
    is_admin: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct AdminUserRow {
    id: Uuid,
    nickname: Option<String>,
    auth_method: String,
    is_admin: bool,
    is_banned: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    comment_count: Option<i64>,
    bookmark_count: Option<i64>,
}

impl AdminUserRow {
    fn into_view(self) -> AdminUserView {
        AdminUserView {
            id: self.id,
            nickname: self.nickname,
            auth_method: self.auth_method,
            is_admin: self.is_admin,
            is_banned: self.is_banned,
            comment_count: self.comment_count.unwrap_or(0),
            bookmark_count: self.bookmark_count.unwrap_or(0),
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AdminCommentRow {
    id: Uuid,
    content: String,
    created_at: chrono::DateTime<chrono::Utc>,
    author_id: Uuid,
    author_nickname: Option<String>,
    author_is_banned: bool,
    tool_name: String,
    tool_slug: String,
}

impl AdminCommentRow {
    fn into_view(self) -> AdminCommentView {
        AdminCommentView {
            id: self.id,
            content: redact_secrets(&self.content),
            created_at: self.created_at,
            author_id: self.author_id,
            author_nickname: self.author_nickname,
            author_is_banned: self.author_is_banned,
            tool_name: self.tool_name,
            tool_slug: self.tool_slug,
        }
    }
}

async fn list_admin_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<AdminUserView>>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let limit = q.limit.clamp(1, 100);
    let pattern = q
        .query
        .as_ref()
        .map(|q| q.trim())
        .filter(|q| !q.is_empty())
        .map(|q| format!("%{q}%"));

    let rows = if let Some(pat) = pattern {
        sqlx::query_as::<_, AdminUserRow>(
            r#"
            SELECT
                p.id, p.nickname, p.auth_method, p.is_admin, p.is_banned, p.created_at,
                (SELECT COUNT(*) FROM comments c WHERE c.user_id = p.id) AS comment_count,
                (SELECT COUNT(*) FROM bookmarks b WHERE b.user_id = p.id) AS bookmark_count
            FROM profiles p
            WHERE p.nickname ILIKE $1 OR p.auth_method ILIKE $1
            ORDER BY p.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(pat)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, AdminUserRow>(
            r#"
            SELECT
                p.id, p.nickname, p.auth_method, p.is_admin, p.is_banned, p.created_at,
                (SELECT COUNT(*) FROM comments c WHERE c.user_id = p.id) AS comment_count,
                (SELECT COUNT(*) FROM bookmarks b WHERE b.user_id = p.id) AS bookmark_count
            FROM profiles p
            ORDER BY p.created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    }
    .map_err(|e| ApiError::Internal(format!("failed to list users: {e}")))?;

    Ok(Json(
        rows.into_iter().map(AdminUserRow::into_view).collect(),
    ))
}

async fn set_user_banned(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(body): Json<BanBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_from(&state, &headers).await?;

    if admin.id == user_id {
        return Err(ApiError::BadRequest(
            "cannot change your own ban status".into(),
        ));
    }

    let result =
        sqlx::query("UPDATE profiles SET is_banned = $1, updated_at = now() WHERE id = $2")
            .bind(body.banned)
            .bind(user_id)
            .execute(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(format!("failed to update ban status: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("user not found".into()));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn set_user_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(body): Json<AdminBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_from(&state, &headers).await?;

    if admin.id == user_id && !body.is_admin {
        return Err(ApiError::BadRequest(
            "cannot remove your own admin role".into(),
        ));
    }

    let result = sqlx::query("UPDATE profiles SET is_admin = $1, updated_at = now() WHERE id = $2")
        .bind(body.is_admin)
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to update admin status: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("user not found".into()));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_from(&state, &headers).await?;

    if admin.id == user_id {
        return Err(ApiError::BadRequest(
            "cannot delete your own account".into(),
        ));
    }

    let result = sqlx::query("DELETE FROM profiles WHERE id = $1")
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to delete user: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("user not found".into()));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_admin_comments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<AdminCommentView>>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let limit = q.limit.clamp(1, 100);
    let pattern = q
        .query
        .as_ref()
        .map(|q| q.trim())
        .filter(|q| !q.is_empty())
        .map(|q| format!("%{q}%"));

    let rows = if let Some(pat) = pattern {
        sqlx::query_as::<_, AdminCommentRow>(
            r#"
            SELECT
                c.id, c.content, c.created_at,
                p.id AS author_id, p.nickname AS author_nickname, p.is_banned AS author_is_banned,
                t.name AS tool_name, t.slug AS tool_slug
            FROM comments c
            JOIN profiles p ON p.id = c.user_id
            JOIN tools t ON t.id = c.tool_id
            WHERE c.content ILIKE $1 OR p.nickname ILIKE $1 OR t.name ILIKE $1
            ORDER BY c.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(pat)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, AdminCommentRow>(
            r#"
            SELECT
                c.id, c.content, c.created_at,
                p.id AS author_id, p.nickname AS author_nickname, p.is_banned AS author_is_banned,
                t.name AS tool_name, t.slug AS tool_slug
            FROM comments c
            JOIN profiles p ON p.id = c.user_id
            JOIN tools t ON t.id = c.tool_id
            ORDER BY c.created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    }
    .map_err(|e| ApiError::Internal(format!("failed to list comments: {e}")))?;

    Ok(Json(
        rows.into_iter().map(AdminCommentRow::into_view).collect(),
    ))
}

async fn delete_admin_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(comment_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let result = sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(comment_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to delete comment: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("comment not found".into()));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn delete_comment_and_ban_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(comment_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_from(&state, &headers).await?;

    let author_id = sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM comments WHERE id = $1")
        .bind(comment_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("comment lookup failed: {e}")))?
        .ok_or_else(|| ApiError::NotFound("comment not found".into()))?;

    if author_id == admin.id {
        return Err(ApiError::BadRequest("cannot ban yourself".into()));
    }

    sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(comment_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to delete comment: {e}")))?;

    sqlx::query("UPDATE profiles SET is_banned = true, updated_at = now() WHERE id = $1")
        .bind(author_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to ban user: {e}")))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
