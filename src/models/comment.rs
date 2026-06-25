//! Social models — `comments`, `upvotes`, `bookmarks`.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A comment row from the `comments` table.
///
/// Threading is one level deep: top-level comments have `parent_id = None`;
/// replies have `parent_id` pointing to a top-level comment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Comment {
    pub id: Uuid,
    pub tool_id: Uuid,
    /// `None` for top-level comments; set for 1-level replies.
    pub parent_id: Option<Uuid>,
    pub user_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An upvote row from the `upvotes` table.
///
/// Unique per `(comment_id, user_id)` via a DB unique index.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Upvote {
    pub id: Uuid,
    pub comment_id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// A bookmark row from the `bookmarks` table.
///
/// Unique per `(tool_id, user_id)` via a DB unique index.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Bookmark {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_serde_round_trip() {
        let now = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let c = Comment {
            id: Uuid::nil(),
            tool_id: Uuid::nil(),
            parent_id: None,
            user_id: Uuid::nil(),
            content: "nice".into(),
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_string(&c).expect("serialize comment");
        let back: Comment = serde_json::from_str(&json).expect("deserialize comment");
        assert_eq!(back.content, "nice");
        assert!(back.parent_id.is_none());
    }

    #[test]
    fn upvote_serde_round_trip() {
        let now = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let u = Upvote {
            id: Uuid::nil(),
            comment_id: Uuid::nil(),
            user_id: Uuid::nil(),
            created_at: now,
        };
        let json = serde_json::to_string(&u).expect("serialize upvote");
        let back: Upvote = serde_json::from_str(&json).expect("deserialize upvote");
        assert_eq!(back.user_id, Uuid::nil());
    }

    #[test]
    fn bookmark_serde_round_trip() {
        let now = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let b = Bookmark {
            id: Uuid::nil(),
            tool_id: Uuid::nil(),
            user_id: Uuid::nil(),
            created_at: now,
        };
        let json = serde_json::to_string(&b).expect("serialize bookmark");
        let back: Bookmark = serde_json::from_str(&json).expect("deserialize bookmark");
        assert_eq!(back.tool_id, Uuid::nil());
    }
}
