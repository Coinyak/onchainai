//! OKX Agent Payments Protocol integration — A2MCP seller-side payment gate.
//!
//! Uses the official OKX x402 SDK crates with the OKX facilitator
//! (HMAC-SHA256 auth) on X Layer (eip155:196). CDP/Axis B gates on public
//! `POST /mcp` live in `x402_payment` / `mcp_x402`. This module meters the
//! OKX marketplace package path (`POST /mcp/okx`) and premium REST routes.
//!
//! Two gate layers:
//! 1. **Route middleware** (`x402_axum::payment_middleware`) — intercepts REST
//!    routes by exact `"METHOD /path"` match. Covers `/api/v2/premium/*` REST.
//! 2. **Handler-level gate** (`require_okx_payment`) — for `POST /mcp/okx`
//!    JSON-RPC only (public `POST /mcp` stays free discovery). Middleware cannot
//!    inspect the tool name inside the body.
//!    Analogous to `require_axis_b_payment` but uses `OkxHttpFacilitatorClient`.
//!
//! Spec: OKX Agent Payments Protocol — https://web3.okx.com/onchainos/dev-docs/payments/app

use std::collections::HashMap;
use std::sync::Arc;

use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use serde_json::json;

use x402_axum::{AcceptConfig, RoutePaymentConfig, RoutesConfig, X402ResourceServer};
use x402_core::facilitator::FacilitatorClient;
use x402_core::http::OkxHttpFacilitatorClient;
use x402_core::http::{decode_payment_signature_header, encode_payment_required_header};
use x402_core::types::{
    PaymentRequired, PaymentRequirements, ResourceInfo, SettleRequest, VerifyRequest,
};
use x402_evm::ExactEvmScheme;

/// X Layer mainnet CAIP-2 identifier.
pub const OKX_NETWORK: &str = "eip155:196";

/// USDT0 token contract on X Layer (6 decimals, EIP-3009).
const XLAYER_USDT0_ADDRESS: &str = "0x779ded0c9e1022225f8e0630b35a9b54be713736";

/// Default price per call for OKX A2MCP premium endpoints (single price for all tools).
const DEFAULT_OKX_PRICE: &str = "$0.1";

/// OKX facilitator base URL (production).
const OKX_FACILITATOR_URL: &str = "https://web3.okx.com";

/// Initialize the OKX x402 resource server.
///
/// Returns `None` if OKX credentials are not configured (graceful degradation —
/// existing CDP premium routes continue to work independently).
pub async fn init_okx_server() -> Option<X402ResourceServer> {
    let api_key = std::env::var("OKX_API_KEY").ok()?;
    let secret_key = std::env::var("OKX_SECRET_KEY").ok()?;
    let passphrase = std::env::var("OKX_PASSPHRASE").ok()?;

    if api_key.is_empty() || secret_key.is_empty() || passphrase.is_empty() {
        tracing::warn!(
            "OKX_API_KEY / OKX_SECRET_KEY / OKX_PASSPHRASE not set — OKX A2MCP disabled"
        );
        return None;
    }

    tracing::info!("OKX credentials found, creating facilitator client...");

    let base_url =
        std::env::var("OKX_FACILITATOR_URL").unwrap_or_else(|_| OKX_FACILITATOR_URL.into());

    let facilitator =
        match OkxHttpFacilitatorClient::with_url(&base_url, &api_key, &secret_key, &passphrase) {
            Ok(client) => {
                tracing::info!("OKX facilitator client created (base_url={base_url})");
                client
            }
            Err(e) => {
                tracing::error!("Failed to create OKX facilitator client: {e}");
                return None;
            }
        };

    tracing::info!("Registering X Layer (eip155:196) with ExactEvmScheme...");
    let mut server =
        X402ResourceServer::new(facilitator).register(OKX_NETWORK, ExactEvmScheme::new());

    tracing::info!("Calling OKX server.initialize() (GET /supported)...");
    if let Err(e) = server.initialize().await {
        tracing::error!("OKX x402 server initialize failed: {e}");
        return None;
    }

    tracing::info!("OKX A2MCP payment server initialized on X Layer (eip155:196)");
    Some(server)
}

/// Seller wallet address for OKX A2MCP payments.
/// Falls back to the existing X402_PAY_TO_ADDRESS env if OKX_PAY_TO_ADDRESS
/// is not set, so operators only need one payout address for both facilitators.
fn okx_pay_to() -> String {
    std::env::var("OKX_PAY_TO_ADDRESS")
        .ok()
        .filter(|v| is_valid_evm_address(v))
        .or_else(|| {
            std::env::var("X402_PAY_TO_ADDRESS")
                .ok()
                .filter(|v| is_valid_evm_address(v))
        })
        .unwrap_or_default()
}

fn okx_price() -> String {
    std::env::var("OKX_PREMIUM_PRICE_USD")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_OKX_PRICE.into())
}

/// Public accessor for the OKX price display string (used in REST response bodies).
pub fn okx_price_display() -> String {
    okx_price()
}

/// Canonical public HTTPS URL for a path (never Railway internal host).
///
/// OKX A2MCP listing validators and payment `resource.url` must point at the
/// public origin. When `resource` is unset, x402 middleware derives the URL from
/// `Host` / `X-Forwarded-*`, which on Railway is `*.up.railway.app`.
fn public_resource_url(path: &str) -> String {
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    format!("{}{path}", crate::config::SITE_ORIGIN)
}

/// Canonical A2MCP MCP endpoint for the OKX marketplace package (public HTTPS).
///
/// Hybrid billing: site users connect to free `POST /mcp`. OKX Path A listings
/// and marketplace agents use this paid package path only.
pub fn okx_a2mcp_endpoint() -> String {
    public_resource_url("/mcp/okx")
}

/// Build the routes config for OKX A2MCP premium endpoints.
///
/// Each premium endpoint is mapped to "METHOD /path" with an `exact` payment
/// scheme on X Layer. The middleware intercepts unpaid requests with HTTP 402
/// + PAYMENT-REQUIRED header, verifies via OKX facilitator, and settles on-chain.
pub fn build_okx_routes() -> RoutesConfig {
    let pay_to = okx_pay_to();
    if pay_to.is_empty() {
        tracing::warn!(
            "OKX_PAY_TO_ADDRESS / X402_PAY_TO_ADDRESS not set — OKX routes will be empty"
        );
        return HashMap::new();
    }

    let price = okx_price();
    let mut routes = HashMap::new();

    // Premium REST endpoints — route middleware handles these by exact path match.
    // Package MCP `POST /mcp/okx` JSON-RPC uses handler-level require_okx_payment
    // (public POST /mcp is free discovery — not middleware-gated).
    // Pin `resource` to SITE_ORIGIN so 402 PAYMENT-REQUIRED never leaks Railway hosts.
    let premium_routes = [
        (
            "POST /api/v2/premium/recommend-verified-tool",
            "/api/v2/premium/recommend-verified-tool",
            "AI agent recommends the best verified tool for a given intent",
        ),
        (
            "POST /api/v2/premium/gap-audit",
            "/api/v2/premium/gap-audit",
            "Gap audit: find missing crypto tool categories for a given intent",
        ),
    ];

    for (route_key, path, description) in premium_routes {
        routes.insert(
            route_key.to_string(),
            RoutePaymentConfig {
                accepts: vec![AcceptConfig {
                    scheme: "exact".to_string(),
                    price: price.clone(),
                    network: OKX_NETWORK.to_string(),
                    pay_to: pay_to.clone(),
                    max_timeout_seconds: Some(300),
                    extra: None,
                }],
                description: description.to_string(),
                mime_type: "application/json".to_string(),
                sync_settle: Some(true),
                resource: Some(public_resource_url(path)),
                operation: None,
            },
        );
    }

    routes
}

/// Check if OKX A2MCP is enabled (credentials present).
pub fn is_okx_enabled() -> bool {
    let api_key = std::env::var("OKX_API_KEY").unwrap_or_default();
    let secret_key = std::env::var("OKX_SECRET_KEY").unwrap_or_default();
    let passphrase = std::env::var("OKX_PASSPHRASE").unwrap_or_default();
    !api_key.is_empty() && !secret_key.is_empty() && !passphrase.is_empty()
}

/// MCP tools metered on the OKX marketplace package path only (`POST /mcp/okx`).
/// Public site MCP (`POST /mcp`) keeps discovery free; OKX $0.1 gate never runs there.
/// On `/mcp/okx`, every tools/call in this list is $0.1 USDT0 when OKX env is active.
/// CDP handler gates are skipped for those calls (no double-charge).
pub const OKX_GATED_ROUTES: &[&str] = &[
    "search_tools",
    "get_tool_detail",
    "get_install_guide",
    "list_categories",
    "get_dashboard_snapshot",
    "compare_tools",
    "get_price_history",
    "get_x402_trends",
    "check_endpoint_health",
    "export_toolkit",
    "recommend_verified_tool",
    "gap_audit",
    "save_to_toolkit",
    "save_stack_to_blueprint",
    "link_status",
];

/// Whether `tool_name` is part of the OKX bundled A2MCP package.
pub fn is_okx_package_tool(tool_name: &str) -> bool {
    OKX_GATED_ROUTES.contains(&tool_name)
}

/// Skip handler-level CDP gate when the request is on the OKX package path and
/// OKX is active for this tool (avoids double-charge with OKX USDT0).
pub fn should_skip_cdp_for_okx(
    okx_package_mode: bool,
    okx_premium_gate_active: bool,
    tool_name: &str,
) -> bool {
    okx_package_mode && okx_premium_gate_active && is_okx_package_tool(tool_name)
}

/// Shared OKX facilitator client (initialized once at startup, stored in AppState).
/// When `None`, OKX is not configured and handler-level gate is skipped.
pub type SharedOkxClient = Option<Arc<OkxHttpFacilitatorClient>>;

/// Create a shared OKX facilitator client for handler-level use.
/// Called once at startup when `init_okx_server` succeeds.
pub fn create_okx_facilitator_client() -> Option<Arc<OkxHttpFacilitatorClient>> {
    let api_key = std::env::var("OKX_API_KEY").ok()?;
    let secret_key = std::env::var("OKX_SECRET_KEY").ok()?;
    let passphrase = std::env::var("OKX_PASSPHRASE").ok()?;

    if api_key.is_empty() || secret_key.is_empty() || passphrase.is_empty() {
        return None;
    }

    let base_url =
        std::env::var("OKX_FACILITATOR_URL").unwrap_or_else(|_| OKX_FACILITATOR_URL.into());

    match OkxHttpFacilitatorClient::with_url(&base_url, &api_key, &secret_key, &passphrase) {
        Ok(client) => {
            tracing::info!("OKX handler-level facilitator client created");
            Some(Arc::new(client))
        }
        Err(e) => {
            tracing::error!("Failed to create OKX handler-level facilitator client: {e}");
            None
        }
    }
}

/// Convert a dollar price string like "$0.1" to USDT0 atomic units (6 decimals).
/// "$0.1" → "100000", "$0.001" → "1000".
fn usd_to_usdt_atomic(price: &str) -> Option<String> {
    let cleaned = price.trim_start_matches('$').trim();
    let dollars: f64 = cleaned.parse().ok()?;
    if dollars < 0.0 {
        return None;
    }
    let atomic = (dollars * 1_000_000.0).round() as u128;
    Some(atomic.to_string())
}

/// Build `PaymentRequirements` for an OKX x402 gate on a specific tool.
fn okx_payment_requirements() -> Option<PaymentRequirements> {
    let pay_to = okx_pay_to();
    if pay_to.is_empty() {
        return None;
    }

    let price = okx_price();
    let amount = usd_to_usdt_atomic(&price)?;

    let mut extra = HashMap::new();
    extra.insert("name".to_string(), json!("USD₮0"));
    extra.insert("version".to_string(), json!("1"));
    extra.insert("decimals".to_string(), json!(6));

    Some(PaymentRequirements {
        scheme: "exact".to_string(),
        network: OKX_NETWORK.to_string(),
        asset: XLAYER_USDT0_ADDRESS.to_string(),
        amount,
        pay_to,
        max_timeout_seconds: 300,
        extra,
    })
}

/// Build `ResourceInfo` for a specific tool call.
///
/// `resource_url` must be a public HTTPS URL (not `mcp://` and not Railway).
/// Use [`okx_a2mcp_endpoint`] for the OKX package path; use public `/mcp` when
/// charging premium tools on the free-discovery endpoint via OKX fallback.
fn okx_resource_info(tool_name: &str, description: &str, resource_url: &str) -> ResourceInfo {
    ResourceInfo {
        url: resource_url.to_string(),
        description: Some(format!("OnchainAI MCP {tool_name}: {description}")),
        mime_type: Some("application/json".to_string()),
    }
}

/// Result of a successful OKX payment settlement.
#[derive(Debug, Clone)]
pub struct OkxSettlement {
    pub payer: Option<String>,
    pub transaction: String,
}

/// Handler-level OKX x402 payment gate for MCP `POST /mcp/okx` JSON-RPC tool calls.
///
/// Analogous to `require_axis_b_payment` but uses the OKX Broker facilitator
/// on X Layer (eip155:196) with USDT0 instead of CDP/Base USDC.
///
/// - `Ok(settlement)` — payment verified and settled
/// - `Err(response)` — HTTP 402 (PAYMENT-REQUIRED header + accepts body) or
///   503 when OKX is misconfigured; return verbatim from the handler.

fn okx_missing_payment_response(
    tool_name: &str,
    resource: ResourceInfo,
    requirements: PaymentRequirements,
) -> Response {
    let payment_required = PaymentRequired {
        x402_version: 2,
        error: Some(format!(
            "Payment required for {tool_name} ({}) on X Layer USDT0",
            okx_price()
        )),
        resource,
        accepts: vec![requirements],
        extensions: None,
    };
    okx_402_response(&payment_required)
}

fn okx_invalid_payment_response(
    message: String,
    resource: ResourceInfo,
    requirements: PaymentRequirements,
) -> Response {
    let payment_required = PaymentRequired {
        x402_version: 2,
        error: Some(message),
        resource,
        accepts: vec![requirements],
        extensions: None,
    };
    okx_402_response(&payment_required)
}

pub async fn require_okx_payment(
    client: &OkxHttpFacilitatorClient,
    tool_name: &str,
    tool_description: &str,
    headers: &HeaderMap,
) -> Result<OkxSettlement, Response> {
    require_okx_payment_for_resource(
        client,
        tool_name,
        tool_description,
        headers,
        &okx_a2mcp_endpoint(),
    )
    .await
}

/// Like [`require_okx_payment`] but pins `resource.url` to the actual request surface
/// (public `/mcp` vs package `/mcp/okx`) so 402 payloads match the client path.
pub async fn require_okx_payment_for_resource(
    client: &OkxHttpFacilitatorClient,
    tool_name: &str,
    tool_description: &str,
    headers: &HeaderMap,
    resource_url: &str,
) -> Result<OkxSettlement, Response> {
    let requirements = match okx_payment_requirements() {
        Some(req) => req,
        None => {
            let body = json!({
                "error": "okx_premium_misconfigured",
                "message": "OKX pay-to address or price not configured",
            });
            return Err((StatusCode::SERVICE_UNAVAILABLE, axum::Json(body)).into_response());
        }
    };

    let resource = okx_resource_info(tool_name, tool_description, resource_url);

    // Check for payment-signature header
    let signature_header = headers
        .get("payment-signature")
        .or_else(|| headers.get("PAYMENT-SIGNATURE"))
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let Some(signature) = signature_header else {
        return Err(okx_missing_payment_response(
            tool_name,
            resource,
            requirements,
        ));
    };

    // Decode and verify the payment payload
    let payment_payload = match decode_payment_signature_header(signature) {
        Ok(payload) => payload,
        Err(e) => {
            return Err(okx_invalid_payment_response(
                format!("Invalid payment signature: {e}"),
                resource,
                requirements,
            ));
        }
    };

    let verify_req = VerifyRequest {
        x402_version: 2,
        payment_payload: payment_payload.clone(),
        payment_requirements: requirements.clone(),
    };

    match client.verify(&verify_req).await {
        Ok(verify_resp) => {
            if !verify_resp.is_valid {
                let reason = verify_resp
                    .invalid_reason
                    .as_deref()
                    .unwrap_or("verification failed");
                let msg = verify_resp.invalid_message.as_deref().unwrap_or("");
                return Err(okx_invalid_payment_response(
                    format!("Payment verification failed: {reason}: {msg}"),
                    resource,
                    requirements.clone(),
                ));
            }
        }
        Err(e) => {
            tracing::error!("OKX facilitator verify error: {e}");
            return Err(okx_invalid_payment_response(
                format!("Facilitator verify error: {e}"),
                resource,
                requirements.clone(),
            ));
        }
    }

    // Settle the payment
    let settle_req = SettleRequest {
        x402_version: 2,
        payment_payload,
        payment_requirements: requirements,
        sync_settle: Some(true),
    };

    match client.settle(&settle_req).await {
        Ok(settle_resp) => {
            if !settle_resp.success {
                let reason = settle_resp
                    .error_reason
                    .as_deref()
                    .unwrap_or("settlement failed");
                let body = json!({
                    "error": "okx_settlement_failed",
                    "message": reason,
                });
                return Err((StatusCode::PAYMENT_REQUIRED, axum::Json(body)).into_response());
            }
            tracing::info!(
                "OKX payment settled: tool={tool_name} tx={} payer={:?}",
                settle_resp.transaction,
                settle_resp.payer
            );
            Ok(OkxSettlement {
                payer: settle_resp.payer,
                transaction: settle_resp.transaction,
            })
        }
        Err(e) => {
            tracing::error!("OKX facilitator settle error: {e}");
            let body = json!({
                "error": "okx_facilitator_error",
                "message": format!("Settlement failed: {e}"),
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, axum::Json(body)).into_response())
        }
    }
}

/// Build a 402 Payment Required response with PAYMENT-REQUIRED header.
fn okx_402_response(payment_required: &PaymentRequired) -> Response {
    let body = serde_json::to_value(payment_required).unwrap_or_else(|_| json!({}));
    let body_str = body.to_string();

    let encoded = match encode_payment_required_header(payment_required) {
        Ok(e) => e,
        Err(_) => return (StatusCode::PAYMENT_REQUIRED, body_str).into_response(),
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Ok(hv) = HeaderValue::from_str(&encoded) {
        headers.insert(axum::http::HeaderName::from_static("payment-required"), hv);
    }

    (StatusCode::PAYMENT_REQUIRED, headers, body_str).into_response()
}

/// Build a payment success response with PAYMENT-RESPONSE header.
pub fn okx_payment_success_response(
    body: serde_json::Value,
    settlement: &OkxSettlement,
) -> Response {
    let header_body = json!({
        "success": true,
        "transaction": settlement.transaction,
        "payer": settlement.payer,
    });

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    if let Ok(encoded) = encode_payment_response_header_from_json(&header_body) {
        if let Ok(hv) = HeaderValue::from_str(&encoded) {
            headers.insert(axum::http::HeaderName::from_static("payment-response"), hv);
        }
    }

    (StatusCode::OK, headers, body.to_string()).into_response()
}

/// Encode a JSON value as a base64 payment-response header.
fn encode_payment_response_header_from_json(value: &serde_json::Value) -> Result<String, String> {
    let json = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(json))
}

/// Validate an EVM address: exact 20-byte hex (0x + 40 hex digits), not a placeholder.
fn is_valid_evm_address(addr: &str) -> bool {
    let trimmed = addr.trim();
    trimmed != "0xYourWalletAddress"
        && trimmed.len() == 42
        && trimmed.starts_with("0x")
        && trimmed[2..].bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
#[path = "okx_payment_tests.rs"]
mod tests;
