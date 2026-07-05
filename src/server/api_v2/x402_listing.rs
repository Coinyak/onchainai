//! x402 open self-serve listing: probe preview + auto-publish.
//!
//! Spec: docs/X402_OPEN_LISTING_SPEC.md §L1. The probe (402 handshake) replaces
//! human review for x402 listings; verification flags stay trust signals only.
//! Registration records terms consent (§M1) — no custody, no payment execution.

use axum::{extract::State, http::HeaderMap, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crawler::normalizer::base_slug;
use crate::models::ToolSubmissionPayload;
use crate::server::functions::SUBMIT_FUNCTIONS;
use crate::server::rate_limit::{check_user_rate_limit, UserRateLimitAction};
use crate::server::x402_verify::{
    probe_x402_details, validate_probe_url, ProbeDetailsOutcome, X402ProbeDetails,
};
use crate::AppState;

use super::auth::require_user_from;
use super::error::ApiError;

/// Current self-listing terms version. Bump when the public terms copy changes.
pub const X402_LISTING_TERMS_VERSION: &str = "x402-open-listing-v1";

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/x402/probe", post(probe_endpoint))
        .route("/api/v2/x402/submit", post(submit_listing))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ProbeRequest {
    url: String,
}

#[derive(Debug, Serialize)]
struct ProbeResponse {
    live: bool,
    status: &'static str,
    details: Option<X402ProbeDetails>,
    reason: Option<String>,
    http_status: Option<u16>,
}

fn probe_response(outcome: ProbeDetailsOutcome) -> ProbeResponse {
    match outcome {
        ProbeDetailsOutcome::Live(details) => ProbeResponse {
            live: true,
            status: "live",
            details: Some(details),
            reason: None,
            http_status: Some(402),
        },
        ProbeDetailsOutcome::NotPaymentRequired { status, .. } => ProbeResponse {
            live: false,
            status: "not_payment_required",
            details: None,
            reason: Some(format!(
                "endpoint returned HTTP {status}, expected 402 Payment Required"
            )),
            http_status: Some(status),
        },
        ProbeDetailsOutcome::ParseFailed => ProbeResponse {
            live: false,
            status: "parse_failed",
            details: None,
            reason: Some("402 response did not include parseable x402 payment requirements".into()),
            http_status: Some(402),
        },
        ProbeDetailsOutcome::SsrfBlocked(reason) => ProbeResponse {
            live: false,
            status: "blocked",
            details: None,
            reason: Some(reason),
            http_status: None,
        },
        ProbeDetailsOutcome::RequestFailed(reason) => ProbeResponse {
            live: false,
            status: "request_failed",
            details: None,
            reason: Some(reason),
            http_status: None,
        },
    }
}

fn probe_history_status(response: &ProbeResponse) -> &'static str {
    match response.status {
        "live" => "live",
        "parse_failed" | "blocked" => "invalid",
        _ => "dead",
    }
}

async fn record_probe_history(
    pool: &sqlx::PgPool,
    tool_id: Option<Uuid>,
    endpoint_url: &str,
    status: &str,
    http_status: Option<u16>,
    actual_price: Option<&str>,
    latency_ms: i32,
) {
    let result = sqlx::query(
        r#"
        INSERT INTO x402_probe_history (tool_id, endpoint_url, status, http_status, actual_price, latency_ms)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(tool_id)
    .bind(endpoint_url)
    .bind(status)
    .bind(http_status.map(i32::from))
    .bind(actual_price)
    .bind(latency_ms)
    .execute(pool)
    .await;
    if let Err(e) = result {
        tracing::warn!("x402 probe history insert failed: {e}");
    }
}

/// POST /api/v2/x402/probe — SSRF-guarded preview of an endpoint's 402 handshake.
async fn probe_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ProbeRequest>,
) -> Result<Json<ProbeResponse>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::X402Probe) {
        return Err(ApiError::TooManyRequests(limit.to_string()));
    }
    let url = input.url.trim().to_string();
    validate_probe_url(&url).map_err(ApiError::BadRequest)?;

    let started = std::time::Instant::now();
    let outcome = probe_x402_details(&url).await;
    let latency_ms = started.elapsed().as_millis().min(i32::MAX as u128) as i32;

    let response = probe_response(outcome);
    let actual = response.details.as_ref().and_then(|d| d.amount.as_deref());
    record_probe_history(
        &state.pool,
        None,
        &url,
        probe_history_status(&response),
        response.http_status,
        actual,
        latency_ms,
    )
    .await;

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct X402SubmitRequest {
    name: String,
    description: String,
    endpoint_url: String,
    #[serde(default)]
    function: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    repo_url: Option<String>,
    terms_version: String,
    terms_accepted: bool,
}

#[derive(Debug, Serialize)]
struct X402SubmitResponse {
    published: bool,
    slug: Option<String>,
    tool_id: Option<Uuid>,
    submission_id: Option<Uuid>,
    probe: ProbeResponse,
}

#[derive(Debug, sqlx::FromRow)]
struct ListingSettings {
    allow_x402_registration: bool,
    default_referral_bps: Option<i32>,
}

async fn listing_settings(pool: &sqlx::PgPool) -> Result<ListingSettings, ApiError> {
    sqlx::query_as::<_, ListingSettings>(
        "SELECT allow_x402_registration, default_referral_bps FROM site_settings LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to load site settings: {e}")))?
    .ok_or_else(|| ApiError::Internal("site settings row missing".into()))
}

fn validate_submit_request(input: &X402SubmitRequest) -> Result<(), ApiError> {
    let name = input.name.trim();
    if name.len() < 2 || name.len() > 100 {
        return Err(ApiError::BadRequest("name must be 2–100 characters".into()));
    }
    let description = input.description.trim();
    if description.len() < 20 || description.len() > 500 {
        return Err(ApiError::BadRequest(
            "description must be 20–500 characters".into(),
        ));
    }
    if !input.terms_accepted {
        return Err(ApiError::BadRequest(
            "listing terms must be accepted".into(),
        ));
    }
    if input.terms_version.trim() != X402_LISTING_TERMS_VERSION {
        return Err(ApiError::BadRequest(format!(
            "unknown terms version (expected {X402_LISTING_TERMS_VERSION})"
        )));
    }
    if let Some(function) = input.function.as_deref() {
        if !SUBMIT_FUNCTIONS.contains(&function.trim()) {
            return Err(ApiError::BadRequest("unknown function".into()));
        }
    }
    for optional_url in [input.homepage.as_deref(), input.repo_url.as_deref()] {
        if let Some(raw) = optional_url.map(str::trim).filter(|s| !s.is_empty()) {
            if raw.len() > 500 || !raw.starts_with("https://") {
                return Err(ApiError::BadRequest("links must use https://".into()));
            }
        }
    }
    validate_probe_url(input.endpoint_url.trim()).map_err(ApiError::BadRequest)?;
    Ok(())
}

async fn reject_duplicates(
    pool: &sqlx::PgPool,
    slug: &str,
    endpoint_url: &str,
) -> Result<(), ApiError> {
    let duplicate_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint FROM (
          SELECT slug FROM tools
            WHERE lower(slug) = lower($1)
               OR (x402_endpoint IS NOT NULL AND lower(x402_endpoint) = lower($2))
          UNION ALL
          SELECT payload->>'slug' FROM tool_submissions
            WHERE status IN ('pending', 'needs_info')
              AND (lower(payload->>'slug') = lower($1)
                   OR lower(payload->>'x402_endpoint_url') = lower($2))
        ) d
        "#,
    )
    .bind(slug)
    .bind(endpoint_url)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("duplicate check failed: {e}")))?;
    if duplicate_count > 0 {
        return Err(ApiError::BadRequest(
            "a tool with this name or endpoint is already listed or pending review".into(),
        ));
    }
    Ok(())
}

fn normalized_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn display_price(details: &X402ProbeDetails) -> Option<String> {
    let amount = details.amount.as_deref()?.trim();
    if amount.is_empty() {
        return None;
    }
    match details
        .asset
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty())
    {
        Some(asset) => Some(format!("{amount} ({asset})")),
        None => Some(amount.to_string()),
    }
}

/// POST /api/v2/x402/submit — probe-gated self-listing (auto-publish on live 402).
async fn submit_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<X402SubmitRequest>,
) -> Result<Json<X402SubmitResponse>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    if let Err(limit) = check_user_rate_limit(user.id, UserRateLimitAction::SubmitTool) {
        return Err(ApiError::TooManyRequests(limit.to_string()));
    }

    let settings = listing_settings(&state.pool).await?;
    if !settings.allow_x402_registration {
        return Err(ApiError::BadRequest(
            "x402 self-listing is currently disabled".into(),
        ));
    }

    validate_submit_request(&input)?;
    let endpoint_url = input.endpoint_url.trim().to_string();
    let name = input.name.trim().to_string();
    let slug = base_slug(&name);
    reject_duplicates(&state.pool, &slug, &endpoint_url).await?;

    let started = std::time::Instant::now();
    let outcome = probe_x402_details(&endpoint_url).await;
    let latency_ms = started.elapsed().as_millis().min(i32::MAX as u128) as i32;
    let probe = probe_response(outcome);

    let function =
        normalized_optional(input.function.as_deref()).unwrap_or_else(|| "payments".to_string());
    let payload = ToolSubmissionPayload {
        name: name.clone(),
        description: input.description.trim().to_string(),
        tool_type: "x402".into(),
        function: function.clone(),
        repo_url: normalized_optional(input.repo_url.as_deref()),
        homepage: normalized_optional(input.homepage.as_deref()),
        npm_package: None,
        mcp_endpoint: None,
        install_command: None,
        chains: probe
            .details
            .as_ref()
            .and_then(|d| d.network.clone())
            .into_iter()
            .collect(),
        category_suggestion: None,
        official_team_claim: false,
        verification_note: None,
        slug: slug.clone(),
        x402_endpoint_url: Some(endpoint_url.clone()),
    };
    let payload_json = serde_json::to_value(&payload)
        .map_err(|e| ApiError::Internal(format!("failed to encode submission: {e}")))?;

    let Some(details) = probe.details.clone() else {
        // Probe failed: keep the submission pending with the failure reason for the operator.
        let submission_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO tool_submissions (
              submitted_by, status, payload, relevance_status, install_risk_level,
              rejection_reason, terms_version, terms_accepted_at
            )
            VALUES ($1, 'pending', $2, 'needs_review', 'low', $3, $4, now())
            RETURNING id
            "#,
        )
        .bind(user.id)
        .bind(&payload_json)
        .bind(probe.reason.as_deref())
        .bind(X402_LISTING_TERMS_VERSION)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("failed to save submission: {e}")))?;

        record_probe_history(
            &state.pool,
            None,
            &endpoint_url,
            probe_history_status(&probe),
            probe.http_status,
            None,
            latency_ms,
        )
        .await;

        return Ok(Json(X402SubmitResponse {
            published: false,
            slug: None,
            tool_id: None,
            submission_id: Some(submission_id),
            probe,
        }));
    };

    // Live 402: auto-publish. The handshake itself is the review (spec §L1).
    let price = display_price(&details);
    let chains: Vec<String> = details.network.clone().into_iter().collect();
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(format!("failed to open transaction: {e}")))?;

    let tool_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO tools (
            name, slug, description, function, type, actor,
            homepage, repo_url, chains, status,
            approval_status, submitted_by,
            crypto_relevance_score, crypto_relevance_reasons, relevance_status,
            install_risk_level, pricing, x402_price,
            x402_endpoint, x402_pay_to_address,
            x402_endpoint_verified, price_verified,
            x402_last_checked_at, x402_check_failures,
            referral_enabled, referral_bps, referral_model,
            source, source_url
        )
        VALUES (
            $1, $2, $3, $4, 'x402', 'ai-agent',
            $5, $6, $7, 'community',
            'approved', $8,
            80, ARRAY['x402 endpoint returned a live 402 payment handshake'], 'accepted',
            'low', 'x402', $9,
            $10, $11,
            true, true,
            now(), 0,
            true, $12, 'attribution',
            'self_listing', $10
        )
        RETURNING id
        "#,
    )
    .bind(&name)
    .bind(&slug)
    .bind(input.description.trim())
    .bind(&function)
    .bind(payload.homepage.as_deref())
    .bind(payload.repo_url.as_deref())
    .bind(&chains)
    .bind(user.id)
    .bind(price.as_deref())
    .bind(&endpoint_url)
    .bind(details.pay_to.as_deref())
    .bind(settings.default_referral_bps)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to publish listing: {e}")))?;

    let submission_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO tool_submissions (
          submitted_by, status, payload, relevance_status, install_risk_level,
          terms_version, terms_accepted_at
        )
        VALUES ($1, 'approved', $2, 'accepted', 'low', $3, now())
        RETURNING id
        "#,
    )
    .bind(user.id)
    .bind(&payload_json)
    .bind(X402_LISTING_TERMS_VERSION)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to record submission: {e}")))?;

    sqlx::query(
        r#"
        INSERT INTO listing_agreements (tool_id, user_id, terms_version, referral_bps, model)
        VALUES ($1, $2, $3, $4, 'attribution')
        "#,
    )
    .bind(tool_id)
    .bind(user.id)
    .bind(X402_LISTING_TERMS_VERSION)
    .bind(settings.default_referral_bps)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(format!("failed to record listing agreement: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(format!("failed to commit listing: {e}")))?;

    record_probe_history(
        &state.pool,
        Some(tool_id),
        &endpoint_url,
        "live",
        probe.http_status,
        details.amount.as_deref(),
        latency_ms,
    )
    .await;

    tracing::info!(%tool_id, slug = %slug, "x402 self-listing auto-published");

    Ok(Json(X402SubmitResponse {
        published: true,
        slug: Some(slug),
        tool_id: Some(tool_id),
        submission_id: Some(submission_id),
        probe,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_request() -> X402SubmitRequest {
        X402SubmitRequest {
            name: "Test Paid API".into(),
            description: "A paid x402 endpoint for agent-native market data.".into(),
            endpoint_url: "https://api.example.com/x402/quote".into(),
            function: None,
            homepage: None,
            repo_url: None,
            terms_version: X402_LISTING_TERMS_VERSION.into(),
            terms_accepted: true,
        }
    }

    #[test]
    fn validate_accepts_minimal_valid_request() {
        assert!(validate_submit_request(&base_request()).is_ok());
    }

    #[test]
    fn validate_rejects_unaccepted_terms() {
        let mut input = base_request();
        input.terms_accepted = false;
        assert!(validate_submit_request(&input).is_err());
    }

    #[test]
    fn validate_rejects_wrong_terms_version() {
        let mut input = base_request();
        input.terms_version = "x402-open-listing-v0".into();
        assert!(validate_submit_request(&input).is_err());
    }

    #[test]
    fn validate_rejects_http_endpoint_and_short_description() {
        let mut input = base_request();
        input.endpoint_url = "http://api.example.com/x402".into();
        assert!(validate_submit_request(&input).is_err());

        let mut input = base_request();
        input.description = "too short".into();
        assert!(validate_submit_request(&input).is_err());
    }

    #[test]
    fn validate_rejects_unknown_function_and_bad_links() {
        let mut input = base_request();
        input.function = Some("casino".into());
        assert!(validate_submit_request(&input).is_err());

        let mut input = base_request();
        input.homepage = Some("http://example.com".into());
        assert!(validate_submit_request(&input).is_err());
    }

    #[test]
    fn display_price_joins_amount_and_asset() {
        let details = X402ProbeDetails {
            amount: Some("1000".into()),
            asset: Some("usdc".into()),
            network: Some("base".into()),
            pay_to: Some("0xabc".into()),
            description: None,
        };
        assert_eq!(display_price(&details).as_deref(), Some("1000 (usdc)"));
    }

    #[test]
    fn probe_history_status_maps_outcomes() {
        let live = probe_response(ProbeDetailsOutcome::Live(X402ProbeDetails {
            amount: None,
            asset: None,
            network: None,
            pay_to: None,
            description: None,
        }));
        assert_eq!(probe_history_status(&live), "live");
        assert_eq!(live.http_status, Some(402));
        let dead = probe_response(ProbeDetailsOutcome::NotPaymentRequired {
            status: 404,
            body_snippet: Some("not found".into()),
        });
        assert_eq!(probe_history_status(&dead), "dead");
        assert_eq!(dead.http_status, Some(404));
        assert!(dead
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("HTTP 404")));
        assert!(!dead
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("not found")));
        let blocked = probe_response(ProbeDetailsOutcome::SsrfBlocked("blocked host".into()));
        assert_eq!(probe_history_status(&blocked), "invalid");
        assert_eq!(blocked.http_status, None);
    }
}
