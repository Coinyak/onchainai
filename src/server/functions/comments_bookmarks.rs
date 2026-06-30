use super::*;

/// Comment with author display fields and upvote count.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommentView {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub user_id: Uuid,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub author_nickname: Option<String>,
    pub author_auth_method: Option<String>,
    pub author_is_admin: bool,
    pub upvote_count: i64,
    pub viewer_upvoted: bool,
}

/// Validate comment body before insert.
pub(crate) fn validate_comment_content(content: &str) -> Result<(), &'static str> {
    let trimmed = content.trim();
    if trimmed.is_empty() || trimmed.len() > 2000 {
        return Err("comment must be 1–2000 characters");
    }
    Ok(())
}

/// List comments for an approved tool (`sort`: `new` | `top`).
#[server(GetToolComments, "/api")]
pub async fn get_tool_comments(
    slug: String,
    sort: String,
) -> Result<Vec<CommentView>, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let viewer = session_from_parts(&parts, &pool, &config.jwt_secret, &config.jwt_issuer())
        .await
        .ok()
        .flatten();

    let tool_id = sqlx::query_scalar::<_, Uuid>(APPROVED_TOOL_ID_BY_SLUG_SQL)
        .bind(&slug)
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to resolve tool: {e}")))?
        .ok_or_else(|| ServerFnError::new(format!("tool not found: {slug}")))?;

    let sql = match sort.as_str() {
        "top" => TOOL_COMMENTS_TOP_SORT_SQL,
        "new" => TOOL_COMMENTS_NEW_SORT_SQL,
        _ => return Err(ServerFnError::new("sort must be 'new' or 'top'")),
    };
    let rows = sqlx::query_as::<_, CommentRow>(sql)
        .bind(tool_id)
        .bind(viewer.as_ref().map(|v| v.id))
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load comments: {e}")))?;

    Ok(rows.into_iter().map(CommentRow::into_view).collect())
}

#[cfg(feature = "ssr")]
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

#[cfg(feature = "ssr")]
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

/// Count approved-tool comments (for list sort / badges).
#[server(GetToolCommentCount, "/api")]
pub async fn get_tool_comment_count(slug: String) -> Result<i64, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let count = sqlx::query_scalar::<_, i64>(TOOL_COMMENT_COUNT_BY_SLUG_SQL)
        .bind(slug)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("comment count failed: {e}")))?;

    Ok(count)
}

/// Post a comment or reply (authenticated).
#[server(CreateComment, "/api")]
pub async fn create_comment(
    slug: String,
    content: String,
    parent_id: Option<Uuid>,
) -> Result<Comment, ServerFnError> {
    if let Err(msg) = validate_comment_content(&content) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, config) = request_context()?;
    let user = require_user(&parts, &pool, &config.jwt_secret, &config.jwt_issuer()).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::CreateComment) {
        return Err(ServerFnError::new(limit.to_string()));
    }

    let tool_id = sqlx::query_scalar::<_, Uuid>(APPROVED_TOOL_ID_BY_SLUG_SQL)
        .bind(&slug)
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to resolve tool: {e}")))?
        .ok_or_else(|| ServerFnError::new(format!("tool not found: {slug}")))?;

    if let Some(parent) = parent_id {
        let parent_row = sqlx::query_as::<_, (Option<Uuid>,)>(
            "SELECT parent_id FROM comments WHERE id = $1 AND tool_id = $2",
        )
        .bind(parent)
        .bind(tool_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("parent lookup failed: {e}")))?;

        match parent_row {
            Some((None,)) => {}
            Some((Some(_),)) => {
                return Err(ServerFnError::new("only one level of replies is allowed"));
            }
            None => return Err(ServerFnError::new("parent comment not found")),
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
    .bind(parent_id)
    .bind(user.id)
    .bind(content.trim())
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to create comment: {e}")))?;

    Ok(comment)
}

/// Toggle upvote on a comment (authenticated, atomic).
#[server(ToggleUpvote, "/api")]
pub async fn toggle_upvote(comment_id: Uuid) -> Result<bool, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let user = require_user(&parts, &pool, &config.jwt_secret, &config.jwt_issuer()).await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("transaction failed: {e}")))?;

    let deleted: i64 = sqlx::query_scalar(
        "WITH deleted AS (DELETE FROM upvotes WHERE comment_id = $1 AND user_id = $2 RETURNING 1) \
         SELECT COUNT(*) FROM deleted",
    )
    .bind(comment_id)
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("upvote toggle failed: {e}")))?;

    if deleted > 0 {
        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(format!("commit failed: {e}")))?;
        return Ok(false);
    }

    sqlx::query("INSERT INTO upvotes (comment_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(comment_id)
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to add upvote: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("commit failed: {e}")))?;
    Ok(true)
}

/// Whether the current user bookmarked a tool (false when signed out).
#[server(IsBookmarked, "/api")]
pub async fn is_bookmarked(slug: String) -> Result<bool, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let Some(user) = optional_session_result(
        session_from_parts(&parts, &pool, &config.jwt_secret, &config.jwt_issuer()).await,
    )?
    else {
        return Ok(false);
    };

    let bookmarked = sqlx::query_scalar::<_, i64>(IS_BOOKMARKED_SQL)
        .bind(slug)
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("bookmark lookup failed: {e}")))?;

    Ok(bookmarked > 0)
}

/// Set bookmark state on a tool (authenticated, idempotent, atomic).
#[server(SetBookmark, "/api")]
pub async fn set_bookmark(slug: String, starred: bool) -> Result<bool, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let user = require_user(&parts, &pool, &config.jwt_secret, &config.jwt_issuer()).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::ToggleBookmark) {
        return Err(ServerFnError::new(limit.to_string()));
    }

    let tool_id = resolve_bookmark_tool_id(&pool, &slug).await?;

    let result = if starred {
        sqlx::query(
            "INSERT INTO bookmarks (tool_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(tool_id)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to add bookmark: {e}")))?;
        true
    } else {
        sqlx::query("DELETE FROM bookmarks WHERE tool_id = $1 AND user_id = $2")
            .bind(tool_id)
            .bind(user.id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("failed to remove bookmark: {e}")))?;
        false
    };
    Ok(result)
}

/// Toggle bookmark on a tool (authenticated, atomic).
#[server(ToggleBookmark, "/api")]
pub async fn toggle_bookmark(slug: String) -> Result<bool, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let user = require_user(&parts, &pool, &config.jwt_secret, &config.jwt_issuer()).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::ToggleBookmark) {
        return Err(ServerFnError::new(limit.to_string()));
    }

    let tool_id = resolve_bookmark_tool_id(&pool, &slug).await?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("transaction failed: {e}")))?;

    let deleted: i64 = sqlx::query_scalar(
        "WITH deleted AS (DELETE FROM bookmarks WHERE tool_id = $1 AND user_id = $2 RETURNING 1) \
         SELECT COUNT(*) FROM deleted",
    )
    .bind(tool_id)
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("bookmark toggle failed: {e}")))?;

    if deleted > 0 {
        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(format!("commit failed: {e}")))?;
        return Ok(false);
    }

    sqlx::query("INSERT INTO bookmarks (tool_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(tool_id)
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to add bookmark: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("commit failed: {e}")))?;
    Ok(true)
}

async fn resolve_bookmark_tool_id(pool: &sqlx::PgPool, slug: &str) -> Result<Uuid, ServerFnError> {
    sqlx::query_scalar::<_, Uuid>(APPROVED_TOOL_ID_BY_SLUG_SQL)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to resolve tool: {e}")))?
        .ok_or_else(|| ServerFnError::new(format!("tool not found: {slug}")))
}
