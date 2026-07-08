//! Comment and bookmark endpoints.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::session::{optional_session_result, session_from_parts};
use crate::models::Comment;
use crate::server::functions::{
    resolve_bookmark_tool_id, validate_comment_content, CommentView, TOGGLE_BOOKMARK_SQL,
    TOGGLE_UPVOTE_SQL,
};
use crate::server::queries::{
    APPROVED_TOOL_ID_BY_SLUG_SQL, IS_BOOKMARKED_SQL, TOOL_COMMENTS_NEW_SORT_SQL,
    TOOL_COMMENTS_TOP_SORT_SQL,
};
use crate::server::rate_limit::{check_user_rate_limit, UserRateLimitAction};
use crate::AppState;

use super::auth::require_user_from;
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v2/tools/{slug}/comments",
            get(get_tool_comments).post(create_comment),
        )
        .route(
            "/api/v2/tools/{slug}/bookmark",
            get(is_bookmarked).put(set_bookmark).post(toggle_bookmark),
        )
        .route("/api/v2/comments/{id}/upvote", post(toggle_upvote))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct CommentsQuery {
    #[serde(default = "default_sort")]
    sort: String,
}

fn default_sort() -> String {
    "new".into()
}

#[derive(Debug, Deserialize)]
struct CreateCommentBody {
    content: String,
    parent_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct SetBookmarkBody {
    starred: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct CommentRow {
    id: Uuid,
    tool_id: Uuid,
    parent_id: Option<Uuid>,
    user_id: Uuid,
    content: String,
    created_at: chrono::DateTime<chrono::Utc>,
    author_nickname: Option<String>,
    author_auth_method: Option<String>,
    author_is_admin: bool,
    upvote_count: Option<i64>,
    viewer_upvoted: Option<bool>,
}

impl CommentRow {
    fn into_view(self) -> CommentView {
        CommentView {
            id: self.id,
            tool_id: self.tool_id,
            parent_id: self.parent_id,
            user_id: self.user_id,
            content: self.content,
            created_at: self.created_at,
            author_nickname: self.author_nickname,
            author_auth_method: self.author_auth_method,
            author_is_admin: self.author_is_admin,
            upvote_count: self.upvote_count.unwrap_or(0),
            viewer_upvoted: self.viewer_upvoted.unwrap_or(false),
        }
    }
}

async fn get_tool_comments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Query(q): Query<CommentsQuery>,
) -> Result<Json<Vec<CommentView>>, ApiError> {
    let parts = super::auth::parts_from_headers(&headers);
    let viewer = session_from_parts(
        &parts,
        &state.pool,
        &state.config.jwt_secret,
        &state.config.jwt_issuer(),
    )
    .await
    .ok()
    .flatten();

    let tool_id = sqlx::query_scalar::<_, Uuid>(APPROVED_TOOL_ID_BY_SLUG_SQL)
        .bind(&slug)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to resolve tool: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("tool not found: {slug}")))?;

    let sql = match q.sort.as_str() {
        "top" => TOOL_COMMENTS_TOP_SORT_SQL,
        "new" => TOOL_COMMENTS_NEW_SORT_SQL,
        _ => return Err(ApiError::BadRequest("sort must be 'new' or 'top'".into())),
    };
    let rows = sqlx::query_as::<_, CommentRow>(sql)
        .bind(tool_id)
        .bind(viewer.as_ref().map(|v| v.id))
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to load comments: {e}")))?;

    Ok(Json(rows.into_iter().map(CommentRow::into_view).collect()))
}

async fn create_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(body): Json<CreateCommentBody>,
) -> Result<Json<Comment>, ApiError> {
    if let Err(msg) = validate_comment_content(&body.content) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    let user = require_user_from(&state, &headers).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::CreateComment) {
        return Err(ApiError::TooManyRequests(limit.to_string()));
    }

    let tool_id = sqlx::query_scalar::<_, Uuid>(APPROVED_TOOL_ID_BY_SLUG_SQL)
        .bind(&slug)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to resolve tool: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("tool not found: {slug}")))?;

    if let Some(parent) = body.parent_id {
        let parent_row = sqlx::query_as::<_, (Option<Uuid>,)>(
            "SELECT parent_id FROM comments WHERE id = $1 AND tool_id = $2",
        )
        .bind(parent)
        .bind(tool_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("parent lookup failed: {e}")))?;

        match parent_row {
            Some((None,)) => {}
            Some((Some(_),)) => {
                return Err(ApiError::BadRequest(
                    "only one level of replies is allowed".into(),
                ));
            }
            None => return Err(ApiError::NotFound("parent comment not found".into())),
        }
    }

    let comment = sqlx::query_as::<_, Comment>(
        r#"
        INSERT INTO comments (tool_id, parent_id, user_id, content)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(tool_id)
    .bind(body.parent_id)
    .bind(user.id)
    .bind(body.content.trim())
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to create comment: {e}")))?;

    Ok(Json(comment))
}

async fn toggle_upvote(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(comment_id): Path<Uuid>,
) -> Result<Json<bool>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    let upvoted = sqlx::query_scalar::<_, bool>(TOGGLE_UPVOTE_SQL)
        .bind(comment_id)
        .bind(user.id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("upvote toggle failed: {e}")))?;
    Ok(Json(upvoted))
}

async fn is_bookmarked(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<bool>, ApiError> {
    let parts = super::auth::parts_from_headers(&headers);
    let Some(user) = optional_session_result(
        session_from_parts(
            &parts,
            &state.pool,
            &state.config.jwt_secret,
            &state.config.jwt_issuer(),
        )
        .await,
    )
    .map_err(|e| ApiError::Internal(e.to_string()))?
    else {
        return Ok(Json(false));
    };

    let bookmarked = sqlx::query_scalar::<_, i64>(IS_BOOKMARKED_SQL)
        .bind(slug)
        .bind(user.id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("bookmark lookup failed: {e}")))?;

    Ok(Json(bookmarked > 0))
}

async fn set_bookmark(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(body): Json<SetBookmarkBody>,
) -> Result<Json<bool>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::ToggleBookmark) {
        return Err(ApiError::TooManyRequests(limit.to_string()));
    }

    let tool_id = resolve_bookmark_tool_id(&state.pool, &slug)
        .await
        .map_err(ApiError::from_server_fn)?;

    let result = if body.starred {
        sqlx::query(
            "INSERT INTO bookmarks (tool_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(tool_id)
        .bind(user.id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to add bookmark: {e}")))?;
        true
    } else {
        sqlx::query("DELETE FROM bookmarks WHERE tool_id = $1 AND user_id = $2")
            .bind(tool_id)
            .bind(user.id)
            .execute(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(format!("failed to remove bookmark: {e}")))?;
        false
    };
    Ok(Json(result))
}

async fn toggle_bookmark(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<bool>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::ToggleBookmark) {
        return Err(ApiError::TooManyRequests(limit.to_string()));
    }

    let tool_id = resolve_bookmark_tool_id(&state.pool, &slug)
        .await
        .map_err(ApiError::from_server_fn)?;
    let starred = sqlx::query_scalar::<_, bool>(TOGGLE_BOOKMARK_SQL)
        .bind(tool_id)
        .bind(user.id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("bookmark toggle failed: {e}")))?;
    Ok(Json(starred))
}
