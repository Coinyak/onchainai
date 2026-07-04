//! Tool submission endpoints.

use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};

use crate::crawler::normalizer::base_slug;
use crate::models::{ToolSubmission, ToolSubmissionPayload};
use crate::server::functions::{
    parse_submission_chains, scan_submission, validate_submit_tool_input, SubmitToolInput,
};
use crate::server::rate_limit::{check_user_rate_limit, UserRateLimitAction};
use crate::AppState;

use super::auth::require_user_from;
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/submit", post(submit_tool))
        .route("/api/v2/my-submissions", get(list_my_submissions))
        .with_state(state)
}

async fn submit_tool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<SubmitToolInput>,
) -> Result<Json<ToolSubmission>, ApiError> {
    if let Err(msg) = validate_submit_tool_input(&input) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    let user = require_user_from(&state, &headers).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::SubmitTool) {
        return Err(ApiError::BadRequest(limit.to_string()));
    }

    // x402 listings go through the probe-gated open-listing flow, and only when
    // the operator switch is on (X402_OPEN_LISTING_SPEC §L1 / activation spec X8).
    if input.tool_type.trim() == "x402" {
        let allow = sqlx::query_scalar::<_, bool>(
            "SELECT allow_x402_registration FROM site_settings LIMIT 1",
        )
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to load site settings: {e}")))?
        .unwrap_or(false);
        if !allow {
            return Err(ApiError::BadRequest(
                "x402 submissions are currently disabled".into(),
            ));
        }
    }

    let scan = scan_submission(&input);
    let chains = parse_submission_chains(&input.chains_raw);
    let slug = base_slug(input.name.trim());

    let duplicate_count = duplicate_submission_count(&state.pool, &slug)
        .await
        .map_err(ApiError::from_server_fn)?;
    if duplicate_count > 0 {
        return Err(ApiError::BadRequest(
            "a similar tool is already listed or pending review".into(),
        ));
    }

    let payload = submission_payload(&input, chains, slug);
    let payload_json = serde_json::to_value(&payload)
        .map_err(|e| ApiError::Internal(format!("failed to encode submission: {e}")))?;

    let row = sqlx::query_as::<_, ToolSubmission>(
        r#"
        INSERT INTO tool_submissions (
          submitted_by, status, payload,
          crypto_relevance_score, relevance_status, install_risk_level
        )
        VALUES ($1, 'pending', $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(user.id)
    .bind(payload_json)
    .bind(scan.crypto_relevance_score)
    .bind(scan.relevance_status)
    .bind(scan.install_risk_level)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to save submission: {e}")))?;

    Ok(Json(row))
}

async fn list_my_submissions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ToolSubmission>>, ApiError> {
    let user = require_user_from(&state, &headers).await?;

    let rows = sqlx::query_as::<_, ToolSubmission>(
        r#"
        SELECT * FROM tool_submissions
        WHERE submitted_by = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to list submissions: {e}")))?;

    Ok(Json(rows))
}

async fn duplicate_submission_count(
    pool: &sqlx::PgPool,
    slug: &str,
) -> Result<i64, crate::server::fn_error::FnError> {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint FROM (
          SELECT slug FROM tools WHERE lower(slug) = lower($1)
          UNION ALL
          SELECT payload->>'slug' FROM tool_submissions
            WHERE status IN ('pending', 'needs_info')
              AND lower(payload->>'slug') = lower($1)
        ) d
        "#,
    )
    .bind(slug)
    .fetch_one(pool)
    .await
    .map_err(|e| crate::server::fn_error::FnError::new(format!("duplicate check failed: {e}")))
}

fn normalized_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn submission_payload(
    input: &SubmitToolInput,
    chains: Vec<String>,
    slug: String,
) -> ToolSubmissionPayload {
    ToolSubmissionPayload {
        name: input.name.trim().to_string(),
        description: input.description.trim().to_string(),
        tool_type: input.tool_type.trim().to_string(),
        function: input.function.trim().to_string(),
        repo_url: normalized_optional_string(input.repo_url.as_deref()),
        homepage: normalized_optional_string(input.homepage.as_deref()),
        npm_package: normalized_optional_string(input.npm_package.as_deref()),
        mcp_endpoint: normalized_optional_string(input.mcp_endpoint.as_deref()),
        install_command: normalized_optional_string(input.install_command.as_deref()),
        chains,
        category_suggestion: normalized_optional_string(input.category_suggestion.as_deref()),
        official_team_claim: input.official_team_claim,
        verification_note: normalized_optional_string(input.verification_note.as_deref()),
        slug,
        x402_endpoint_url: None,
    }
}
