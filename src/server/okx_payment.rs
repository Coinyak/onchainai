//! OKX Agent Payments Protocol integration — A2MCP seller-side payment gate.
//!
//! Uses the official OKX x402 SDK crates with the OKX facilitator
//! (HMAC-SHA256 auth) on X Layer (eip155:196). The existing CDP facilitator
//! code (`x402_payment.rs`) stays for discovery/free tools; this module gates
//! premium A2MCP endpoints listed on OKX.AI.
//!
//! Spec: OKX Agent Payments Protocol — https://web3.okx.com/onchainos/dev-docs/payments/app

use std::collections::HashMap;

use x402_axum::{AcceptConfig, RoutePaymentConfig, RoutesConfig, X402ResourceServer};
use x402_core::http::OkxHttpFacilitatorClient;
use x402_evm::ExactEvmScheme;

/// X Layer mainnet CAIP-2 identifier.
pub const OKX_NETWORK: &str = "eip155:196";

/// Default price per call for OKX A2MCP premium endpoints.
const DEFAULT_OKX_PRICE: &str = "$0.001";

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

    let base_url =
        std::env::var("OKX_FACILITATOR_URL").unwrap_or_else(|_| OKX_FACILITATOR_URL.into());

    let facilitator =
        match OkxHttpFacilitatorClient::with_url(&base_url, &api_key, &secret_key, &passphrase) {
            Ok(client) => client,
            Err(e) => {
                tracing::error!("Failed to create OKX facilitator client: {e}");
                return None;
            }
        };

    let mut server =
        X402ResourceServer::new(facilitator).register(OKX_NETWORK, ExactEvmScheme::new());

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

    // Premium A2MCP endpoints — each returns 402 without payment, settles via OKX facilitator on X Layer.
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

/// Routes gated by the OKX middleware. When OKX is enabled, handler-level
/// CDP payment gates for these routes must be skipped to avoid double-charging.
pub const OKX_GATED_ROUTES: &[&str] = &["recommend_verified_tool", "gap_audit"];

/// Skip handler-level CDP gate only when OKX middleware is active for this tool.
pub fn should_skip_cdp_for_okx(okx_premium_gate_active: bool, tool_name: &str) -> bool {
    okx_premium_gate_active && OKX_GATED_ROUTES.contains(&tool_name)
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
    fn okx_price_defaults_to_001() {
        let _g = _lock_env();
        std::env::remove_var("OKX_PREMIUM_PRICE_USD");
        assert_eq!(okx_price(), "$0.001");
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
        assert!(should_skip_cdp_for_okx(true, "recommend_verified_tool"));
        assert!(should_skip_cdp_for_okx(true, "gap_audit"));
        assert!(!should_skip_cdp_for_okx(true, "check_endpoint_health"));
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
}
