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
