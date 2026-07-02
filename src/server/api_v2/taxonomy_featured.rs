//! Featured cards and category admin endpoints.

use axum::{
    extract::{Multipart, Path, Query, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::models::{Category, FeaturedCard};
use crate::server::functions::{
    featured_image_extension, normalize_optional_text, validate_category_input,
    validate_featured_card_input, validate_featured_image_upload, AdminCategoryView,
    AdminFeaturedCardView, CategoryInput, FeaturedCardInput, FeaturedCardView, ToolPickerItem,
    UpdateFeaturedCardInput, FEATURED_TOOL_EXISTS_SQL, GET_FEATURED_CARDS_SQL,
    SEARCH_TOOLS_FOR_PICKER_SQL,
};
use crate::AppState;

use super::auth::require_admin_from;
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/featured", get(get_featured_cards))
        .route(
            "/api/v2/admin/featured",
            get(list_featured_cards).post(create_featured_card),
        )
        .route("/api/v2/admin/featured/upload", post(upload_featured_image))
        .route(
            "/api/v2/admin/featured/search",
            get(search_tools_for_picker),
        )
        .route(
            "/api/v2/admin/featured/{id}",
            put(update_featured_card).delete(delete_featured_card),
        )
        .route(
            "/api/v2/admin/categories",
            get(list_admin_categories).post(create_category),
        )
        .route(
            "/api/v2/admin/categories/{id}",
            put(update_category).delete(delete_category),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct PickerQuery {
    query: String,
    #[serde(default = "default_picker_limit")]
    limit: i64,
}

fn default_picker_limit() -> i64 {
    20
}

async fn get_featured_cards(
    State(state): State<AppState>,
) -> Result<Json<Vec<FeaturedCardView>>, ApiError> {
    let rows = sqlx::query_as::<_, FeaturedCardView>(GET_FEATURED_CARDS_SQL)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to load featured cards: {e}")))?;
    Ok(Json(rows))
}

async fn list_featured_cards(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AdminFeaturedCardView>>, ApiError> {
    require_admin_from(&state, &headers).await?;

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
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to list featured cards: {e}")))?;

    Ok(Json(rows))
}

async fn create_featured_card(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<FeaturedCardInput>,
) -> Result<Json<FeaturedCard>, ApiError> {
    if let Err(msg) = validate_featured_card_input(
        &input.image_url,
        input.headline.as_deref(),
        input.subtitle.as_deref(),
    ) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    require_admin_from(&state, &headers).await?;
    ensure_featured_tool_exists(&state.pool, input.tool_id).await?;

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
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to create featured card: {e}")))?;

    Ok(Json(card))
}

async fn update_featured_card(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(mut input): Json<UpdateFeaturedCardInput>,
) -> Result<Json<FeaturedCard>, ApiError> {
    input.id = id;
    if let Err(msg) = validate_featured_card_input(
        &input.image_url,
        input.headline.as_deref(),
        input.subtitle.as_deref(),
    ) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    require_admin_from(&state, &headers).await?;
    ensure_featured_tool_exists(&state.pool, input.tool_id).await?;

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
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to update featured card: {e}")))?;

    Ok(Json(card))
}

async fn delete_featured_card(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let result = sqlx::query("DELETE FROM featured_cards WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to delete featured card: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("featured card not found".into()));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn upload_featured_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let mut filename = String::from("upload");
    let mut content_type = String::new();
    let mut bytes = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("invalid multipart: {e}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" || name == "image" {
            if let Some(ct) = field.content_type() {
                content_type = ct.to_string();
            }
            if let Some(fn_) = field.file_name() {
                filename = fn_.to_string();
            }
            bytes = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("failed to read upload: {e}")))?
                .to_vec();
        }
    }

    if content_type.is_empty() {
        content_type = "application/octet-stream".into();
    }

    if let Err(msg) = validate_featured_image_upload(&content_type, bytes.len()) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    let ext = featured_image_extension(&content_type, &filename)
        .ok_or_else(|| ApiError::BadRequest("unsupported image type".into()))?;
    let object_path = format!("{}.{}", Uuid::new_v4(), ext);
    let upload_url = format!(
        "{}/storage/v1/object/featured/{}",
        state.config.supabase_url.trim_end_matches('/'),
        object_path
    );
    let public_url = format!(
        "{}/storage/v1/object/public/featured/{}",
        state.config.supabase_url.trim_end_matches('/'),
        object_path
    );

    let response = reqwest::Client::new()
        .post(&upload_url)
        .header("apikey", &state.config.supabase_service_key)
        .header(
            "Authorization",
            format!("Bearer {}", state.config.supabase_service_key),
        )
        .header("Content-Type", &content_type)
        .header("x-upsert", "true")
        .body(bytes)
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("image upload failed: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ApiError::Internal(format!(
            "storage upload failed ({status}): {body}"
        )));
    }

    Ok(Json(serde_json::json!({ "url": public_url })))
}

async fn search_tools_for_picker(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<PickerQuery>,
) -> Result<Json<Vec<ToolPickerItem>>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let query = q.query.trim();
    if query.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let limit = q.limit.clamp(1, 50);
    let pattern = format!("%{query}%");
    let rows = sqlx::query_as::<_, ToolPickerItem>(SEARCH_TOOLS_FOR_PICKER_SQL)
        .bind(pattern)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("tool search failed: {e}")))?;

    Ok(Json(rows))
}

async fn list_admin_categories(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AdminCategoryView>>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let rows = sqlx::query_as::<_, (String, String, String, String, i32, i64)>(
        r#"
        SELECT c.id, c.label, c.icon, c.description, c.sort_order,
               COUNT(t.id) AS tool_count
        FROM categories c
        LEFT JOIN tools t ON t.function = c.id
          AND t.approval_status = 'approved'
          AND t.quarantined_at IS NULL
        GROUP BY c.id, c.label, c.icon, c.description, c.sort_order
        ORDER BY c.sort_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to list categories: {e}")))?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, label, icon, description, sort_order, tool_count)| AdminCategoryView {
                    id,
                    label,
                    icon,
                    description,
                    sort_order,
                    tool_count,
                },
            )
            .collect(),
    ))
}

async fn create_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CategoryInput>,
) -> Result<Json<Category>, ApiError> {
    if let Err(msg) = validate_category_input(&input) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    require_admin_from(&state, &headers).await?;

    let category = sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO categories (id, label, icon, description, sort_order)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(input.id.trim())
    .bind(input.label.trim())
    .bind(input.icon.trim())
    .bind(input.description.trim())
    .bind(input.sort_order)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to create category: {e}")))?;

    Ok(Json(category))
}

async fn update_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(mut input): Json<CategoryInput>,
) -> Result<Json<Category>, ApiError> {
    input.id = id;
    if let Err(msg) = validate_category_input(&input) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    require_admin_from(&state, &headers).await?;

    let category = sqlx::query_as::<_, Category>(
        r#"
        UPDATE categories
        SET label = $2, icon = $3, description = $4, sort_order = $5
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(input.id.trim())
    .bind(input.label.trim())
    .bind(input.icon.trim())
    .bind(input.description.trim())
    .bind(input.sort_order)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to update category: {e}")))?
    .ok_or_else(|| ApiError::NotFound(format!("category not found: {}", input.id)))?;

    Ok(Json(category))
}

async fn delete_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let id = id.trim().to_string();
    if id.is_empty() {
        return Err(ApiError::BadRequest("category id required".into()));
    }

    require_admin_from(&state, &headers).await?;

    let tool_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tools WHERE function = $1")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("tool count failed: {e}")))?;

    if tool_count > 0 {
        return Err(ApiError::BadRequest(
            "cannot delete category with linked tools — reassign tools first".into(),
        ));
    }

    let result = sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to delete category: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("category not found: {id}")));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn ensure_featured_tool_exists(pool: &sqlx::PgPool, tool_id: Uuid) -> Result<(), ApiError> {
    let exists = sqlx::query_scalar::<_, bool>(FEATURED_TOOL_EXISTS_SQL)
        .bind(tool_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("tool lookup failed: {e}")))?;

    if exists {
        Ok(())
    } else {
        Err(ApiError::NotFound("approved tool not found".into()))
    }
}
