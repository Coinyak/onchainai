//! Tests extracted from `okx_payment.rs` for Code Health scoring.

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
    // Pin public HTTPS resource URLs (no Railway internal host).
    let gap = routes
        .get("POST /api/v2/premium/gap-audit")
        .expect("gap-audit");
    assert_eq!(
        gap.resource.as_deref(),
        Some("https://www.onchain-ai.xyz/api/v2/premium/gap-audit")
    );
    let rec = routes
        .get("POST /api/v2/premium/recommend-verified-tool")
        .expect("recommend");
    assert_eq!(
        rec.resource.as_deref(),
        Some("https://www.onchain-ai.xyz/api/v2/premium/recommend-verified-tool")
    );
    std::env::remove_var("OKX_PAY_TO_ADDRESS");
}

#[test]
fn okx_resource_info_uses_public_https_mcp_okx_endpoint() {
    let info = okx_resource_info(
        "search_tools",
        "search crypto tools",
        "https://www.onchain-ai.xyz/mcp/okx",
    );
    assert_eq!(info.url, "https://www.onchain-ai.xyz/mcp/okx");
    assert!(info
        .description
        .as_deref()
        .unwrap_or("")
        .contains("search_tools"));
    assert!(!info.url.contains("railway"));
    assert!(!info.url.starts_with("mcp://"));
}

#[test]
fn public_resource_url_and_a2mcp_endpoint() {
    assert_eq!(
        public_resource_url("/api/v2/premium/gap-audit"),
        "https://www.onchain-ai.xyz/api/v2/premium/gap-audit"
    );
    assert_eq!(public_resource_url("mcp"), "https://www.onchain-ai.xyz/mcp");
    // OKX marketplace package path only — public site agents use free /mcp.
    assert_eq!(okx_a2mcp_endpoint(), "https://www.onchain-ai.xyz/mcp/okx");
}

#[test]
fn should_skip_cdp_only_on_okx_package_path_when_active() {
    // Public /mcp (okx_package_mode=false): never skip CDP for OKX.
    assert!(!should_skip_cdp_for_okx(false, true, "search_tools"));
    assert!(!should_skip_cdp_for_okx(
        false,
        true,
        "check_endpoint_health"
    ));
    assert!(!should_skip_cdp_for_okx(false, false, "search_tools"));
    // OKX package path: skip CDP only when OKX gate is active.
    assert!(!should_skip_cdp_for_okx(true, false, "search_tools"));
    assert!(should_skip_cdp_for_okx(true, true, "search_tools"));
    assert!(should_skip_cdp_for_okx(true, true, "compare_tools"));
    assert!(should_skip_cdp_for_okx(true, true, "check_endpoint_health"));
    assert!(should_skip_cdp_for_okx(true, true, "export_toolkit"));
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
fn okx_gated_routes_includes_bundled_package_tools() {
    assert!(OKX_GATED_ROUTES.contains(&"search_tools"));
    assert!(OKX_GATED_ROUTES.contains(&"compare_tools"));
    assert!(OKX_GATED_ROUTES.contains(&"check_endpoint_health"));
    assert!(OKX_GATED_ROUTES.contains(&"export_toolkit"));
    assert!(OKX_GATED_ROUTES.contains(&"recommend_verified_tool"));
    assert!(OKX_GATED_ROUTES.contains(&"gap_audit"));
    // Hybrid: package list is for /mcp/okx only; public /mcp discovery stays free.
    assert!(is_okx_package_tool("search_tools"));
    assert!(is_okx_package_tool("compare_tools"));
    assert!(!is_okx_package_tool("not_a_tool"));
}
