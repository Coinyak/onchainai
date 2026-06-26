//! Featured carousel card model.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A `featured_cards` row.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct FeaturedCard {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub image_url: String,
    pub headline: Option<String>,
    pub subtitle: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
