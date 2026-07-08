//! OKX Agent Payments Protocol integration — A2MCP seller-side payment gate.
//!
//! Uses the official OKX x402 SDK crates with the OKX facilitator
//! (HMAC-SHA256 auth) on X Layer (eip155:196). The existing CDP facilitator
//! code (`x402_payment.rs`) stays for discovery/free tools; this module gates
//! premium A2MCP endpoints listed on OKX.AI.
//!
//! Two gate layers:
//! 1. **Route middleware** (`x402_axum::payment_middleware`) — intercepts REST
//!    routes by exact `"METHOD /path"` match. Covers `/api/v2/premium/*` REST.
//! 2. **Handler-level gate** (`require_okx_payment`) — for `/mcp` JSON-RPC,
//!    where the middleware cannot inspect the tool name inside the body.
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
    // MCP `/mcp` JSON-RPC is handled at the handler level (require_okx_payment).
    let premium_routes = [
        (
            "POST /api/v2/premium/recommend-verified-tool",
            "AI agent recommends the best verified tool for a given intent",
        ),
        (
            "POST /api/v2/premium/gap-audit",
            "Gap audit: find missing crypto tool categories for a given intent",
        ),
    ];

    for (route_key, description) in premium_routes {
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
                resource: None,
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

/// All premium tools gated by OKX when active. When OKX is enabled, handler-level
/// CDP payment gates for these tools must be skipped to avoid double-charging.
/// Includes both MCP-dispatched tools and REST check_endpoint_health.
pub const OKX_GATED_ROUTES: &[&str] = &[
    "recommend_verified_tool",
    "gap_audit",
    "export_toolkit",
    "check_endpoint_health",
];

/// Skip handler-level CDP gate only when OKX middleware is active for this tool.
pub fn should_skip_cdp_for_okx(okx_premium_gate_active: bool, tool_name: &str) -> bool {
    okx_premium_gate_active && OKX_GATED_ROUTES.contains(&tool_name)
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
fn okx_resource_info(tool_name: &str, description: &str) -> ResourceInfo {
    ResourceInfo {
        url: format!("mcp://tool/{tool_name}"),
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

/// Handler-level OKX x402 payment gate for MCP `/mcp` JSON-RPC tool calls.
///
/// Analogous to `require_axis_b_payment` but uses the OKX Broker facilitator
/// on X Layer (eip155:196) with USDT0 instead of CDP/Base USDC.
///
/// - `Ok(settlement)` — payment verified and settled
/// - `Err(response)` — HTTP 402 (PAYMENT-REQUIRED header + accepts body) or
///   503 when OKX is misconfigured; return verbatim from the handler.
pub async fn require_okx_payment(
    client: &OkxHttpFacilitatorClient,
    tool_name: &str,
    tool_description: &str,
    headers: &HeaderMap,
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

    let resource = okx_resource_info(tool_name, tool_description);

    // Check for payment-signature header
    let signature_header = headers
        .get("payment-signature")
        .or_else(|| headers.get("PAYMENT-SIGNATURE"))
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let Some(signature) = signature_header else {
        // No payment — return 402 with PAYMENT-REQUIRED header
        let payment_required = PaymentRequired {
            x402_version: 2,
            error: Some(format!(
                "Payment required for {tool_name} ({}) on X Layer USDT0",
                okx_price()
            )),
            resource,
            accepts: vec![requirements.clone()],
            extensions: None,
        };
        return Err(okx_402_response(&payment_required));
    };

    // Decode and verify the payment payload
    let payment_payload = match decode_payment_signature_header(signature) {
        Ok(payload) => payload,
        Err(e) => {
            let payment_required = PaymentRequired {
                x402_version: 2,
                error: Some(format!("Invalid payment signature: {e}")),
                resource,
                accepts: vec![requirements.clone()],
                extensions: None,
            };
            return Err(okx_402_response(&payment_required));
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
                let payment_required = PaymentRequired {
                    x402_version: 2,
                    error: Some(format!("Payment verification failed: {reason}: {msg}")),
                    resource,
                    accepts: vec![requirements.clone()],
                    extensions: None,
                };
                return Err(okx_402_response(&payment_required));
            }
        }
        Err(e) => {
            tracing::error!("OKX facilitator verify error: {e}");
            let payment_required = PaymentRequired {
                x402_version: 2,
                error: Some(format!("Facilitator verify error: {e}")),
                resource,
                accepts: vec![requirements.clone()],
                extensions: None,
            };
            return Err(okx_402_response(&payment_required));
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
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialize env-var-mutating tests to avoid parallel races.
    static ENV_GUARD: Mutex<()> = Mutex::new(());

    fn _lock_env() -> std::sync::MutexGuard<'static, ()> {
        ENV_GUARD.lock().unwrap()
    }

    #[test]
    fn okx_network_is_x_layer() {
        assert_eq!(OKX_NETWORK, "eip155:196");
    }

    #[test]
    fn okx_pay_to_falls_back_to_cdp_env() {
        let _g = _lock_env();
        // When OKX_PAY_TO_ADDRESS is not set, falls back to X402_PAY_TO_ADDRESS.
        std::env::remove_var("OKX_PAY_TO_ADDRESS");
        std::env::set_var(
            "X402_PAY_TO_ADDRESS",
            "0x1234567890abcdef1234567890abcdef12345678",
        );
        assert_eq!(okx_pay_to(), "0x1234567890abcdef1234567890abcdef12345678");
        std::env::remove_var("X402_PAY_TO_ADDRESS");
    }

    #[test]
    fn okx_pay_to_prefers_okx_env() {
        let _g = _lock_env();
        std::env::set_var(
            "OKX_PAY_TO_ADDRESS",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        std::env::set_var(
            "X402_PAY_TO_ADDRESS",
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert_eq!(okx_pay_to(), "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        std::env::remove_var("OKX_PAY_TO_ADDRESS");
        std::env::remove_var("X402_PAY_TO_ADDRESS");
    }

    #[test]
    fn okx_price_defaults_to_0_1() {
        let _g = _lock_env();
        std::env::remove_var("OKX_PREMIUM_PRICE_USD");
        assert_eq!(okx_price(), "$0.1");
    }

    #[test]
    fn is_okx_enabled_false_without_credentials() {
        let _g = _lock_env();
        std::env::remove_var("OKX_API_KEY");
        std::env::remove_var("OKX_SECRET_KEY");
        std::env::remove_var("OKX_PASSPHRASE");
        assert!(!is_okx_enabled());
    }

    #[test]
    fn is_okx_enabled_true_with_all_credentials() {
        let _g = _lock_env();
        std::env::set_var("OKX_API_KEY", "test-key");
        std::env::set_var("OKX_SECRET_KEY", "test-secret");
        std::env::set_var("OKX_PASSPHRASE", "test-pass");
        assert!(is_okx_enabled());
        std::env::remove_var("OKX_API_KEY");
        std::env::remove_var("OKX_SECRET_KEY");
        std::env::remove_var("OKX_PASSPHRASE");
    }

    #[test]
    fn build_okx_routes_empty_without_pay_to() {
        let _g = _lock_env();
        std::env::remove_var("OKX_PAY_TO_ADDRESS");
        std::env::remove_var("X402_PAY_TO_ADDRESS");
        let routes = build_okx_routes();
        assert!(routes.is_empty());
    }

    #[test]
    fn build_okx_routes_has_premium_endpoints() {
        let _g = _lock_env();
        std::env::set_var(
            "OKX_PAY_TO_ADDRESS",
            "0x1234567890abcdef1234567890abcdef12345678",
        );
        let routes = build_okx_routes();
        assert!(routes.contains_key("POST /api/v2/premium/recommend-verified-tool"));
        assert!(routes.contains_key("POST /api/v2/premium/gap-audit"));
        // check-endpoint-health uses {slug} — not OKX-gated (exact-string match can't handle params).
        assert!(!routes.contains_key("GET /api/v2/premium/check-endpoint-health/{slug}"));
        std::env::remove_var("OKX_PAY_TO_ADDRESS");
    }

    #[test]
    fn should_skip_cdp_only_when_middleware_active() {
        assert!(!should_skip_cdp_for_okx(false, "recommend_verified_tool"));
        assert!(!should_skip_cdp_for_okx(false, "gap_audit"));
        assert!(!should_skip_cdp_for_okx(false, "export_toolkit"));
        assert!(!should_skip_cdp_for_okx(false, "check_endpoint_health"));
        assert!(should_skip_cdp_for_okx(true, "recommend_verified_tool"));
        assert!(should_skip_cdp_for_okx(true, "gap_audit"));
        assert!(should_skip_cdp_for_okx(true, "export_toolkit"));
        assert!(should_skip_cdp_for_okx(true, "check_endpoint_health"));
        assert!(!should_skip_cdp_for_okx(true, "search_tools"));
    }

    #[test]
    fn is_valid_evm_address_rejects_invalid() {
        assert!(!is_valid_evm_address("0xYourWalletAddress"));
        assert!(!is_valid_evm_address(""));
        assert!(!is_valid_evm_address("0x123"));
        assert!(!is_valid_evm_address(
            "0x1234567890abcdef1234567890abcdef123456789"
        ));
        assert!(!is_valid_evm_address(
            "0x1234567890abcdef1234567890abcdef1234567g"
        ));
        assert!(is_valid_evm_address(
            "0x1234567890abcdef1234567890abcdef12345678"
        ));
        assert!(is_valid_evm_address(
            "0xAbCdEf1234567890abcdef1234567890abcdef12"
        ));
    }

    #[test]
    fn usd_to_usdt_atomic_converts_correctly() {
        assert_eq!(usd_to_usdt_atomic("$0.1").as_deref(), Some("100000"));
        assert_eq!(usd_to_usdt_atomic("$0.001").as_deref(), Some("1000"));
        assert_eq!(usd_to_usdt_atomic("$1.0").as_deref(), Some("1000000"));
        assert_eq!(usd_to_usdt_atomic("0.1").as_deref(), Some("100000"));
        assert!(usd_to_usdt_atomic("invalid").is_none());
        assert!(usd_to_usdt_atomic("$-1").is_none());
    }

    #[test]
    fn okx_payment_requirements_builds_correctly() {
        let _g = _lock_env();
        std::env::set_var(
            "OKX_PAY_TO_ADDRESS",
            "0x2af05c1661da38a2919dc27b4c8b71cb91c30017",
        );
        std::env::remove_var("OKX_PREMIUM_PRICE_USD");
        let req = okx_payment_requirements();
        assert!(req.is_some());
        let req = req.unwrap();
        assert_eq!(req.network, "eip155:196");
        assert_eq!(req.asset, XLAYER_USDT0_ADDRESS);
        assert_eq!(req.amount, "100000"); // $0.1 → 6-decimal atomic
        assert_eq!(req.scheme, "exact");
        assert_eq!(req.pay_to, "0x2af05c1661da38a2919dc27b4c8b71cb91c30017");
        assert_eq!(req.extra.get("decimals").unwrap(), &json!(6));
        std::env::remove_var("OKX_PAY_TO_ADDRESS");
    }

    #[test]
    fn okx_payment_requirements_none_without_pay_to() {
        let _g = _lock_env();
        std::env::remove_var("OKX_PAY_TO_ADDRESS");
        std::env::remove_var("X402_PAY_TO_ADDRESS");
        assert!(okx_payment_requirements().is_none());
    }

    #[test]
    fn okx_gated_routes_includes_all_premium_tools() {
        assert!(OKX_GATED_ROUTES.contains(&"check_endpoint_health"));
        assert!(OKX_GATED_ROUTES.contains(&"export_toolkit"));
        assert!(OKX_GATED_ROUTES.contains(&"recommend_verified_tool"));
        assert!(OKX_GATED_ROUTES.contains(&"gap_audit"));
        // Free tools must not be in the gated set
        assert!(!OKX_GATED_ROUTES.contains(&"search_tools"));
        assert!(!OKX_GATED_ROUTES.contains(&"compare_tools"));
    }
}
