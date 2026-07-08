//! Axis B MCP premium pricing — HTTP 402 on POST /mcp (no custody, no third-party proxy).
//!
//! Reuses the same facilitator verify/settle gate as K2 (`x402_payment`); the
//! payee, network, and price come from operator-managed `site_settings` instead
//! of env. Default disabled: all Axis B tools stay free until the operator
//! enables MCP premium in admin settings.

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::models::SiteSettings;
use crate::server::x402_payment::{
    default_usdc_asset, facilitator_client, payment_signature_from_headers, require_payment,
    usd_to_usdc_atomic, PaymentSettlement, X402PaymentConfig,
};

/// MCP tools that may require OnchainAI's own x402 payment when premium is enabled.
/// Discovery tools (`search_tools`, `get_tool_detail`, `compare_tools`, etc.) are
/// currently free as a product guideline, not a hard rule — the operator may move
/// any tool into the premium set. Product A (`recommend_verified_tool`), S0
/// (`gap_audit`), and `export_toolkit` are premium (operator-toggled via
/// site_settings). M3 analytics (`get_price_history`, `get_x402_trends`) are
/// discovery/metadata endpoints — currently free.
pub const PREMIUM_MCP_TOOLS: &[&str] = &["export_toolkit", "recommend_verified_tool", "gap_audit"];

pub fn is_premium_mcp_tool(name: &str) -> bool {
    PREMIUM_MCP_TOOLS.contains(&name)
}

/// Dev-only payment bypass — ignored when `SIWX_DOMAIN` is production.
fn dev_accept_bypass_allowed() -> bool {
    std::env::var("SIWX_DOMAIN")
        .ok()
        .is_some_and(|d| crate::auth::session::is_local_dev_domain(&d))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPremiumConfig {
    pub enabled: bool,
    pub pay_to: String,
    pub price: String,
    pub network: String,
    pub asset: Option<String>,
    pub display_price: Option<String>,
}

impl McpPremiumConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
            && !self.pay_to.is_empty()
            && !self.price.is_empty()
            && !self.network.is_empty()
    }
}

pub async fn load_mcp_premium_config(pool: &PgPool) -> Result<McpPremiumConfig, sqlx::Error> {
    let row = sqlx::query_as::<_, SiteSettings>("SELECT * FROM site_settings WHERE id = 1")
        .fetch_one(pool)
        .await?;
    Ok(McpPremiumConfig {
        enabled: row.mcp_premium_enabled,
        pay_to: row.mcp_premium_pay_to_address.unwrap_or_default(),
        price: row.mcp_premium_price.unwrap_or_default(),
        network: row.mcp_premium_network,
        asset: row.mcp_premium_asset,
        display_price: row.mcp_premium_display_price,
    })
}

/// Payment-gate config for Axis B: env supplies facilitator URL + CDP auth;
/// `site_settings` supplies payee, network, price, and (optionally) asset.
fn axis_b_gate_config(config: &McpPremiumConfig) -> Result<X402PaymentConfig, String> {
    let mut gate = X402PaymentConfig::from_env();
    gate.pay_to = config.pay_to.clone();
    gate.network = config.network.clone();
    gate.asset = match config.asset.as_deref().filter(|v| !v.is_empty()) {
        Some(asset) => asset.to_string(),
        None => default_usdc_asset(&config.network),
    };
    gate.amount = usd_to_usdc_atomic(&config.price)
        .ok_or_else(|| format!("invalid MCP premium price '{}'", config.price))?;
    gate.price_display = config
        .display_price
        .clone()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| config.price.clone());
    // Caller has already checked `is_active()`; env `enabled` reflects K2 config,
    // not the operator toggle, so force it on for this gate instance.
    gate.enabled = true;
    Ok(gate)
}

/// Gate an Axis B premium tool call through facilitator verify + settle.
///
/// - `Ok(None)` — dev bypass accepted (local dev only)
/// - `Ok(Some(settlement))` — payment verified and settled
/// - `Err(response)` — HTTP 402 (PAYMENT-REQUIRED header + accepts body) or
///   503 when premium is misconfigured; return verbatim from the handler.
pub async fn require_axis_b_payment(
    config: &McpPremiumConfig,
    tool_name: &str,
    headers: &HeaderMap,
) -> Result<Option<PaymentSettlement>, Response> {
    if dev_accept_bypass_allowed()
        && std::env::var("ONCHAINAI_MCP_X402_DEV_ACCEPT")
            .ok()
            .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"))
        && payment_signature_from_headers(headers).is_some()
    {
        tracing::warn!(
            "ONCHAINAI_MCP_X402_DEV_ACCEPT set — accepting MCP premium without facilitator verify"
        );
        return Ok(None);
    }

    let gate = match axis_b_gate_config(config) {
        Ok(gate) => gate,
        Err(message) => {
            let body = json!({
                "error": "mcp_premium_misconfigured",
                "message": message,
            });
            return Err((StatusCode::SERVICE_UNAVAILABLE, axum::Json(body)).into_response());
        }
    };
    let requirements = gate.requirement_for(
        &format!("mcp://tool/{tool_name}"),
        &format!("OnchainAI MCP {tool_name}"),
        "application/json",
    );
    let client = facilitator_client();
    require_payment(&client, &gate, headers, requirements, None)
        .await
        .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn premium_config() -> McpPremiumConfig {
        McpPremiumConfig {
            enabled: true,
            pay_to: "0x0000000000000000000000000000000000000001".into(),
            price: "$0.01".into(),
            network: "eip155:8453".into(),
            asset: None,
            display_price: Some("$0.01/call".into()),
        }
    }

    #[test]
    fn premium_tool_names_are_stable() {
        assert!(is_premium_mcp_tool("export_toolkit"));
        assert!(is_premium_mcp_tool("recommend_verified_tool"));
        assert!(is_premium_mcp_tool("gap_audit"));
        assert!(!is_premium_mcp_tool("get_price_history"));
        assert!(!is_premium_mcp_tool("get_x402_trends"));
        assert!(!is_premium_mcp_tool("search_tools"));
        assert!(!is_premium_mcp_tool("check_endpoint_health"));
        assert!(!is_premium_mcp_tool("compare_tools"));
    }

    #[test]
    fn inactive_until_payee_and_price_set() {
        let mut config = premium_config();
        assert!(config.is_active());
        config.pay_to.clear();
        assert!(!config.is_active());
    }

    #[test]
    fn axis_b_gate_overrides_env_with_site_settings() {
        let config = premium_config();
        let gate = axis_b_gate_config(&config).expect("gate config");
        assert_eq!(gate.pay_to, config.pay_to);
        assert_eq!(gate.network, "eip155:8453");
        assert_eq!(gate.amount, "10000"); // $0.01 → USDC 6-decimals atomic
        assert_eq!(gate.price_display, "$0.01/call");
        assert!(gate.enabled);
        assert!(!gate.asset.is_empty(), "asset defaults from network");
    }

    #[test]
    fn axis_b_gate_rejects_unparseable_price() {
        let mut config = premium_config();
        config.price = "one cent".into();
        assert!(axis_b_gate_config(&config).is_err());
    }
}
