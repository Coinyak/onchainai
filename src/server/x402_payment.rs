//! x402 payment gate for OnchainAI-owned premium services (K2).
//!
//! Uses a facilitator for verify/settle only — no custody, no third-party proxy.
//! Spec: docs/X402_OPEN_LISTING_SPEC.md §M2/M3, docs/PRODUCT_ENHANCEMENT_SPEC.md §K2.

use std::time::Duration;

use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const HEADER_PAYMENT_SIGNATURE: &str = "PAYMENT-SIGNATURE";
pub const HEADER_PAYMENT_REQUIRED: &str = "PAYMENT-REQUIRED";
pub const HEADER_PAYMENT_RESPONSE: &str = "PAYMENT-RESPONSE";

pub const DEFAULT_FACILITATOR_URL: &str = "https://x402.org/facilitator";
pub const DEFAULT_NETWORK: &str = "eip155:84532";
pub const DEFAULT_PRICE_USD: &str = "$0.001";
pub const DEFAULT_TIMEOUT_SECS: i32 = 300;

/// USDC on Base Sepolia (testnet) and Base mainnet.
const USDC_BASE_SEPOLIA: &str = "0x036CbD53842c542663c028720630235A916019A";
const USDC_BASE_MAINNET: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bDa02913";

#[derive(Debug, Clone)]
pub struct X402PaymentConfig {
    pub enabled: bool,
    pub facilitator_url: String,
    pub pay_to: String,
    pub network: String,
    pub asset: String,
    pub amount: String,
    pub price_display: String,
    pub cdp_api_key_name: Option<String>,
    pub cdp_api_key_private: Option<String>,
}

impl X402PaymentConfig {
    pub fn from_env() -> Self {
        let pay_to = std::env::var("X402_PAY_TO_ADDRESS").unwrap_or_default();
        let enabled = is_configured_pay_to(&pay_to);
        let network = std::env::var("X402_NETWORK").unwrap_or_else(|_| DEFAULT_NETWORK.into());
        let price_display =
            std::env::var("X402_PREMIUM_PRICE_USD").unwrap_or_else(|_| DEFAULT_PRICE_USD.into());
        let amount = std::env::var("X402_PREMIUM_AMOUNT_ATOMIC").unwrap_or_else(|_| {
            usd_to_usdc_atomic(&price_display).unwrap_or_else(|| "1000".into())
        });
        let asset =
            std::env::var("X402_USDC_ASSET").unwrap_or_else(|_| default_usdc_asset(&network));
        Self {
            enabled,
            facilitator_url: std::env::var("X402_FACILITATOR_URL")
                .unwrap_or_else(|_| DEFAULT_FACILITATOR_URL.into()),
            pay_to,
            network,
            asset,
            amount,
            price_display,
            cdp_api_key_name: std::env::var("CDP_API_KEY_NAME").ok(),
            cdp_api_key_private: std::env::var("CDP_API_KEY_PRIVATE_KEY").ok(),
        }
    }

    pub fn requirement_for(
        &self,
        resource_url: &str,
        description: &str,
        mime_type: &str,
    ) -> PaymentRequirementsV2 {
        PaymentRequirementsV2 {
            scheme: "exact".into(),
            network: self.network.clone(),
            asset: self.asset.clone(),
            amount: self.amount.clone(),
            pay_to: self.pay_to.clone(),
            max_timeout_seconds: DEFAULT_TIMEOUT_SECS,
            extra: None,
            resource: Some(ResourceInfo {
                url: resource_url.into(),
                description: Some(description.into()),
                mime_type: Some(mime_type.into()),
                service_name: Some("OnchainAI".into()),
                tags: Some(vec!["premium".into(), "trust-data".into()]),
                icon_url: None,
            }),
        }
    }
}

fn is_configured_pay_to(pay_to: &str) -> bool {
    let trimmed = pay_to.trim();
    !trimmed.is_empty()
        && trimmed != "0xYourWalletAddress"
        && trimmed.starts_with("0x")
        && trimmed.len() >= 42
}

fn default_usdc_asset(network: &str) -> String {
    if network == "eip155:8453" {
        USDC_BASE_MAINNET.into()
    } else {
        USDC_BASE_SEPOLIA.into()
    }
}

/// Convert a dollar string like "$0.001" to USDC atomic units (6 decimals).
pub fn usd_to_usdc_atomic(price: &str) -> Option<String> {
    let trimmed = price.trim().trim_start_matches('$');
    let value: f64 = trimmed.parse().ok()?;
    if value <= 0.0 || !value.is_finite() {
        return None;
    }
    let atomic = (value * 1_000_000.0).round() as u64;
    Some(atomic.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(rename = "serviceName", skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "iconUrl", skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequirementsV2 {
    pub scheme: String,
    pub network: String,
    pub asset: String,
    pub amount: String,
    #[serde(rename = "payTo")]
    pub pay_to: String,
    #[serde(rename = "maxTimeoutSeconds")]
    pub max_timeout_seconds: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<ResourceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequiredV2 {
    #[serde(rename = "x402Version")]
    pub x402_version: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<ResourceInfo>,
    pub accepts: Vec<PaymentRequirementsV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PaymentPayloadV2 {
    #[serde(rename = "x402Version")]
    x402_version: i32,
    payload: Value,
    accepted: PaymentRequirementsV2,
}

#[derive(Debug, Deserialize)]
struct VerifyResponse {
    #[serde(rename = "isValid")]
    is_valid: bool,
    #[serde(rename = "invalidReason", default)]
    invalid_reason: Option<String>,
    #[serde(rename = "invalidMessage", default)]
    invalid_message: Option<String>,
    #[serde(default)]
    payer: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SettleResponse {
    success: bool,
    #[serde(rename = "errorReason", default)]
    error_reason: Option<String>,
    #[serde(rename = "errorMessage", default)]
    error_message: Option<String>,
    #[serde(default)]
    payer: Option<String>,
    #[serde(default)]
    transaction: Option<String>,
}

#[derive(Debug)]
pub enum PaymentGateError {
    Misconfigured,
    InvalidSignature(String),
    Facilitator(String),
}

#[derive(Debug)]
pub struct PaymentSettlement {
    pub payer: Option<String>,
    pub transaction: Option<String>,
}

pub fn payment_signature_from_headers(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(HEADER_PAYMENT_SIGNATURE)
        .or_else(|| headers.get("payment-signature"))
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

pub fn build_payment_required(
    requirements: PaymentRequirementsV2,
    error: Option<&str>,
) -> PaymentRequiredV2 {
    let resource = requirements.resource.clone();
    PaymentRequiredV2 {
        x402_version: 2,
        error: error.map(str::to_string),
        resource,
        accepts: vec![requirements],
    }
}

pub fn encode_payment_header<T: Serialize>(value: &T) -> Result<String, String> {
    let json = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(json))
}

pub fn payment_required_response(
    payment_required: &PaymentRequiredV2,
    body: Value,
) -> Result<Response, String> {
    let encoded = encode_payment_header(payment_required)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(
        HeaderName::from_static(HEADER_PAYMENT_REQUIRED),
        HeaderValue::from_str(&encoded).map_err(|e| e.to_string())?,
    );
    Ok((StatusCode::PAYMENT_REQUIRED, headers, body.to_string()).into_response())
}

pub fn payment_success_response(
    body: Value,
    settlement: &PaymentSettlement,
) -> Result<Response, String> {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Some(tx) = &settlement.transaction {
        let header_body = json!({
            "success": true,
            "transaction": tx,
            "payer": settlement.payer,
        });
        let encoded = encode_payment_header(&header_body)?;
        headers.insert(
            HeaderName::from_static(HEADER_PAYMENT_RESPONSE),
            HeaderValue::from_str(&encoded).map_err(|e| e.to_string())?,
        );
    }
    Ok((StatusCode::OK, headers, body.to_string()).into_response())
}

pub fn requirements_match(
    accepted: &PaymentRequirementsV2,
    expected: &PaymentRequirementsV2,
) -> bool {
    accepted.scheme == expected.scheme
        && accepted.network == expected.network
        && accepted.asset.eq_ignore_ascii_case(&expected.asset)
        && accepted.amount == expected.amount
        && accepted.pay_to.eq_ignore_ascii_case(&expected.pay_to)
}

fn decode_payment_payload(header: &str) -> Result<PaymentPayloadV2, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(header.trim())
        .map_err(|e| format!("invalid base64 payment signature: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("invalid payment payload json: {e}"))
}

pub fn facilitator_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("facilitator reqwest client")
}

async fn facilitator_post(
    client: &Client,
    config: &X402PaymentConfig,
    path: &str,
    body: Value,
) -> Result<Value, String> {
    let url = format!(
        "{}/{}",
        config.facilitator_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    );
    let mut req = client.post(&url).json(&body);
    if let (Some(name), Some(private)) = (&config.cdp_api_key_name, &config.cdp_api_key_private) {
        req = req.basic_auth(name, Some(private));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("facilitator {path} request failed: {e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("facilitator {path} read failed: {e}"))?;
    if !status.is_success() {
        return Err(format!("facilitator {path} returned {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("facilitator {path} invalid json: {e}"))
}

pub async fn verify_and_settle(
    client: &Client,
    config: &X402PaymentConfig,
    payment_signature: &str,
    expected: &PaymentRequirementsV2,
) -> Result<PaymentSettlement, PaymentGateError> {
    if !config.enabled {
        return Err(PaymentGateError::Misconfigured);
    }

    let payload =
        decode_payment_payload(payment_signature).map_err(PaymentGateError::InvalidSignature)?;

    if payload.x402_version != 2 {
        return Err(PaymentGateError::InvalidSignature(format!(
            "unsupported x402 version {}",
            payload.x402_version
        )));
    }

    if !requirements_match(&payload.accepted, expected) {
        return Err(PaymentGateError::InvalidSignature(
            "payment does not match declared requirements".into(),
        ));
    }

    let requirements_value = serde_json::to_value(&payload.accepted)
        .map_err(|e| PaymentGateError::InvalidSignature(e.to_string()))?;
    let payload_value = serde_json::to_value(&payload)
        .map_err(|e| PaymentGateError::InvalidSignature(e.to_string()))?;

    let verify_body = json!({
        "x402Version": 2,
        "paymentPayload": payload_value,
        "paymentRequirements": requirements_value,
    });

    let verify_raw = facilitator_post(client, config, "verify", verify_body)
        .await
        .map_err(PaymentGateError::Facilitator)?;

    let verify: VerifyResponse = serde_json::from_value(verify_raw)
        .map_err(|e| PaymentGateError::Facilitator(format!("verify parse: {e}")))?;

    if !verify.is_valid {
        let reason = verify
            .invalid_message
            .or(verify.invalid_reason)
            .unwrap_or_else(|| "payment verification failed".into());
        return Err(PaymentGateError::Facilitator(reason));
    }

    let settle_body = json!({
        "x402Version": 2,
        "paymentPayload": serde_json::to_value(&payload).map_err(|e| PaymentGateError::Facilitator(e.to_string()))?,
        "paymentRequirements": requirements_value,
    });

    let settle_raw = facilitator_post(client, config, "settle", settle_body)
        .await
        .map_err(PaymentGateError::Facilitator)?;

    let settle: SettleResponse = serde_json::from_value(settle_raw)
        .map_err(|e| PaymentGateError::Facilitator(format!("settle parse: {e}")))?;

    if !settle.success {
        let reason = settle
            .error_message
            .or(settle.error_reason)
            .unwrap_or_else(|| "settlement failed".into());
        return Err(PaymentGateError::Facilitator(reason));
    }

    Ok(PaymentSettlement {
        payer: settle.payer.or(verify.payer),
        transaction: settle.transaction,
    })
}

pub async fn require_payment(
    client: &Client,
    config: &X402PaymentConfig,
    headers: &HeaderMap,
    requirements: PaymentRequirementsV2,
) -> Result<PaymentSettlement, Response> {
    if !config.enabled {
        let body = json!({
            "error": "premium_x402_not_configured",
            "message": "OnchainAI premium x402 is not configured (set X402_PAY_TO_ADDRESS)",
        });
        return Err((StatusCode::SERVICE_UNAVAILABLE, axum::Json(body)).into_response());
    }

    let Some(signature) = payment_signature_from_headers(headers) else {
        let payment_required = build_payment_required(requirements, Some("Payment required"));
        let body = serde_json::to_value(&payment_required).unwrap_or_else(|_| json!({}));
        return Err(payment_required_response(&payment_required, body)
            .unwrap_or_else(|_| StatusCode::PAYMENT_REQUIRED.into_response()));
    };

    match verify_and_settle(client, config, signature, &requirements).await {
        Ok(settlement) => Ok(settlement),
        Err(PaymentGateError::InvalidSignature(msg)) | Err(PaymentGateError::Facilitator(msg)) => {
            let payment_required = build_payment_required(requirements, Some(&msg));
            let body = serde_json::to_value(&payment_required).unwrap_or_else(|_| json!({}));
            Err(payment_required_response(&payment_required, body)
                .unwrap_or_else(|_| StatusCode::PAYMENT_REQUIRED.into_response()))
        }
        Err(PaymentGateError::Misconfigured) => {
            let body = json!({
                "error": "premium_x402_not_configured",
                "message": "OnchainAI premium x402 is not configured",
            });
            Err((StatusCode::SERVICE_UNAVAILABLE, axum::Json(body)).into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usd_to_atomic_converts_micropayment() {
        assert_eq!(usd_to_usdc_atomic("$0.001").as_deref(), Some("1000"));
        assert_eq!(usd_to_usdc_atomic("$0.01").as_deref(), Some("10000"));
        assert_eq!(usd_to_usdc_atomic("0.001").as_deref(), Some("1000"));
    }

    #[test]
    fn requirements_match_compares_case_insensitive_addresses() {
        let expected = PaymentRequirementsV2 {
            scheme: "exact".into(),
            network: "eip155:84532".into(),
            asset: USDC_BASE_SEPOLIA.into(),
            amount: "1000".into(),
            pay_to: "0xAbCdEf0000000000000000000000000000000001".into(),
            max_timeout_seconds: 300,
            extra: None,
            resource: None,
        };
        let mut accepted = expected.clone();
        accepted.pay_to = "0xabcdef0000000000000000000000000000000001".into();
        accepted.asset = accepted.asset.to_uppercase();
        assert!(requirements_match(&accepted, &expected));
    }

    #[test]
    fn build_payment_required_wraps_single_accept() {
        let req = PaymentRequirementsV2 {
            scheme: "exact".into(),
            network: DEFAULT_NETWORK.into(),
            asset: USDC_BASE_SEPOLIA.into(),
            amount: "1000".into(),
            pay_to: "0x0000000000000000000000000000000000000001".into(),
            max_timeout_seconds: 300,
            extra: None,
            resource: Some(ResourceInfo {
                url: "https://example.com/premium".into(),
                description: Some("test".into()),
                mime_type: Some("application/json".into()),
                service_name: None,
                tags: None,
                icon_url: None,
            }),
        };
        let required = build_payment_required(req.clone(), Some("Payment required"));
        assert_eq!(required.x402_version, 2);
        assert_eq!(required.accepts.len(), 1);
        assert_eq!(required.accepts[0].amount, "1000");
    }

    #[test]
    fn is_configured_rejects_placeholder_wallet() {
        assert!(!is_configured_pay_to("0xYourWalletAddress"));
        assert!(!is_configured_pay_to(""));
        assert!(is_configured_pay_to(
            "0x0000000000000000000000000000000000000001"
        ));
    }
}
