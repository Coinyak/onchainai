//! Tool report and claim request endpoints.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};

use crate::models::{ToolClaimRequest, ToolReport};
use crate::server::functions::{
    build_claim_proof_note, validate_claim_tool_input, validate_report_details,
    validate_report_reason, ClaimToolInput, ReportToolInput,
};
use crate::server::queries::APPROVED_TOOL_ID_BY_SLUG_SQL;
use crate::server::review_persistence::insert_candidate_official_link;
use crate::AppState;

use super::auth::require_user_from;
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/tools/{slug}/report", post(report_tool))
        .route("/api/v2/tools/{slug}/claim", post(request_tool_claim))
        .with_state(state)
}

async fn report_tool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(mut input): Json<ReportToolInput>,
) -> Result<Json<ToolReport>, ApiError> {
    input.slug = slug;
    if let Err(msg) = validate_report_reason(input.reason.trim()) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }
    if let Err(msg) = validate_report_details(input.details.as_deref()) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    let slug = input.slug.trim();
    if slug.is_empty() {
        return Err(ApiError::BadRequest("tool slug is required".into()));
    }

    let user = require_user_from(&state, &headers).await?;

    let tool_id = sqlx::query_scalar::<_, uuid::Uuid>(APPROVED_TOOL_ID_BY_SLUG_SQL)
        .bind(slug)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to resolve tool: {e}")))?
        .ok_or_else(|| ApiError::NotFound("tool not found".into()))?;

    let details = input
        .details
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let row = sqlx::query_as::<_, ToolReport>(
        r#"
        INSERT INTO tool_reports (tool_id, reported_by, reason, details, status)
        VALUES ($1, $2, $3, $4, 'open')
        RETURNING *
        "#,
    )
    .bind(tool_id)
    .bind(user.id)
    .bind(input.reason.trim())
    .bind(details)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to save report: {e}")))?;

    Ok(Json(row))
}

async fn request_tool_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(mut input): Json<ClaimToolInput>,
) -> Result<Json<ToolClaimRequest>, ApiError> {
    input.slug = slug;
    if let Err(msg) = validate_claim_tool_input(&input) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    let user = require_user_from(&state, &headers).await?;

    let tool = claim_tool_by_slug(&state.pool, &input.slug).await?;
    validate_claim_state_available(&tool)?;
    let contact_email =
        normalized_claim_optional(input.contact_email.as_deref()).map(str::to_string);
    let proof_note =
        build_claim_proof_note(&input).map_err(|msg| ApiError::BadRequest(msg.to_string()))?;

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(format!("transaction failed: {e}")))?;
    let claim =
        insert_claim_request_row(&mut tx, tool.id, user.id, &proof_note, contact_email).await?;
    insert_claim_official_links(&mut tx, tool.id, &input).await?;
    mark_claim_pending(&mut tx, tool.id).await?;
    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(format!("commit failed: {e}")))?;

    Ok(Json(claim))
}

async fn claim_tool_by_slug(
    pool: &sqlx::PgPool,
    slug: &str,
) -> Result<crate::models::Tool, ApiError> {
    use crate::server::queries::APPROVED_TOOL_BY_SLUG_SQL;

    sqlx::query_as::<_, crate::models::Tool>(APPROVED_TOOL_BY_SLUG_SQL)
        .bind(slug.trim())
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to resolve tool: {e}")))?
        .ok_or_else(|| ApiError::NotFound("tool not found".into()))
}

fn validate_claim_state_available(tool: &crate::models::Tool) -> Result<(), ApiError> {
    match tool.claim_state.as_str() {
        "claimed" => Err(ApiError::BadRequest(
            "this listing is already claimed".into(),
        )),
        "claim_pending" => Err(ApiError::BadRequest(
            "a claim request is already pending review".into(),
        )),
        _ => Ok(()),
    }
}

fn normalized_claim_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|s| !s.is_empty())
}

async fn insert_claim_request_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tool_id: uuid::Uuid,
    user_id: uuid::Uuid,
    proof_note: &str,
    contact_email: Option<String>,
) -> Result<ToolClaimRequest, ApiError> {
    sqlx::query_as::<_, ToolClaimRequest>(
        r#"
        INSERT INTO tool_claim_requests (tool_id, requested_by, verification_note, contact_email, status)
        VALUES ($1, $2, $3, $4, 'pending')
        RETURNING *
        "#,
    )
    .bind(tool_id)
    .bind(user_id)
    .bind(proof_note)
    .bind(contact_email)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to save claim request: {e}")))
}

struct ClaimOfficialLinkCandidate<'a> {
    link_type: &'static str,
    url: Option<&'a str>,
    label: &'static str,
    source: &'static str,
}

async fn insert_claim_official_links(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tool_id: uuid::Uuid,
    input: &ClaimToolInput,
) -> Result<(), ApiError> {
    insert_claim_official_link(
        tx,
        tool_id,
        ClaimOfficialLinkCandidate {
            link_type: "github",
            url: input.github_url.as_deref(),
            label: "Claimed GitHub",
            source: "claim:github",
        },
    )
    .await?;
    insert_claim_official_link(
        tx,
        tool_id,
        ClaimOfficialLinkCandidate {
            link_type: "website",
            url: input.website_url.as_deref(),
            label: "Claimed Website",
            source: "claim:website",
        },
    )
    .await?;
    insert_claim_official_link(
        tx,
        tool_id,
        ClaimOfficialLinkCandidate {
            link_type: "x",
            url: input.x_url.as_deref(),
            label: "Claimed X",
            source: "claim:x",
        },
    )
    .await
}

async fn insert_claim_official_link(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tool_id: uuid::Uuid,
    candidate: ClaimOfficialLinkCandidate<'_>,
) -> Result<(), ApiError> {
    let Some(url) = normalized_claim_optional(candidate.url) else {
        return Ok(());
    };
    insert_candidate_official_link(
        tx,
        tool_id,
        candidate.link_type,
        url,
        candidate.label,
        candidate.source,
    )
    .await
    .map_err(ApiError::from_server_fn)
}

async fn mark_claim_pending(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tool_id: uuid::Uuid,
) -> Result<(), ApiError> {
    sqlx::query("UPDATE tools SET claim_state = 'claim_pending', updated_at = now() WHERE id = $1")
        .bind(tool_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to update claim state: {e}")))?;
    Ok(())
}
