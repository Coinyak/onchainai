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

pub(crate) const TOGGLE_UPVOTE_SQL: &str = r#"
WITH lock_key AS (
    SELECT pg_advisory_xact_lock(hashtextextended('upvote:' || $1::text || ':' || $2::text, 0::bigint))
),
deleted AS (
    DELETE FROM upvotes
    WHERE comment_id = $1
      AND user_id = $2
      AND EXISTS (SELECT 1 FROM lock_key)
    RETURNING 1
),
inserted AS (
    INSERT INTO upvotes (comment_id, user_id)
    SELECT $1, $2
    WHERE NOT EXISTS (SELECT 1 FROM deleted)
      AND EXISTS (SELECT 1 FROM lock_key)
    ON CONFLICT DO NOTHING
    RETURNING 1
)
SELECT EXISTS(SELECT 1 FROM inserted)
"#;

pub(crate) const TOGGLE_BOOKMARK_SQL: &str = r#"
WITH lock_key AS (
    SELECT pg_advisory_xact_lock(hashtextextended('bookmark:' || $1::text || ':' || $2::text, 0::bigint))
),
deleted AS (
    DELETE FROM bookmarks
    WHERE tool_id = $1
      AND user_id = $2
      AND EXISTS (SELECT 1 FROM lock_key)
    RETURNING 1
),
inserted AS (
    INSERT INTO bookmarks (tool_id, user_id)
    SELECT $1, $2
    WHERE NOT EXISTS (SELECT 1 FROM deleted)
      AND EXISTS (SELECT 1 FROM lock_key)
    ON CONFLICT DO NOTHING
    RETURNING 1
)
SELECT EXISTS(SELECT 1 FROM inserted)
"#;

/// Validate comment body before insert (Unicode scalar count, not bytes).
///
/// Caps the scalar walk at 2001 so oversized bodies (up to the request-body
/// limit) do not force a full-string `chars().count()`. A cheap byte-length
/// guard rejects inputs that cannot fit in 2000 UTF-8 scalars (max 4 bytes each).
pub(crate) fn validate_comment_content(content: &str) -> Result<(), &'static str> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("comment must be 1–2000 characters");
    }
    // 2000 scalars × up to 4 UTF-8 bytes/scalar.
    if trimmed.len() > 2000 * 4 {
        return Err("comment must be 1–2000 characters");
    }
    let chars = trimmed.chars().take(2001).count();
    if chars > 2000 {
        return Err("comment must be 1–2000 characters");
    }
    Ok(())
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

#[cfg(feature = "ssr")]
pub(crate) async fn resolve_bookmark_tool_id(
    pool: &sqlx::PgPool,
    slug: &str,
) -> Result<Uuid, FnError> {
    sqlx::query_scalar::<_, Uuid>(APPROVED_TOOL_ID_BY_SLUG_SQL)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(|e| FnError::new(format!("failed to resolve tool: {e}")))?
        .ok_or_else(|| FnError::new(format!("tool not found: {slug}")))
}
