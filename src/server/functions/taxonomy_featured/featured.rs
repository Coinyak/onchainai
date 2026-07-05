use super::*;

/// Public featured carousel card joined to tool slug/name.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct FeaturedCardView {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub tool_slug: String,
    pub tool_name: String,
    pub image_url: String,
    pub headline: Option<String>,
    pub subtitle: Option<String>,
    pub sort_order: i32,
}

/// Admin featured card row with linked tool metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct AdminFeaturedCardView {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub tool_slug: String,
    pub tool_name: String,
    pub image_url: String,
    pub headline: Option<String>,
    pub subtitle: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Tool picker row for featured-card admin forms.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ToolPickerItem {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
}

pub(crate) const GET_FEATURED_CARDS_SQL: &str = r#"
        SELECT
            fc.id,
            fc.tool_id,
            t.slug AS tool_slug,
            t.name AS tool_name,
            fc.image_url,
            fc.headline,
            fc.subtitle,
            fc.sort_order
        FROM featured_cards fc
        INNER JOIN tools t ON t.id = fc.tool_id
        WHERE fc.is_active = true
          AND t.approval_status = 'approved'
          AND t.relevance_status = 'accepted'
          AND NOT (t.crypto_relevance_score = 0
            AND 'migration-backfill: crypto keyword in name or description' = ANY(t.crypto_relevance_reasons))
          AND t.install_risk_level <> 'critical'
          AND t.quarantined_at IS NULL
        ORDER BY fc.sort_order ASC, fc.created_at ASC
        "#;

pub(crate) const SEARCH_TOOLS_FOR_PICKER_SQL: &str = r#"
        SELECT id, name, slug
        FROM tools
        WHERE approval_status = 'approved'
          AND relevance_status = 'accepted'
          AND NOT (crypto_relevance_score = 0
            AND 'migration-backfill: crypto keyword in name or description' = ANY(crypto_relevance_reasons))
          AND install_risk_level <> 'critical'
          AND quarantined_at IS NULL
          AND (name ILIKE $1 OR slug ILIKE $1)
        ORDER BY stars DESC, name ASC
        LIMIT $2
        "#;

pub(crate) const FEATURED_TOOL_EXISTS_SQL: &str = r#"
        SELECT EXISTS(
            SELECT 1
            FROM tools
            WHERE id = $1
              AND approval_status = 'approved'
              AND relevance_status = 'accepted'
              AND NOT (crypto_relevance_score = 0
                AND 'migration-backfill: crypto keyword in name or description' = ANY(crypto_relevance_reasons))
              AND install_risk_level <> 'critical'
              AND quarantined_at IS NULL
        )
        "#;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeaturedCardInput {
    pub tool_id: Uuid,
    pub image_url: String,
    pub headline: Option<String>,
    pub subtitle: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateFeaturedCardInput {
    pub id: Uuid,
    pub tool_id: Uuid,
    pub image_url: String,
    pub headline: Option<String>,
    pub subtitle: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UploadFeaturedImageInput {
    pub filename: String,
    pub content_type: String,
    pub data_base64: String,
}

#[cfg(feature = "ssr")]
async fn ensure_featured_tool_exists(pool: &sqlx::PgPool, tool_id: Uuid) -> Result<(), FnError> {
    let exists = sqlx::query_scalar::<_, bool>(FEATURED_TOOL_EXISTS_SQL)
        .bind(tool_id)
        .fetch_one(pool)
        .await
        .map_err(|e| FnError::new(format!("tool lookup failed: {e}")))?;

    if exists {
        Ok(())
    } else {
        Err(FnError::new("approved tool not found"))
    }
}

pub(crate) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) const MAX_FEATURED_IMAGE_BYTES: usize = 5 * 1024 * 1024;

pub(crate) fn validate_featured_image_upload(
    content_type: &str,
    bytes_len: usize,
) -> Result<(), &'static str> {
    let allowed = ["image/jpeg", "image/png", "image/webp", "image/svg+xml"];
    if !allowed.contains(&content_type) {
        return Err("unsupported image type");
    }
    if bytes_len == 0 {
        return Err("image is empty");
    }
    if bytes_len > MAX_FEATURED_IMAGE_BYTES {
        return Err("image too large (max 5 MB)");
    }
    Ok(())
}

pub(crate) fn validate_featured_card_input(
    image_url: &str,
    headline: Option<&str>,
    subtitle: Option<&str>,
) -> Result<(), &'static str> {
    let trimmed = image_url.trim();
    if trimmed.is_empty() {
        return Err("image URL is required");
    }
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err("image URL must start with http:// or https://");
    }
    if let Some(h) = headline {
        if h.chars().count() > 120 {
            return Err("headline is too long");
        }
    }
    if let Some(s) = subtitle {
        if s.chars().count() > 200 {
            return Err("subtitle is too long");
        }
    }
    Ok(())
}

pub(crate) fn featured_image_extension(content_type: &str, filename: &str) -> Option<&'static str> {
    match content_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        _ => filename
            .rsplit('.')
            .next()
            .and_then(|ext| match ext.to_ascii_lowercase().as_str() {
                "jpg" | "jpeg" => Some("jpg"),
                "png" => Some("png"),
                "webp" => Some("webp"),
                "svg" => Some("svg"),
                _ => None,
            }),
    }
}

/// Pure selection helper for tests — mirrors public featured-card ordering/filtering.
#[allow(dead_code)]
pub(crate) fn select_active_featured_cards(
    cards: &mut [FeaturedCardView],
) -> Vec<FeaturedCardView> {
    cards.sort_by(|a, b| {
        a.sort_order
            .cmp(&b.sort_order)
            .then_with(|| a.id.cmp(&b.id))
    });
    cards.to_vec()
}
