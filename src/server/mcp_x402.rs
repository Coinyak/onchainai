//! Axis B MCP premium pricing — HTTP 402 on POST /mcp (no custody, no third-party proxy).

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::models::SiteSettings;

pub const PAYMENT_REQUIRED_HEADER: &str = "PAYMENT-REQUIRED";
pub const PAYMENT_SIGNATURE_HEADER: &str = "PAYMENT-SIGNATURE";

/// MCP tools that may require OnchainAI's own x402 payment when premium is enabled.
pub const PREMIUM_MCP_TOOLS: &[&str] = &["compare_tools", "export_toolkit"];

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

pub fn build_payment_required_header(config: &McpPremiumConfig, tool_name: &str) -> String {
    let mut accept = json!({
        "scheme": "exact",
        "network": config.network,
        "price": config.price,
        "payTo": config.pay_to,
    });
    if let Some(asset) = config.asset.as_deref().filter(|v| !v.is_empty()) {
        accept["asset"] = json!(asset);
    }
    let body = json!({
        "x402Version": 2,
        "accepts": [accept],
        "resource": {
            "url": format!("mcp://tool/{tool_name}"),
            "description": format!("OnchainAI MCP {tool_name}"),
            "mimeType": "application/json",
        },
    });
    STANDARD.encode(body.to_string())
}

#[derive(Debug, Deserialize)]
struct PaymentPayloadEnvelope {
    #[serde(default)]
    x402_version: Option<u8>,
    #[serde(rename = "x402Version", default)]
    x402_version_camel: Option<u8>,
}

/// Structural validation only — settlement verification uses an external facilitator when configured.
pub fn payment_signature_structurally_valid(header: Option<&str>) -> bool {
    let Some(raw) = header.map(str::trim).filter(|v| !v.is_empty()) else {
        return false;
    };
    let decoded = match STANDARD.decode(raw) {
        Ok(bytes) => bytes,
        Err(_) => return payment_signature_is_json(raw),
    };
    let text = match std::str::from_utf8(&decoded) {
        Ok(text) => text,
        Err(_) => return false,
    };
    payment_signature_is_json(text)
}

fn payment_signature_is_json(text: &str) -> bool {
    let parsed: PaymentPayloadEnvelope = match serde_json::from_str(text) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };
    parsed.x402_version == Some(2) || parsed.x402_version_camel == Some(2)
}

pub async fn payment_granted(
    config: &McpPremiumConfig,
    payment_signature: Option<&str>,
) -> Result<bool, String> {
    if !config.is_active() {
        return Ok(true);
    }
    if !payment_signature_structurally_valid(payment_signature) {
        return Ok(false);
    }
    if dev_accept_bypass_allowed()
        && std::env::var("ONCHAINAI_MCP_X402_DEV_ACCEPT")
            .ok()
            .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"))
    {
        tracing::warn!(
            "ONCHAINAI_MCP_X402_DEV_ACCEPT set — accepting MCP premium without facilitator verify"
        );
        return Ok(true);
    }
    let facilitator = std::env::var("X402_FACILITATOR_URL")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let Some(facilitator_url) = facilitator else {
        tracing::warn!(
            "X402_FACILITATOR_URL unset — MCP premium payment signature present but not verified"
        );
        return Ok(false);
    };
    verify_with_facilitator(
        &facilitator_url,
        payment_signature.unwrap_or_default(),
        config,
    )
    .await
}

async fn verify_with_facilitator(
    facilitator_url: &str,
    payment_signature: &str,
    config: &McpPremiumConfig,
) -> Result<bool, String> {
    let base = facilitator_url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("facilitator client: {e}"))?;
    let response = client
        .post(format!("{base}/verify"))
        .header(PAYMENT_SIGNATURE_HEADER, payment_signature)
        .json(&json!({
            "network": config.network,
            "payTo": config.pay_to,
            "price": config.price,
        }))
        .send()
        .await
        .map_err(|e| format!("facilitator verify failed: {e}"))?;
    Ok(response.status().is_success())
}

pub fn premium_tool_notice(config: &McpPremiumConfig, tool_name: &str) -> Option<String> {
    if !config.is_active() || !is_premium_mcp_tool(tool_name) {
        return None;
    }
    let price = config
        .display_price
        .as_deref()
        .filter(|v| !v.is_empty())
        .unwrap_or(config.price.as_str());
    Some(format!(
        "x402 premium tool ({price} per call). Retry POST /mcp with PAYMENT-SIGNATURE after wallet payment. Free discovery tools: search_tools, get_tool_detail, get_install_guide."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premium_tool_names_are_stable() {
        assert!(is_premium_mcp_tool("compare_tools"));
        assert!(is_premium_mcp_tool("export_toolkit"));
        assert!(!is_premium_mcp_tool("search_tools"));
    }

    #[test]
    fn payment_required_header_is_base64_json() {
        let config = McpPremiumConfig {
            enabled: true,
            pay_to: "0x0000000000000000000000000000000000000001".into(),
            price: "$0.01".into(),
            network: "eip155:8453".into(),
            asset: None,
            display_price: Some("$0.01/call".into()),
        };
        let encoded = build_payment_required_header(&config, "compare_tools");
        let decoded = STANDARD.decode(encoded).expect("base64");
        let json: serde_json::Value = serde_json::from_slice(&decoded).expect("json");
        assert_eq!(json["x402Version"], 2);
        assert_eq!(json["accepts"][0]["payTo"], config.pay_to);
        assert!(json["resource"]["url"]
            .as_str()
            .unwrap()
            .contains("compare_tools"));
    }

    #[test]
    fn payment_signature_requires_v2_marker() {
        let payload = r#"{"x402Version":2,"payload":{}}"#;
        let encoded = STANDARD.encode(payload);
        assert!(payment_signature_structurally_valid(Some(&encoded)));
        assert!(!payment_signature_structurally_valid(Some(r#"{"v":1}"#)));
    }
}
