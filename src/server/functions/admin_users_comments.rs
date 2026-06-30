use super::*;

/// Admin user row with activity counts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUserView {
    pub id: Uuid,
    pub nickname: Option<String>,
    pub auth_method: String,
    pub is_admin: bool,
    pub is_banned: bool,
    pub comment_count: i64,
    pub bookmark_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// List profiles for moderation (admin).
#[server(ListAdminUsers, "/api")]
pub async fn list_admin_users(
    query: Option<String>,
    limit: i64,
) -> Result<Vec<AdminUserView>, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let limit = limit.clamp(1, 100);
    let pattern = query
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
        .fetch_all(&pool)
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
        .fetch_all(&pool)
        .await
    }
    .map_err(|e| ServerFnError::new(format!("failed to list users: {e}")))?;

    Ok(rows.into_iter().map(AdminUserRow::into_view).collect())
}

#[cfg(feature = "ssr")]
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

#[cfg(feature = "ssr")]
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

/// Ban or unban a user (admin).
#[server(SetUserBanned, "/api")]
pub async fn set_user_banned(user_id: Uuid, banned: bool) -> Result<(), ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let admin = require_admin(&parts, &pool, &config).await?;

    if admin.id == user_id {
        return Err(ServerFnError::new("cannot change your own ban status"));
    }

    let result =
        sqlx::query("UPDATE profiles SET is_banned = $1, updated_at = now() WHERE id = $2")
            .bind(banned)
            .bind(user_id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("failed to update ban status: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("user not found"));
    }

    Ok(())
}

/// Grant or revoke admin role (admin).
#[server(SetUserAdmin, "/api")]
pub async fn set_user_admin(user_id: Uuid, is_admin: bool) -> Result<(), ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let admin = require_admin(&parts, &pool, &config).await?;

    if admin.id == user_id && !is_admin {
        return Err(ServerFnError::new("cannot remove your own admin role"));
    }

    let result = sqlx::query("UPDATE profiles SET is_admin = $1, updated_at = now() WHERE id = $2")
        .bind(is_admin)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to update admin status: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("user not found"));
    }

    Ok(())
}

/// Delete a user profile and cascaded social data (admin).
#[server(DeleteUser, "/api")]
pub async fn delete_user(user_id: Uuid) -> Result<(), ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let admin = require_admin(&parts, &pool, &config).await?;

    if admin.id == user_id {
        return Err(ServerFnError::new("cannot delete your own account"));
    }

    let result = sqlx::query("DELETE FROM profiles WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to delete user: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("user not found"));
    }

    Ok(())
}

/// Admin comment row for moderation queue.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCommentView {
    pub id: Uuid,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub author_id: Uuid,
    pub author_nickname: Option<String>,
    pub author_is_banned: bool,
    pub tool_name: String,
    pub tool_slug: String,
}

/// List recent comments for moderation (admin).
#[server(ListAdminComments, "/api")]
pub async fn list_admin_comments(
    query: Option<String>,
    limit: i64,
) -> Result<Vec<AdminCommentView>, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let limit = limit.clamp(1, 100);
    let pattern = query
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
        .fetch_all(&pool)
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
        .fetch_all(&pool)
        .await
    }
    .map_err(|e| ServerFnError::new(format!("failed to list comments: {e}")))?;

    Ok(rows.into_iter().map(AdminCommentRow::into_view).collect())
}

#[cfg(feature = "ssr")]
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

#[cfg(feature = "ssr")]
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

/// Delete a comment (admin).
#[server(DeleteAdminComment, "/api")]
pub async fn delete_admin_comment(comment_id: Uuid) -> Result<(), ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let result = sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(comment_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to delete comment: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("comment not found"));
    }

    Ok(())
}

/// Delete a comment and ban its author (admin).
#[server(DeleteCommentAndBanUser, "/api")]
pub async fn delete_comment_and_ban_user(comment_id: Uuid) -> Result<(), ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let admin = require_admin(&parts, &pool, &config).await?;

    let author_id = sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM comments WHERE id = $1")
        .bind(comment_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("comment lookup failed: {e}")))?
        .ok_or_else(|| ServerFnError::new("comment not found"))?;

    if author_id == admin.id {
        return Err(ServerFnError::new("cannot ban yourself"));
    }

    sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(comment_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to delete comment: {e}")))?;

    sqlx::query("UPDATE profiles SET is_banned = true, updated_at = now() WHERE id = $1")
        .bind(author_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to ban user: {e}")))?;

    Ok(())
}
