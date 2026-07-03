//! x402 endpoint liveness and price honesty probes (attribution/trust only — no custody).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::redirect::Policy;
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use tokio::sync::Semaphore;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::server::queries::PUBLIC_TOOL_WHERE;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const FAILURE_DEMOTE_THRESHOLD: i32 = 3;
const DEFAULT_X402_VERIFY_CRON: &str = "0 0 3 * * *";
const MAX_CONCURRENT_PROBES: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeOutcome {
    Verified {
        amount: Option<String>,
        asset: Option<String>,
    },
    NotPaymentRequired,
    SsrfBlocked(String),
    RequestFailed(String),
    ParseFailed,
}

#[derive(Debug, Deserialize)]
struct PaymentRequirements {
    #[serde(default)]
    accepts: Vec<PaymentAccept>,
}

#[derive(Debug, Deserialize)]
struct PaymentAccept {
    #[serde(rename = "maxAmountRequired")]
    max_amount_required: Option<String>,
    #[serde(rename = "maxAmount")]
    max_amount: Option<String>,
    asset: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct X402VerifyStatus {
    pub tool_id: Uuid,
    pub x402_endpoint_verified: bool,
    pub price_verified: bool,
    pub x402_check_failures: i32,
    pub x402_last_checked_at: Option<DateTime<Utc>>,
}

pub fn probe_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(PROBE_TIMEOUT)
        .build()
        .expect("reqwest client")
}

/// Validate probe URL scheme/host before DNS resolution (sync guard).
pub fn validate_probe_url(url_str: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(url_str.trim()).map_err(|e| format!("invalid url: {e}"))?;
    if parsed.scheme() != "https" {
        return Err("only https endpoints are allowed".into());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "url must include a host".to_string())?;
    let host_lower = host.to_lowercase();
    if host_lower == "localhost"
        || host_lower.ends_with(".localhost")
        || host_lower.ends_with(".local")
        || host_lower == "metadata.google.internal"
    {
        return Err("blocked host".into());
    }
    if parsed.username() != "" || parsed.password().is_some() {
        return Err("userinfo in url is not allowed".into());
    }
    Ok(parsed)
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.octets()[0] == 0
                || v4 == Ipv4Addr::new(169, 254, 169, 254)
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xc0) == 64)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || is_unique_local_v6(v6)
                || is_link_local_v6(v6)
        }
    }
}

fn is_unique_local_v6(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

fn is_link_local_v6(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

/// Resolve host and reject private/link-local/metadata addresses (async SSRF guard).
pub async fn resolve_host_is_public(host: &str) -> Result<(), String> {
    let port = 443u16;
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| format!("dns resolution failed: {e}"))?;
    let mut any = false;
    for addr in addrs {
        any = true;
        if is_blocked_ip(addr.ip()) {
            return Err("resolved to blocked address".into());
        }
    }
    if !any {
        return Err("dns resolution returned no addresses".into());
    }
    Ok(())
}

fn truncate_body(body: &str) -> &str {
    if body.len() <= MAX_RESPONSE_BYTES {
        return body;
    }
    let mut end = MAX_RESPONSE_BYTES;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }
    &body[..end]
}

fn parse_payment_requirements(body: &str) -> Option<(Option<String>, Option<String>)> {
    let trimmed = truncate_body(body.trim());
    if trimmed.is_empty() {
        return None;
    }
    let parsed: PaymentRequirements = serde_json::from_str(trimmed).ok()?;
    let first = parsed.accepts.first()?;
    let amount = first
        .max_amount_required
        .clone()
        .or_else(|| first.max_amount.clone());
    Some((amount, first.asset.clone()))
}

/// Normalize advertised x402_price text for comparison with probe amount strings.
pub fn normalize_price_token(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(',', "")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.')
        .collect()
}

pub fn price_matches_advertised(probed_amount: &str, x402_price: &str) -> bool {
    let left = normalize_price_token(probed_amount);
    let right = normalize_price_token(x402_price);
    if left.is_empty() || right.is_empty() {
        return false;
    }
    if left == right {
        return true;
    }
    // Allow advertised labels to embed the probed amount (e.g. "$1,000 USDC"),
    // but reject trivial substring hits such as "1" inside "1000".
    const MIN_PARTIAL_LEN: usize = 3;
    if left.len() >= MIN_PARTIAL_LEN && right.contains(&left) {
        return true;
    }
    if right.len() >= MIN_PARTIAL_LEN && left.contains(&right) {
        return true;
    }
    false
}

/// Pick the first public address for `host` and pin DNS for the outbound probe.
async fn pick_public_probe_addr(host: &str) -> Result<SocketAddr, String> {
    resolve_host_is_public(host).await?;
    let addrs = tokio::net::lookup_host((host, 443u16))
        .await
        .map_err(|e| format!("dns resolution failed: {e}"))?;
    for addr in addrs {
        if !is_blocked_ip(addr.ip()) {
            return Ok(addr);
        }
    }
    Err("dns resolution returned no public addresses".into())
}

fn pinned_probe_client(host: &str, addr: SocketAddr) -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(PROBE_TIMEOUT)
        .resolve(host, addr)
        .build()
        .expect("pinned reqwest client")
}

pub async fn probe_x402_endpoint(_client: &reqwest::Client, url_str: &str) -> ProbeOutcome {
    let parsed = match validate_probe_url(url_str) {
        Ok(url) => url,
        Err(reason) => return ProbeOutcome::SsrfBlocked(reason),
    };
    let host = parsed.host_str().expect("validated host");
    let pinned_addr = match pick_public_probe_addr(host).await {
        Ok(addr) => addr,
        Err(reason) => return ProbeOutcome::SsrfBlocked(reason),
    };
    let pinned_client = pinned_probe_client(host, pinned_addr);

    let response = match pinned_client.post(parsed.clone()).send().await {
        Ok(resp) => resp,
        Err(_) => match pinned_client.get(parsed).send().await {
            Ok(resp) => resp,
            Err(e) => return ProbeOutcome::RequestFailed(e.to_string()),
        },
    };
    let status = response.status();
    let body = match response.text().await {
        Ok(text) => text,
        Err(e) => return ProbeOutcome::RequestFailed(e.to_string()),
    };

    classify_probe_response(status, &body)
}

pub fn classify_probe_response(status: StatusCode, body: &str) -> ProbeOutcome {
    if status != StatusCode::PAYMENT_REQUIRED {
        return ProbeOutcome::NotPaymentRequired;
    }
    match parse_payment_requirements(body) {
        Some((amount, asset)) => ProbeOutcome::Verified { amount, asset },
        None => ProbeOutcome::ParseFailed,
    }
}

fn apply_outcome_to_flags(
    outcome: &ProbeOutcome,
    x402_price: Option<&str>,
    current_endpoint_verified: bool,
    current_failures: i32,
) -> (bool, bool, i32) {
    match outcome {
        ProbeOutcome::Verified { amount, .. } => {
            let endpoint_verified = true;
            let price_verified = amount
                .as_deref()
                .zip(x402_price)
                .is_some_and(|(probed, advertised)| price_matches_advertised(probed, advertised));
            (endpoint_verified, price_verified, 0)
        }
        ProbeOutcome::NotPaymentRequired | ProbeOutcome::ParseFailed | ProbeOutcome::RequestFailed(_) => {
            let failures = current_failures.saturating_add(1);
            let endpoint_verified = if failures >= FAILURE_DEMOTE_THRESHOLD {
                false
            } else {
                current_endpoint_verified
            };
            (endpoint_verified, false, failures)
        }
        ProbeOutcome::SsrfBlocked(_) => (false, false, current_failures.saturating_add(1)),
    }
}

pub async fn verify_tool_by_id(
    pool: &PgPool,
    client: &reqwest::Client,
    tool_id: Uuid,
) -> Result<Option<X402VerifyStatus>, sqlx::Error> {
    let row = sqlx::query_as::<_, ToolProbeRow>(
        r#"
        SELECT x402_endpoint, x402_price, x402_endpoint_verified, x402_check_failures
        FROM tools
        WHERE id = $1
        "#,
    )
    .bind(tool_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };
    let endpoint = match row.x402_endpoint.as_deref() {
        Some(url) if !url.trim().is_empty() => url,
        _ => return Ok(None),
    };

    let outcome = probe_x402_endpoint(client, endpoint).await;
    let (endpoint_verified, price_verified, failures) = apply_outcome_to_flags(
        &outcome,
        row.x402_price.as_deref(),
        row.x402_endpoint_verified,
        row.x402_check_failures,
    );

    let checked_at = Utc::now();
    sqlx::query(
        r#"
        UPDATE tools
        SET x402_endpoint_verified = $2,
            price_verified = $3,
            x402_check_failures = $4,
            x402_last_checked_at = $5,
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(tool_id)
    .bind(endpoint_verified)
    .bind(price_verified)
    .bind(failures)
    .bind(checked_at)
    .execute(pool)
    .await?;

    tracing::info!(
        tool_id = %tool_id,
        ?outcome,
        endpoint_verified,
        price_verified,
        failures,
        "x402 probe completed"
    );

    Ok(Some(X402VerifyStatus {
        tool_id,
        x402_endpoint_verified: endpoint_verified,
        price_verified,
        x402_check_failures: failures,
        x402_last_checked_at: Some(checked_at),
    }))
}

#[derive(Debug, sqlx::FromRow)]
struct ToolProbeRow {
    x402_endpoint: Option<String>,
    x402_price: Option<String>,
    x402_endpoint_verified: bool,
    x402_check_failures: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct ScheduledProbeRow {
    id: Uuid,
}

pub async fn run_scheduled_verification(pool: &PgPool, client: &reqwest::Client) {
    let sql = format!(
        r#"
        SELECT id
        FROM tools
        WHERE pricing = 'x402'
          AND x402_endpoint IS NOT NULL
          AND trim(x402_endpoint) <> ''
          AND {PUBLIC_TOOL_WHERE}
        "#
    );

    let rows = match sqlx::query_as::<_, ScheduledProbeRow>(&sql)
        .fetch_all(pool)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("x402 scheduled verify: failed to list tools: {e}");
            return;
        }
    };

    if rows.is_empty() {
        tracing::info!("x402 scheduled verify: no eligible tools");
        return;
    }

    tracing::info!(count = rows.len(), "x402 scheduled verify: starting batch");
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_PROBES));
    let mut handles = Vec::with_capacity(rows.len());

    for row in rows {
        let pool = pool.clone();
        let client = client.clone();
        let permit = semaphore.clone().acquire_owned().await;
        handles.push(tokio::spawn(async move {
            let _permit = permit;
            if let Err(e) = verify_tool_by_id(&pool, &client, row.id).await {
                tracing::error!(tool_id = %row.id, "x402 scheduled verify failed: {e}");
            }
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }
}

pub fn x402_verify_cron_expr() -> String {
    std::env::var("X402_VERIFY_CRON").unwrap_or_else(|_| DEFAULT_X402_VERIFY_CRON.to_string())
}

pub async fn start_scheduler(pool: PgPool) -> anyhow::Result<()> {
    let cron = x402_verify_cron_expr();
    let scheduler = JobScheduler::new().await?;
    let job_pool = pool.clone();
    let client = probe_client();

    let job = Job::new_async(cron.as_str(), move |_uuid, _l| {
        let pool = job_pool.clone();
        let client = client.clone();
        Box::pin(async move {
            tracing::info!("scheduled job: x402 verification");
            run_scheduled_verification(&pool, &client).await;
        })
    })?;

    scheduler.add(job).await?;
    scheduler.start().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn validate_probe_url_rejects_http_and_localhost() {
        assert!(validate_probe_url("http://example.com/pay").is_err());
        assert!(validate_probe_url("https://localhost/pay").is_err());
        assert!(validate_probe_url("https://127.0.0.1/pay").is_ok());
    }

    #[test]
    fn blocked_ip_covers_private_and_metadata() {
        assert!(is_blocked_ip("127.0.0.1".parse().unwrap()));
        assert!(is_blocked_ip("10.0.0.1".parse().unwrap()));
        assert!(is_blocked_ip("169.254.169.254".parse().unwrap()));
        assert!(!is_blocked_ip("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn price_matches_advertised_normalizes_tokens() {
        assert!(price_matches_advertised("1000", "$1,000 USDC"));
        assert!(price_matches_advertised("0.001", "0.001 usdc"));
        assert!(!price_matches_advertised("2000", "0.001 usdc"));
        assert!(!price_matches_advertised("1", "0.01 usdc"));
        assert!(!price_matches_advertised("1", "1000"));
    }

    #[test]
    fn truncate_body_respects_utf8_boundaries() {
        let emoji_body = "💳".repeat(40_000);
        let truncated = truncate_body(&emoji_body);
        assert!(truncated.len() <= MAX_RESPONSE_BYTES);
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    }

    #[test]
    fn parse_payment_requirements_reads_accepts() {
        let body = r#"{"accepts":[{"scheme":"exact","network":"base","maxAmountRequired":"1000","payTo":"0xabc","asset":"0xusdc"}]}"#;
        let (amount, asset) = parse_payment_requirements(body).expect("parsed");
        assert_eq!(amount.as_deref(), Some("1000"));
        assert_eq!(asset.as_deref(), Some("0xusdc"));
    }

    #[test]
    fn classify_probe_response_accepts_402_with_requirements() {
        let body = r#"{"accepts":[{"maxAmountRequired":"2500","asset":"usdc"}]}"#;
        let outcome = classify_probe_response(StatusCode::PAYMENT_REQUIRED, body);
        assert_eq!(
            outcome,
            ProbeOutcome::Verified {
                amount: Some("2500".into()),
                asset: Some("usdc".into()),
            }
        );
    }

    #[test]
    fn classify_probe_response_rejects_non_402() {
        let outcome = classify_probe_response(StatusCode::OK, "ok");
        assert_eq!(outcome, ProbeOutcome::NotPaymentRequired);
    }

    #[tokio::test]
    async fn wiremock_402_body_shape() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/pay"))
            .respond_with(ResponseTemplate::new(402).set_body_json(serde_json::json!({
                "accepts": [{"maxAmountRequired": "2500", "asset": "usdc"}]
            })))
            .mount(&server)
            .await;

        let client = probe_client();
        let response = client
            .post(format!("{}/pay", server.uri()))
            .send()
            .await
            .expect("request");
        let body = response.text().await.expect("body");
        let outcome = classify_probe_response(StatusCode::PAYMENT_REQUIRED, &body);
        assert_eq!(
            outcome,
            ProbeOutcome::Verified {
                amount: Some("2500".into()),
                asset: Some("usdc".into()),
            }
        );
    }

    #[test]
    fn apply_outcome_demotes_after_three_failures() {
        let (endpoint, price, failures) = apply_outcome_to_flags(
            &ProbeOutcome::NotPaymentRequired,
            Some("1000"),
            true,
            1,
        );
        assert!(endpoint);
        assert!(!price);
        assert_eq!(failures, 2);

        let (endpoint, _, failures) = apply_outcome_to_flags(
            &ProbeOutcome::NotPaymentRequired,
            Some("1000"),
            true,
            2,
        );
        assert!(!endpoint);
        assert_eq!(failures, 3);
    }
}