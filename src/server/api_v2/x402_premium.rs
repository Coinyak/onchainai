//! x402-gated premium API routes (K2 + Product A + S0 gap_audit + M3 analytics).

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use crate::server::gap_audit::{
    gap_cache_get, gap_cache_key, gap_cache_set, run_gap_audit, validate_gap_audit_intent,
    GapAuditError,
};
use crate::server::m3_analytics::{get_price_history, get_x402_trends};
use crate::server::mcp_search::{mcp_search_tools, McpSearchSort};
use crate::server::mcp_x402::{load_mcp_premium_config, require_axis_b_payment};
use crate::server::product_a::{
    cache_get, cache_key, cache_set, recommend_verified_tool, validate_intent, ProductAError,
    ProductAResponse, PRODUCT_A_DISCLAIMER,
};
use crate::server::x402_payment::{facilitator_client, require_payment, X402PaymentConfig};
use crate::server::x402_premium::check_endpoint_health;
use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v2/premium/check-endpoint-health/{slug}",
            get(get_check_endpoint_health),
        )
        .route(
            "/api/v2/premium/recommend-verified-tool",
            post(post_recommend_verified_tool),
        )
        .route("/api/v2/premium/gap-audit", post(post_gap_audit))
        .route(
            "/api/v2/premium/price-history/{slug}",
            get(get_price_history_route),
        )
        .route("/api/v2/premium/x402-trends", get(get_x402_trends_route))
        .with_state(state)
}

async fn get_check_endpoint_health(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    // When OKX is active, use OKX handler-level gate instead of CDP.
    if state.okx_premium_gate_active {
        if let Some(okx_client) = &state.okx_client {
            let settlement =
                match crate::server::okx_payment::require_okx_payment(
                    okx_client,
                    "check_endpoint_health",
                    "x402 endpoint liveness probe",
                    &headers,
                )
                .await
                {
                    Ok(s) => s,
                    Err(resp) => return resp,
                };

            return match check_endpoint_health(&state.pool, &slug).await {
                Ok(report) => {
                    let body = json!({
                        "data": report,
                        "payment": {
                            "payer": settlement.payer,
                            "transaction": settlement.transaction,
                            "price": crate::server::okx_payment::okx_price_display(),
                        }
                    });
                    crate::server::okx_payment::okx_payment_success_response(body, &settlement)
                }
                Err(err) => {
                    (err.status_code(), Json(json!({ "error": err.message() }))).into_response()
                }
            };
        }
    }

    // CDP/Base fallback
    let config = X402PaymentConfig::from_env();
    let resource_url = format!("/api/v2/premium/check-endpoint-health/{slug}");
    let requirements = config.requirement_for(
        &resource_url,
        "x402 endpoint liveness, 30-day uptime, and last probe timestamp",
        "application/json",
    );

    let client = facilitator_client();
    let settlement = match require_payment(
        &client,
        &config,
        &headers,
        requirements,
        Some(crate::server::x402_payment::CHECK_ENDPOINT_HEALTH_PAYMENT_HINT),
    )
    .await
    {
        Ok(s) => s,
        Err(resp) => return resp,
    };

    match check_endpoint_health(&state.pool, &slug).await {
        Ok(report) => {
            let body = json!({
                "data": report,
                "payment": {
                    "payer": settlement.payer,
                    "transaction": settlement.transaction,
                    "price": config.price_display,
                }
            });
            match crate::server::x402_payment::payment_success_response(body.clone(), &settlement) {
                Ok(resp) => resp,
                Err(_) => (axum::http::StatusCode::OK, Json(body)).into_response(),
            }
        }
        Err(err) => (err.status_code(), Json(json!({ "error": err.message() }))).into_response(),
    }
}

/// Product A REST endpoint — Axis-B premium (same gate as export_toolkit).
#[derive(serde::Deserialize)]
struct RecommendVerifiedToolRequest {
    intent: String,
    chain: Option<String>,
    function: Option<String>,
}

async fn post_recommend_verified_tool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RecommendVerifiedToolRequest>,
) -> Response {
    let intent = match validate_intent(&payload.intent) {
        Ok(v) => v,
        Err(e) => return (e.status_code(), Json(json!({ "error": e.message() }))).into_response(),
    };
    let now = chrono::Utc::now();
    let ckey = cache_key(
        &intent,
        payload.chain.as_deref(),
        payload.function.as_deref(),
    );

    // Axis-B premium gate before cache — avoid serving paid probe results without payment.
    // Skip CDP handler-level gate when OKX middleware handles this route (prevents double-charge).
    let config = match load_mcp_premium_config(&state.pool).await {
        Ok(config) => config,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("settings load failed: {e}") })),
            )
                .into_response()
        }
    };
    if config.is_active()
        && !crate::server::okx_payment::should_skip_cdp_for_okx(
            state.okx_premium_gate_active,
            "recommend_verified_tool",
        )
    {
        match require_axis_b_payment(&config, "recommend_verified_tool", &headers).await {
            Ok(_settlement) => {}
            Err(response) => return response,
        }
    }

    if let Some(cached) = cache_get(&ckey, now) {
        return (axum::http::StatusCode::OK, Json(json!(cached))).into_response();
    }

    // Extract candidates via free search.
    let search_page = match mcp_search_tools(
        &state.pool,
        &intent,
        payload.function.clone(),
        payload.chain.clone(),
        McpSearchSort::Trust,
        10,
        0,
    )
    .await
    {
        Ok(page) => page,
        Err((code, msg)) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": msg, "code": code })),
            )
                .into_response()
        }
    };

    let candidate_slugs: Vec<String> = search_page.tools.iter().map(|t| t.slug.clone()).collect();

    if candidate_slugs.is_empty() {
        let response = ProductAResponse {
            verified_tool: None,
            rejected: vec![],
            disclaimer: PRODUCT_A_DISCLAIMER,
            probed_at: now,
            cached: None,
        };
        cache_set(ckey, response.clone(), now);
        return (axum::http::StatusCode::OK, Json(json!(response))).into_response();
    }

    match recommend_verified_tool(&state.pool, &candidate_slugs).await {
        Ok(result) => {
            cache_set(ckey, result.clone(), now);
            (axum::http::StatusCode::OK, Json(json!(result))).into_response()
        }
        Err(ProductAError::NoCandidates) => {
            let response = ProductAResponse {
                verified_tool: None,
                rejected: vec![],
                disclaimer: PRODUCT_A_DISCLAIMER,
                probed_at: now,
                cached: None,
            };
            cache_set(ckey, response.clone(), now);
            (axum::http::StatusCode::OK, Json(json!(response))).into_response()
        }
        Err(e) => (e.status_code(), Json(json!({ "error": e.message() }))).into_response(),
    }
}

/// S0 gap_audit REST endpoint — Axis-B premium.
#[derive(serde::Deserialize)]
struct GapAuditRequest {
    intent: String,
}

async fn post_gap_audit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<GapAuditRequest>,
) -> Response {
    let intent = match validate_gap_audit_intent(&payload.intent) {
        Ok(v) => v,
        Err(e) => return (e.status_code(), Json(json!({ "error": e.message() }))).into_response(),
    };
    let now = chrono::Utc::now();
    let ckey = gap_cache_key(&intent);

    let config = match load_mcp_premium_config(&state.pool).await {
        Ok(config) => config,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("settings load failed: {e}") })),
            )
                .into_response()
        }
    };
    if config.is_active()
        && !crate::server::okx_payment::should_skip_cdp_for_okx(
            state.okx_premium_gate_active,
            "gap_audit",
        )
    {
        match require_axis_b_payment(&config, "gap_audit", &headers).await {
            Ok(_settlement) => {}
            Err(response) => return response,
        }
    }

    if let Some(cached) = gap_cache_get(&ckey, now) {
        return (axum::http::StatusCode::OK, Json(json!(cached))).into_response();
    }

    match run_gap_audit(&state.pool, &intent).await {
        Ok(result) => {
            gap_cache_set(ckey, result.clone(), now);
            (axum::http::StatusCode::OK, Json(json!(result))).into_response()
        }
        Err(GapAuditError::InvalidIntent) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "error": GapAuditError::InvalidIntent.message() })),
        )
            .into_response(),
        Err(GapAuditError::Database(msg)) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": msg })),
        )
            .into_response(),
    }
}

/// M3 price history REST endpoint — free discovery/metadata (OD-FTG §2).
#[derive(serde::Deserialize)]
struct DaysQuery {
    days: Option<i64>,
}

async fn get_price_history_route(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<DaysQuery>,
) -> Response {
    match get_price_history(&state.pool, &slug, q.days).await {
        Ok(result) => (axum::http::StatusCode::OK, Json(json!(result))).into_response(),
        Err(e) => (e.status_code(), Json(json!({ "error": e.message() }))).into_response(),
    }
}

/// M3 x402 trends REST endpoint — free discovery/metadata (OD-FTG §2).
async fn get_x402_trends_route(
    State(state): State<AppState>,
    Query(q): Query<DaysQuery>,
) -> Response {
    match get_x402_trends(&state.pool, q.days).await {
        Ok(result) => (axum::http::StatusCode::OK, Json(json!(result))).into_response(),
        Err(e) => (e.status_code(), Json(json!({ "error": e.message() }))).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premium_route_path_is_registered() {
        let path = "/api/v2/premium/check-endpoint-health/{slug}";
        assert!(path.contains("check-endpoint-health"));
    }

    #[test]
    fn product_a_route_path_is_registered() {
        let path = "/api/v2/premium/recommend-verified-tool";
        assert!(path.contains("recommend-verified-tool"));
    }

    #[test]
    fn gap_audit_route_path_is_registered() {
        let path = "/api/v2/premium/gap-audit";
        assert!(path.contains("gap-audit"));
    }

    #[test]
    fn price_history_route_path_is_registered() {
        let path = "/api/v2/premium/price-history/{slug}";
        assert!(path.contains("price-history"));
    }

    #[test]
    fn x402_trends_route_path_is_registered() {
        let path = "/api/v2/premium/x402-trends";
        assert!(path.contains("x402-trends"));
    }
}
