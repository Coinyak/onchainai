//! Admin tool review endpoints.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;

use crate::models::Tool;
use crate::server::functions::{
    clamp_admin_review_list_limit, derive_claim_state, derive_lifecycle_state,
    list_crawler_sources_inner, normalize_optional_text, review_queue_sql, run_review_tool,
    validate_review_action, validate_set_tool_approval_input, validate_tool_referral_payload,
    AdminDashboardStats, DuplicateCandidateStub, ReferralDashboardStats, ReviewQueueItem,
    ReviewToolPayload, UpdateToolReferralPayload, LIST_PENDING_TOOLS_SQL,
};
use crate::server::secret_redaction::redact_tool_for_admin;
use crate::AppState;

use super::auth::require_admin_from;
use super::error::ApiError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/admin/pending", get(list_pending_tools))
        .route("/api/v2/admin/stats", get(get_admin_dashboard_stats))
        .route("/api/v2/admin/review-queue", get(list_review_queue))
        .route("/api/v2/admin/review", post(review_tool))
        .route("/api/v2/admin/approval", post(set_tool_approval))
        .route(
            "/api/v2/admin/referral-stats",
            get(get_referral_dashboard_stats),
        )
        .route("/api/v2/admin/tool-referral", put(update_tool_referral))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct LimitQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
struct ReviewQueueQuery {
    queue: String,
    #[serde(default = "default_limit")]
    limit: i64,
}

#[derive(Debug, Deserialize)]
struct SetToolApprovalBody {
    slug: String,
    status: String,
    reason: Option<String>,
}

async fn list_pending_tools(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Vec<Tool>>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let tools = sqlx::query_as::<_, Tool>(LIST_PENDING_TOOLS_SQL)
        .bind(clamp_admin_review_list_limit(q.limit))
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to list pending tools: {e}")))?;

    Ok(Json(tools))
}

async fn count_open_reports(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM tool_reports WHERE status = 'open'")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

async fn count_reported_tools(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT tool_id)::bigint FROM tool_reports WHERE status = 'open'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

async fn get_admin_dashboard_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminDashboardStats>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let counts = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>(
        r#"
        SELECT
          COUNT(*) FILTER (
            WHERE approval_status = 'pending'
              AND last_reviewed_at IS NULL
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status = 'approved'
              AND last_reviewed_at IS NOT NULL
              AND updated_at > last_reviewed_at
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status IN ('pending', 'approved')
              AND install_risk_level IN ('high', 'critical')
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status = 'approved'
              AND relevance_status = 'accepted'
              AND install_risk_level <> 'critical'
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status IN ('pending', 'approved')
              AND relevance_status = 'needs_review'
              AND crypto_relevance_score < 50
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status = 'pending'
              AND relevance_status = 'rejected'
              AND quarantined_at IS NULL
          )::bigint
        FROM tools
        "#,
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to load dashboard counts: {e}")))?;

    let open_reports = count_open_reports(&state.pool).await;
    let reported = count_reported_tools(&state.pool).await;
    let crawler_sources = list_crawler_sources_inner(&state.pool)
        .await
        .map_err(ApiError::from_server_fn)?;

    Ok(Json(AdminDashboardStats {
        pending_candidates: counts.0,
        known_updates: counts.1,
        high_risk_installs: counts.2,
        public_tool_count: counts.3,
        needs_manual_research: counts.4,
        low_relevance: counts.5,
        reported,
        open_reports,
        crawler_sources,
    }))
}

async fn list_review_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ReviewQueueQuery>,
) -> Result<Json<Vec<ReviewQueueItem>>, ApiError> {
    if review_queue_sql(&q.queue).is_err() {
        return Err(ApiError::BadRequest("unknown review queue".into()));
    }

    require_admin_from(&state, &headers).await?;

    let sql = review_queue_sql(&q.queue).expect("validated above");
    let tools = sqlx::query_as::<_, Tool>(sql)
        .bind(clamp_admin_review_list_limit(q.limit))
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to list review queue: {e}")))?;

    let mut items = Vec::with_capacity(tools.len());
    for tool in tools {
        let duplicates = fetch_duplicate_candidates(&state.pool, &tool).await?;
        items.push(ReviewQueueItem {
            lifecycle_state: derive_lifecycle_state(&tool),
            claim_state: derive_claim_state(&tool),
            duplicate_candidates: duplicates,
            tool: redact_tool_for_admin(tool),
        });
    }

    Ok(Json(items))
}

async fn fetch_duplicate_candidates(
    pool: &sqlx::PgPool,
    tool: &Tool,
) -> Result<Vec<DuplicateCandidateStub>, ApiError> {
    let repo = tool.repo_url.as_deref().unwrap_or("");
    let rows = if repo.is_empty() {
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT slug, name
            FROM tools
            WHERE id <> $1
              AND approval_status = 'pending'
              AND lower(name) = lower($2)
            ORDER BY created_at DESC
            LIMIT 3
            "#,
        )
        .bind(tool.id)
        .bind(&tool.name)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT slug, name
            FROM tools
            WHERE id <> $1
              AND approval_status = 'pending'
              AND repo_url = $2
            ORDER BY created_at DESC
            LIMIT 3
            "#,
        )
        .bind(tool.id)
        .bind(repo)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| ApiError::Internal(format!("failed to load duplicate candidates: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|(slug, name)| DuplicateCandidateStub { slug, name })
        .collect())
}

async fn review_tool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ReviewToolPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Err(msg) = validate_review_action(&payload.action, &payload.reason) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    let admin = require_admin_from(&state, &headers).await?;
    run_review_tool(&state.pool, admin.id, &payload)
        .await
        .map_err(ApiError::from_server_fn)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn set_tool_approval(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SetToolApprovalBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Err(msg) = validate_set_tool_approval_input(&body.status, body.reason.as_deref()) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    let review_reason = match body.reason {
        Some(r) if !r.trim().is_empty() => r,
        _ if body.status == "approved" => "Approved via legacy set_tool_approval".into(),
        _ => String::new(),
    };

    let admin = require_admin_from(&state, &headers).await?;
    run_review_tool(
        &state.pool,
        admin.id,
        &ReviewToolPayload {
            slug: body.slug,
            action: body.status,
            reason: review_reason,
            override_reason: None,
            expected_updated_at: None,
            snapshot_id: None,
            recommendation_id: None,
        },
    )
    .await
    .map_err(ApiError::from_server_fn)?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn get_referral_dashboard_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ReferralDashboardStats>, ApiError> {
    require_admin_from(&state, &headers).await?;

    let stats = sqlx::query_as::<_, ReferralDashboardStats>(
        r#"
        SELECT
            (SELECT COUNT(*) FROM tools WHERE pricing = 'x402') AS x402_tools,
            (SELECT COUNT(*) FROM tools WHERE referral_enabled = true) AS referral_enabled_tools,
            (SELECT COUNT(*) FROM referral_events) AS attribution_events,
            (SELECT COUNT(*) FROM referral_events WHERE event_type = 'reported_settlement') AS reported_settlements
        "#,
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to load referral stats: {e}")))?;

    Ok(Json(stats))
}

async fn update_tool_referral(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateToolReferralPayload>,
) -> Result<Json<Tool>, ApiError> {
    if let Err(msg) = validate_tool_referral_payload(&payload) {
        return Err(ApiError::BadRequest(msg.to_string()));
    }

    require_admin_from(&state, &headers).await?;

    let tool = sqlx::query_as::<_, Tool>(
        r#"
        UPDATE tools
        SET referral_enabled = $1,
            referral_bps = $2,
            referral_payout_address = $3,
            referral_model = $4,
            x402_pay_to_address = $5,
            x402_builder_code = $6,
            payment_verified = $7,
            x402_endpoint_verified = $8,
            price_verified = $9,
            updated_at = now()
        WHERE slug = $10
        RETURNING *
        "#,
    )
    .bind(payload.referral_enabled)
    .bind(payload.referral_bps)
    .bind(normalize_optional_text(payload.referral_payout_address))
    .bind(normalize_optional_text(payload.referral_model))
    .bind(normalize_optional_text(payload.x402_pay_to_address))
    .bind(normalize_optional_text(payload.x402_builder_code))
    .bind(payload.payment_verified)
    .bind(payload.x402_endpoint_verified)
    .bind(payload.price_verified)
    .bind(payload.slug.trim())
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to update referral settings: {e}")))?
    .ok_or_else(|| ApiError::NotFound(format!("tool not found: {}", payload.slug)))?;

    Ok(Json(redact_tool_for_admin(tool)))
}
