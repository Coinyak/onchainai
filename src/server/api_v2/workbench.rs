//! Admin workbench and trust view endpoints.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};

use crate::models::tool::sanitize_tool_for_public_response;
use crate::models::{Tool, ToolOfficialLink};
use crate::server::functions::{
    AdminToolWorkbenchView, AdminWorkbenchSummary, ToolTrustView, VerifyOfficialLinkPayload,
};
use crate::server::queries::APPROVED_TOOL_BY_SLUG_SQL;
use crate::server::review_persistence::{
    compute_tool_trust, load_tool_review_timeline, verify_official_link, VerifyOfficialLinkInput,
};
use crate::server::secret_redaction::redact_tool_for_admin;
use crate::trust_verification::{official_promotion_allowed, verify_tool_trust};
use crate::workbench::build_summary_cards;
use crate::AppState;

use super::auth::require_admin_from;
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/admin/trust/{slug}", get(get_tool_trust_view))
        .route(
            "/api/v2/admin/workbench/summary",
            get(get_admin_workbench_summary),
        )
        .route(
            "/api/v2/admin/workbench/{slug}",
            get(get_admin_tool_workbench),
        )
        .route("/api/v2/admin/verify-link", post(verify_tool_official_link))
        .with_state(state)
}

async fn get_tool_trust_view(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<ToolTrustView>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let tool = sqlx::query_as::<_, Tool>(APPROVED_TOOL_BY_SLUG_SQL)
        .bind(slug.trim())
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to load tool trust view: {e}")))?;

    let official_links =
        crate::server::review_persistence::list_public_official_links(&state.pool, tool.id)
            .await
            .map_err(ApiError::from_server_fn)?;
    let trust = verify_tool_trust(&tool, &official_links);

    Ok(Json(ToolTrustView {
        tool: sanitize_tool_for_public_response(tool),
        official_links,
        trust_facts: trust.trust_facts,
    }))
}

async fn get_admin_workbench_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminWorkbenchSummary>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let counts = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        r#"
        SELECT
          COUNT(*) FILTER (
            WHERE approval_status = 'pending'
              AND last_reviewed_at IS NULL
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE claim_state = 'claim_pending' AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status = 'approved'
              AND status = 'community'
              AND claim_state = 'claimed'
              AND quarantined_at IS NULL
          )::bigint,
          (SELECT COUNT(*)::bigint
             FROM tools t
            WHERE t.approval_status = 'approved'
              AND t.quarantined_at IS NULL
              AND t.status IN ('verified', 'official')
              AND NOT EXISTS (
                SELECT 1 FROM featured_cards fc
                WHERE fc.tool_id = t.id AND fc.is_active = true
              ))
        FROM tools
        "#,
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to load workbench summary: {e}")))?;

    Ok(Json(AdminWorkbenchSummary {
        cards: build_summary_cards(counts.0, counts.1, counts.2, counts.3),
    }))
}

async fn get_admin_tool_workbench(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<AdminToolWorkbenchView>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let tool = match sqlx::query_as::<_, Tool>("SELECT * FROM tools WHERE slug = $1")
        .bind(slug.trim())
        .fetch_one(&state.pool)
        .await
    {
        Ok(tool) => tool,
        Err(sqlx::Error::RowNotFound) => {
            return Err(ApiError::NotFound(format!(
                "tool not found: {}",
                slug.trim()
            )));
        }
        Err(e) => {
            return Err(ApiError::Internal(format!(
                "failed to load tool workbench: {e}"
            )));
        }
    };

    let (trust, official_links) = compute_tool_trust(&state.pool, &tool)
        .await
        .map_err(ApiError::from_server_fn)?;
    let review_timeline = load_tool_review_timeline(&state.pool, tool.id)
        .await
        .map_err(ApiError::from_server_fn)?;
    let promotion_ok = official_promotion_allowed(&tool, &official_links, &trust);

    Ok(Json(AdminToolWorkbenchView {
        tool: redact_tool_for_admin(tool),
        official_links,
        trust,
        timeline: review_timeline.entries,
        verdicts: review_timeline.operator_verdicts,
        official_promotion_allowed: promotion_ok,
    }))
}

async fn verify_tool_official_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<VerifyOfficialLinkPayload>,
) -> Result<Json<ToolOfficialLink>, ApiError> {
    let admin = require_admin_from(&state, &headers).await?;

    const STATUSES: &[&str] = &["candidate", "claimed", "verified", "rejected"];
    const STRENGTHS: &[&str] = &["weak", "medium", "strong"];
    if !STATUSES.contains(&payload.verification_status.as_str()) {
        return Err(ApiError::BadRequest("invalid verification status".into()));
    }
    if !STRENGTHS.contains(&payload.evidence_strength.as_str()) {
        return Err(ApiError::BadRequest("invalid evidence strength".into()));
    }
    if payload.official_badge_allowed && payload.verification_status != "verified" {
        return Err(ApiError::BadRequest(
            "official badge requires verified link status".into(),
        ));
    }

    let link = verify_official_link(
        &state.pool,
        VerifyOfficialLinkInput {
            link_id: payload.link_id,
            verification_status: payload.verification_status,
            evidence_strength: payload.evidence_strength,
            official_badge_allowed: payload.official_badge_allowed,
            verification_method: payload.verification_method,
            notes: payload.notes,
            operator_id: admin.id,
        },
    )
    .await
    .map_err(ApiError::from_server_fn)?;

    Ok(Json(link))
}
