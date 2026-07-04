//! Site settings endpoints.

use axum::{extract::State, http::HeaderMap, routing::get, Json, Router};

use crate::models::{sanitize_site_settings_for_public, SiteSettings};
use crate::server::functions::{
    parse_search_keywords, validate_update_site_settings_input, SiteSettingsValidationInput,
    UpdateSiteSettingsPayload,
};
use crate::AppState;

use super::auth::require_admin_from;
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/settings", get(get_site_settings))
        .route(
            "/api/v2/admin/settings",
            get(get_admin_site_settings).put(update_site_settings),
        )
        .with_state(state)
}

async fn get_site_settings(State(state): State<AppState>) -> Result<Json<SiteSettings>, ApiError> {
    let settings = sqlx::query_as::<_, SiteSettings>("SELECT * FROM site_settings WHERE id = 1")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to load site settings: {e}")))?;

    Ok(Json(sanitize_site_settings_for_public(settings)))
}

async fn get_admin_site_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SiteSettings>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let settings = sqlx::query_as::<_, SiteSettings>("SELECT * FROM site_settings WHERE id = 1")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to load site settings: {e}")))?;

    Ok(Json(settings))
}

async fn update_site_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateSiteSettingsPayload>,
) -> Result<Json<SiteSettings>, ApiError> {
    let keywords = parse_search_keywords(&payload.search_keywords_raw);
    if let Err(msg) = validate_update_site_settings_input(SiteSettingsValidationInput {
        site_name: &payload.site_name,
        slogan: &payload.slogan,
        description: &payload.description,
        mcp_endpoint: &payload.mcp_endpoint,
        search_keywords: &keywords,
        default_referral_bps: payload.default_referral_bps,
        default_referral_payout_address: payload.default_referral_payout_address.as_deref(),
        x402_builder_code: payload.x402_builder_code.as_deref(),
        mcp_premium_enabled: payload.mcp_premium_enabled,
        mcp_premium_pay_to_address: payload.mcp_premium_pay_to_address.as_deref(),
        mcp_premium_price: payload.mcp_premium_price.as_deref(),
        mcp_premium_network: &payload.mcp_premium_network,
        mcp_premium_asset: payload.mcp_premium_asset.as_deref(),
        mcp_premium_display_price: payload.mcp_premium_display_price.as_deref(),
        hero_title: payload.hero_title.as_deref(),
        hero_subtitle: payload.hero_subtitle.as_deref(),
        about_content: payload.about_content.as_deref(),
        footer_links: &payload.footer_links,
    }) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    require_admin_from(&state, &headers).await?;

    let settings = sqlx::query_as::<_, SiteSettings>(
        r#"
        UPDATE site_settings
        SET site_name = $1,
            slogan = $2,
            description = $3,
            mcp_endpoint = $4,
            search_keywords = $5,
            allow_free_registration = $6,
            require_tool_approval = $7,
            allow_x402_registration = $8,
            default_referral_bps = $9,
            default_referral_payout_address = $10,
            x402_builder_code = $11,
            mcp_premium_enabled = $12,
            mcp_premium_pay_to_address = $13,
            mcp_premium_price = $14,
            mcp_premium_network = $15,
            mcp_premium_asset = $16,
            mcp_premium_display_price = $17,
            hero_title = $18,
            hero_subtitle = $19,
            about_content = $20,
            footer_links = $21,
            updated_at = now()
        WHERE id = 1
        RETURNING *
        "#,
    )
    .bind(payload.site_name.trim())
    .bind(payload.slogan.trim())
    .bind(payload.description.trim())
    .bind(payload.mcp_endpoint.trim())
    .bind(&keywords)
    .bind(payload.allow_free_registration)
    .bind(payload.require_tool_approval)
    .bind(payload.allow_x402_registration)
    .bind(payload.default_referral_bps)
    .bind(
        payload
            .default_referral_payout_address
            .as_deref()
            .map(str::trim),
    )
    .bind(payload.x402_builder_code.as_deref().map(str::trim))
    .bind(payload.mcp_premium_enabled)
    .bind(
        payload
            .mcp_premium_pay_to_address
            .as_deref()
            .map(str::trim),
    )
    .bind(payload.mcp_premium_price.as_deref().map(str::trim))
    .bind(payload.mcp_premium_network.trim())
    .bind(payload.mcp_premium_asset.as_deref().map(str::trim))
    .bind(
        payload
            .mcp_premium_display_price
            .as_deref()
            .map(str::trim),
    )
    .bind(payload.hero_title.as_deref().map(str::trim))
    .bind(payload.hero_subtitle.as_deref().map(str::trim))
    .bind(payload.about_content.as_deref().map(str::trim))
    .bind(sqlx::types::Json(&payload.footer_links))
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to update site settings: {e}")))?;

    Ok(Json(settings))
}
