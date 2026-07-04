//! x402-gated premium API routes (K2).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;

use crate::server::x402_payment::{facilitator_client, require_payment, X402PaymentConfig};
use crate::server::x402_premium::check_endpoint_health;
use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v2/premium/check-endpoint-health/{slug}",
            get(get_check_endpoint_health),
        )
        .with_state(state)
}

async fn get_check_endpoint_health(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let config = X402PaymentConfig::from_env();
    let resource_url = format!("/api/v2/premium/check-endpoint-health/{slug}");
    let requirements = config.requirement_for(
        &resource_url,
        "x402 endpoint liveness, 30-day uptime, and last probe timestamp",
        "application/json",
    );

    let client = facilitator_client();
    let settlement = match require_payment(&client, &config, &headers, requirements).await {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premium_route_path_is_registered() {
        let path = "/api/v2/premium/check-endpoint-health/{slug}";
        assert!(path.contains("check-endpoint-health"));
    }
}
