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

/// Active featured cards for the public carousel (ordered).
#[server(GetFeaturedCards, "/api")]
pub async fn get_featured_cards() -> Result<Vec<FeaturedCardView>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let rows = sqlx::query_as::<_, FeaturedCardView>(GET_FEATURED_CARDS_SQL)
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load featured cards: {e}")))?;

    Ok(rows)
}

/// List all featured cards for admin management.
#[server(ListFeaturedCards, "/api")]
pub async fn list_featured_cards() -> Result<Vec<AdminFeaturedCardView>, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let rows = sqlx::query_as::<_, AdminFeaturedCardView>(
        r#"
        SELECT
            fc.id,
            fc.tool_id,
            t.slug AS tool_slug,
            t.name AS tool_name,
            fc.image_url,
            fc.headline,
            fc.subtitle,
            fc.sort_order,
            fc.is_active,
            fc.created_at,
            fc.updated_at
        FROM featured_cards fc
        INNER JOIN tools t ON t.id = fc.tool_id
        ORDER BY fc.sort_order ASC, fc.created_at ASC
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to list featured cards: {e}")))?;

    Ok(rows)
}

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

/// Create a featured carousel card (admin).
#[server(CreateFeaturedCard, "/api")]
pub async fn create_featured_card(input: FeaturedCardInput) -> Result<FeaturedCard, ServerFnError> {
    if let Err(msg) = validate_featured_card_input(
        &input.image_url,
        input.headline.as_deref(),
        input.subtitle.as_deref(),
    ) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    ensure_featured_tool_exists(&pool, input.tool_id).await?;

    let card = sqlx::query_as::<_, FeaturedCard>(
        r#"
        INSERT INTO featured_cards (tool_id, image_url, headline, subtitle, sort_order, is_active)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(input.tool_id)
    .bind(input.image_url.trim())
    .bind(normalize_optional_text(input.headline))
    .bind(normalize_optional_text(input.subtitle))
    .bind(input.sort_order)
    .bind(input.is_active)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to create featured card: {e}")))?;

    Ok(card)
}

/// Update a featured carousel card (admin).
#[server(UpdateFeaturedCard, "/api")]
pub async fn update_featured_card(
    input: UpdateFeaturedCardInput,
) -> Result<FeaturedCard, ServerFnError> {
    if let Err(msg) = validate_featured_card_input(
        &input.image_url,
        input.headline.as_deref(),
        input.subtitle.as_deref(),
    ) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    ensure_featured_tool_exists(&pool, input.tool_id).await?;

    let card = sqlx::query_as::<_, FeaturedCard>(
        r#"
        UPDATE featured_cards
        SET tool_id = $2,
            image_url = $3,
            headline = $4,
            subtitle = $5,
            sort_order = $6,
            is_active = $7
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(input.id)
    .bind(input.tool_id)
    .bind(input.image_url.trim())
    .bind(normalize_optional_text(input.headline))
    .bind(normalize_optional_text(input.subtitle))
    .bind(input.sort_order)
    .bind(input.is_active)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to update featured card: {e}")))?;

    Ok(card)
}

/// Delete a featured carousel card (admin).
#[server(DeleteFeaturedCard, "/api")]
pub async fn delete_featured_card(id: Uuid) -> Result<(), ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let result = sqlx::query("DELETE FROM featured_cards WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to delete featured card: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("featured card not found"));
    }

    Ok(())
}

/// Upload a featured-card image to Supabase Storage (admin).
#[server(UploadFeaturedImage, "/api")]
pub async fn upload_featured_image(
    input: UploadFeaturedImageInput,
) -> Result<String, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let config =
        use_context::<Config>().ok_or_else(|| ServerFnError::new("configuration not available"))?;

    let bytes = {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(input.data_base64.trim())
            .map_err(|e| ServerFnError::new(format!("invalid image encoding: {e}")))?
    };

    if let Err(msg) = validate_featured_image_upload(&input.content_type, bytes.len()) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let ext = featured_image_extension(&input.content_type, &input.filename)
        .ok_or_else(|| ServerFnError::new("unsupported image type"))?;
    let object_path = format!("{}.{}", Uuid::new_v4(), ext);
    let upload_url = format!(
        "{}/storage/v1/object/featured/{}",
        config.supabase_url.trim_end_matches('/'),
        object_path
    );
    let public_url = format!(
        "{}/storage/v1/object/public/featured/{}",
        config.supabase_url.trim_end_matches('/'),
        object_path
    );

    let response = reqwest::Client::new()
        .post(&upload_url)
        .header("apikey", &config.supabase_service_key)
        .header(
            "Authorization",
            format!("Bearer {}", config.supabase_service_key),
        )
        .header("Content-Type", &input.content_type)
        .header("x-upsert", "true")
        .body(bytes)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("image upload failed: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!(
            "storage upload failed ({status}): {body}"
        )));
    }

    Ok(public_url)
}

/// Search approved tools by name or slug for the featured-card picker (admin).
#[server(SearchToolsForPicker, "/api")]
pub async fn search_tools_for_picker(
    query: String,
    limit: i64,
) -> Result<Vec<ToolPickerItem>, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }

    let limit = limit.clamp(1, 50);
    let pattern = format!("%{q}%");
    let rows = sqlx::query_as::<_, ToolPickerItem>(SEARCH_TOOLS_FOR_PICKER_SQL)
        .bind(pattern)
        .bind(limit)
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("tool search failed: {e}")))?;

    Ok(rows)
}

#[cfg(feature = "ssr")]
async fn ensure_featured_tool_exists(
    pool: &sqlx::PgPool,
    tool_id: Uuid,
) -> Result<(), ServerFnError> {
    let exists = sqlx::query_scalar::<_, bool>(FEATURED_TOOL_EXISTS_SQL)
        .bind(tool_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("tool lookup failed: {e}")))?;

    if exists {
        Ok(())
    } else {
        Err(ServerFnError::new("approved tool not found"))
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
    if image_url.trim().is_empty() {
        return Err("image URL is required");
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
