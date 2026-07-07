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
const L4_CONSECUTIVE_FAILURE_DAYS: i64 = 14;
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
    network: Option<String>,
    #[serde(rename = "payTo")]
    pay_to: Option<String>,
    description: Option<String>,
}

/// Full first-accept payment details for the self-listing probe preview.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct X402ProbeDetails {
    pub amount: Option<String>,
    pub asset: Option<String>,
    pub network: Option<String>,
    pub pay_to: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeDetailsOutcome {
    Live(X402ProbeDetails),
    NotPaymentRequired {
        status: u16,
        body_snippet: Option<String>,
    },
    SsrfBlocked(String),
    RequestFailed(String),
    ParseFailed,
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
        .unwrap_or_else(|_| reqwest::Client::new())
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
    if host.parse::<IpAddr>().is_ok() {
        return Err("ip literal hosts are not allowed".into());
    }
    match parsed.port() {
        None | Some(443) => {}
        Some(port) => return Err(format!("only port 443 is allowed (got {port})")),
    }
    Ok(parsed)
}

async fn read_limited_response_body(mut response: reqwest::Response) -> Result<String, String> {
    let mut body = Vec::new();
    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(e) => return Err(e.to_string()),
        };
        if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(format!("response body exceeds {MAX_RESPONSE_BYTES} bytes"));
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|e| e.to_string())
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

/// Resolve host once, reject blocked IPs, return the first public socket for pinning.
pub async fn resolve_public_probe_addr(host: &str) -> Result<SocketAddr, String> {
    let port = 443u16;
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| format!("dns resolution failed: {e}"))?;
    let mut any = false;
    let mut first_public = None;
    for addr in addrs {
        any = true;
        if is_blocked_ip(addr.ip()) {
            return Err("resolved to blocked address".into());
        }
        if first_public.is_none() {
            first_public = Some(addr);
        }
    }
    if !any {
        return Err("dns resolution returned no addresses".into());
    }
    first_public.ok_or_else(|| "dns resolution returned no public addresses".into())
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

const PROBE_BODY_SNIPPET_MAX: usize = 200;

fn probe_body_snippet(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.len() <= PROBE_BODY_SNIPPET_MAX {
        return Some(trimmed.to_string());
    }
    let mut end = PROBE_BODY_SNIPPET_MAX;
    while end > 0 && !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    Some(format!("{}…", &trimmed[..end]))
}

fn parse_first_payment_accept(body: &str) -> Option<X402ProbeDetails> {
    let trimmed = truncate_body(body.trim());
    if trimmed.is_empty() {
        return None;
    }
    let mut parsed: PaymentRequirements = serde_json::from_str(trimmed).ok()?;
    if parsed.accepts.is_empty() {
        return None;
    }
    let first = parsed.accepts.swap_remove(0);
    Some(X402ProbeDetails {
        amount: first.max_amount_required.or(first.max_amount),
        asset: first.asset,
        network: first.network,
        pay_to: first.pay_to,
        description: first.description,
    })
}

fn parse_payment_requirements(body: &str) -> Option<(Option<String>, Option<String>)> {
    parse_first_payment_accept(body).map(|d| (d.amount, d.asset))
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

/// Leading numeric token (digits + optional decimal point) from normalized price text.
fn extract_amount_digits(value: &str) -> String {
    normalize_price_token(value)
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect()
}

pub fn price_matches_advertised(probed_amount: &str, x402_price: &str) -> bool {
    let probed = extract_amount_digits(probed_amount);
    let advertised = extract_amount_digits(x402_price);
    !probed.is_empty() && probed == advertised
}

/// Map a scheduled probe outcome to `x402_probe_history.status`.
pub fn probe_history_status(
    outcome: &ProbeOutcome,
    x402_price: Option<&str>,
) -> (&'static str, Option<String>) {
    match outcome {
        ProbeOutcome::Verified { amount, .. } => {
            let price_match = amount
                .as_deref()
                .zip(x402_price)
                .is_some_and(|(probed, advertised)| price_matches_advertised(probed, advertised));
            if price_match {
                ("live", amount.clone())
            } else {
                ("price_mismatch", amount.clone())
            }
        }
        ProbeOutcome::NotPaymentRequired | ProbeOutcome::RequestFailed(_) => ("dead", None),
        ProbeOutcome::ParseFailed | ProbeOutcome::SsrfBlocked(_) => ("invalid", None),
    }
}

pub async fn record_probe_history(
    pool: &PgPool,
    tool_id: Uuid,
    endpoint_url: &str,
    status: &str,
    actual_price: Option<&str>,
    latency_ms: i32,
) {
    let result = sqlx::query(
        r#"
        INSERT INTO x402_probe_history (tool_id, endpoint_url, status, actual_price, latency_ms)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(tool_id)
    .bind(endpoint_url)
    .bind(status)
    .bind(actual_price)
    .bind(latency_ms)
    .execute(pool)
    .await;
    if let Err(e) = result {
        tracing::warn!(tool_id = %tool_id, "x402 probe history insert failed: {e}");
    }
}

/// True when `today` and the prior `L4_CONSECUTIVE_FAILURE_DAYS - 1` UTC days
/// each have a non-live probe (no gaps).
pub fn l4_consecutive_failure_days_met(
    daily_rows: &[(chrono::NaiveDate, String)],
    today: chrono::NaiveDate,
) -> bool {
    if daily_rows.len() < L4_CONSECUTIVE_FAILURE_DAYS as usize {
        return false;
    }
    for offset in 0..L4_CONSECUTIVE_FAILURE_DAYS {
        let expected_day = today - chrono::Duration::days(offset);
        let (day, status) = &daily_rows[offset as usize];
        if *day != expected_day || status == "live" {
            return false;
        }
    }
    true
}

pub async fn maybe_auto_quarantine_l4(pool: &PgPool, tool_id: Uuid) -> Result<bool, sqlx::Error> {
    let rows = sqlx::query_as::<_, DailyProbeStatusRow>(
        r#"
        SELECT day, status
        FROM (
            SELECT DISTINCT ON (date_trunc('day', probed_at AT TIME ZONE 'UTC'))
                (date_trunc('day', probed_at AT TIME ZONE 'UTC'))::date AS day,
                status,
                probed_at
            FROM x402_probe_history
            WHERE tool_id = $1
              AND probed_at >= (now() AT TIME ZONE 'UTC' - ($2::bigint * interval '1 day'))
            ORDER BY date_trunc('day', probed_at AT TIME ZONE 'UTC') DESC, probed_at DESC
        ) daily
        ORDER BY day DESC
        LIMIT $2
        "#,
    )
    .bind(tool_id)
    .bind(L4_CONSECUTIVE_FAILURE_DAYS)
    .fetch_all(pool)
    .await?;

    let today = chrono::Utc::now().date_naive();
    let daily_rows: Vec<(chrono::NaiveDate, String)> =
        rows.into_iter().map(|r| (r.day, r.status)).collect();
    if !l4_consecutive_failure_days_met(&daily_rows, today) {
        return Ok(false);
    }

    let result = sqlx::query(
        r#"
        UPDATE tools
        SET quarantined_at = now(),
            updated_at = now()
        WHERE id = $1
          AND quarantined_at IS NULL
        "#,
    )
    .bind(tool_id)
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        tracing::warn!(
            tool_id = %tool_id,
            days = L4_CONSECUTIVE_FAILURE_DAYS,
            "L4 auto-quarantine: consecutive probe failures"
        );
        return Ok(true);
    }
    Ok(false)
}

fn pinned_probe_client(host: &str, addr: SocketAddr) -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(PROBE_TIMEOUT)
        .resolve(host, addr)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

enum ProbeFetchError {
    SsrfBlocked(String),
    RequestFailed(String),
}

/// Shared SSRF-guarded fetch: validate URL, pin DNS, POST-then-GET, bounded read.
async fn fetch_probe_response(url_str: &str) -> Result<(StatusCode, String), ProbeFetchError> {
    let parsed = validate_probe_url(url_str).map_err(ProbeFetchError::SsrfBlocked)?;
    let host = match parsed.host_str() {
        Some(host) => host,
        None => {
            return Err(ProbeFetchError::SsrfBlocked(
                "url must include a host".into(),
            ))
        }
    };
    let pinned_addr = resolve_public_probe_addr(host)
        .await
        .map_err(ProbeFetchError::SsrfBlocked)?;
    let pinned_client = pinned_probe_client(host, pinned_addr);

    let response = match pinned_client.post(parsed.clone()).send().await {
        Ok(resp) => resp,
        Err(_) => pinned_client
            .get(parsed)
            .send()
            .await
            .map_err(|e| ProbeFetchError::RequestFailed(e.to_string()))?,
    };
    let status = response.status();
    let body = read_limited_response_body(response)
        .await
        .map_err(ProbeFetchError::RequestFailed)?;
    Ok((status, body))
}

/// Probe an x402 endpoint. Builds a per-host DNS-pinned client (shared pools are not reused).
pub async fn probe_x402_endpoint(url_str: &str) -> ProbeOutcome {
    match fetch_probe_response(url_str).await {
        Ok((status, body)) => classify_probe_response(status, &body),
        Err(ProbeFetchError::SsrfBlocked(reason)) => ProbeOutcome::SsrfBlocked(reason),
        Err(ProbeFetchError::RequestFailed(reason)) => ProbeOutcome::RequestFailed(reason),
    }
}

/// Probe an x402 endpoint and return full payment details (self-listing preview/publish).
pub async fn probe_x402_details(url_str: &str) -> ProbeDetailsOutcome {
    match fetch_probe_response(url_str).await {
        Ok((status, body)) => classify_probe_details(status, &body),
        Err(ProbeFetchError::SsrfBlocked(reason)) => ProbeDetailsOutcome::SsrfBlocked(reason),
        Err(ProbeFetchError::RequestFailed(reason)) => ProbeDetailsOutcome::RequestFailed(reason),
    }
}

pub fn classify_probe_details(status: StatusCode, body: &str) -> ProbeDetailsOutcome {
    if status != StatusCode::PAYMENT_REQUIRED {
        return ProbeDetailsOutcome::NotPaymentRequired {
            status: status.as_u16(),
            body_snippet: probe_body_snippet(body),
        };
    }
    match parse_first_payment_accept(body) {
        Some(details) => ProbeDetailsOutcome::Live(details),
        None => ProbeDetailsOutcome::ParseFailed,
    }
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
        ProbeOutcome::NotPaymentRequired
        | ProbeOutcome::ParseFailed
        | ProbeOutcome::RequestFailed(_) => {
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

#[derive(Debug, Clone)]
pub struct ToolProbeRun {
    pub outcome: ProbeOutcome,
    pub history_status: String,
    pub actual_price: Option<String>,
    pub probed_at: DateTime<Utc>,
    pub latency_ms: i32,
    pub endpoint_verified: bool,
    pub price_verified: bool,
    pub failures: i32,
}

async fn probe_tool_and_record_history(
    pool: &PgPool,
    tool_id: Uuid,
    endpoint: &str,
    x402_price: Option<&str>,
) -> Result<(ProbeOutcome, String, Option<String>, DateTime<Utc>, i32), sqlx::Error> {
    let started = std::time::Instant::now();
    let outcome = probe_x402_endpoint(endpoint).await;
    let latency_ms = started.elapsed().as_millis().min(i32::MAX as u128) as i32;
    let (history_status, actual_price) = probe_history_status(&outcome, x402_price);
    record_probe_history(
        pool,
        tool_id,
        endpoint,
        history_status,
        actual_price.as_deref(),
        latency_ms,
    )
    .await;
    Ok((
        outcome,
        history_status.to_string(),
        actual_price,
        Utc::now(),
        latency_ms,
    ))
}

/// K2 paid probe: on-demand outcome only — no catalog flags, probe history, or L4 side effects.
pub async fn run_k2_on_demand_probe(
    _pool: &PgPool,
    _tool_id: Uuid,
    endpoint: &str,
    x402_price: Option<&str>,
) -> Result<ToolProbeRun, sqlx::Error> {
    let started = std::time::Instant::now();
    let outcome = probe_x402_endpoint(endpoint).await;
    let latency_ms = started.elapsed().as_millis().min(i32::MAX as u128) as i32;
    let (history_status, actual_price) = probe_history_status(&outcome, x402_price);
    let (endpoint_verified, price_verified, failures) =
        apply_outcome_to_flags(&outcome, x402_price, false, 0);
    Ok(ToolProbeRun {
        outcome,
        history_status: history_status.to_string(),
        actual_price,
        probed_at: Utc::now(),
        latency_ms,
        endpoint_verified,
        price_verified,
        failures,
    })
}

pub async fn run_on_demand_tool_probe(
    pool: &PgPool,
    tool_id: Uuid,
    endpoint: &str,
    x402_price: Option<&str>,
    current_endpoint_verified: bool,
    current_failures: i32,
) -> Result<ToolProbeRun, sqlx::Error> {
    let (outcome, history_status, actual_price, probed_at, latency_ms) =
        probe_tool_and_record_history(pool, tool_id, endpoint, x402_price).await?;

    let (endpoint_verified, price_verified, failures) = apply_outcome_to_flags(
        &outcome,
        x402_price,
        current_endpoint_verified,
        current_failures,
    );

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
    .bind(probed_at)
    .execute(pool)
    .await?;

    if let Err(e) = maybe_auto_quarantine_l4(pool, tool_id).await {
        tracing::error!(tool_id = %tool_id, "L4 auto-quarantine check failed: {e}");
    }

    Ok(ToolProbeRun {
        outcome,
        history_status,
        actual_price,
        probed_at,
        latency_ms,
        endpoint_verified,
        price_verified,
        failures,
    })
}

pub async fn verify_tool_by_id(
    pool: &PgPool,
    _client: &reqwest::Client,
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

    let run = run_on_demand_tool_probe(
        pool,
        tool_id,
        endpoint,
        row.x402_price.as_deref(),
        row.x402_endpoint_verified,
        row.x402_check_failures,
    )
    .await?;

    tracing::info!(
        tool_id = %tool_id,
        outcome = ?run.outcome,
        endpoint_verified = run.endpoint_verified,
        price_verified = run.price_verified,
        failures = run.failures,
        history_status = %run.history_status,
        "x402 probe completed"
    );

    Ok(Some(X402VerifyStatus {
        tool_id,
        x402_endpoint_verified: run.endpoint_verified,
        price_verified: run.price_verified,
        x402_check_failures: run.failures,
        x402_last_checked_at: Some(run.probed_at),
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

#[derive(Debug, sqlx::FromRow)]
struct DailyProbeStatusRow {
    day: chrono::NaiveDate,
    status: String,
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
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(e) => {
                tracing::error!("x402 scheduled verify: semaphore closed: {e}");
                break;
            }
        };
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

    if std::env::var("X402_VERIFY_RUN_ON_START").is_ok() {
        let boot_pool = pool.clone();
        let boot_client = client.clone();
        tokio::spawn(async move {
            tracing::info!("x402 verification run-on-start");
            run_scheduled_verification(&boot_pool, &boot_client).await;
        });
    }

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
        assert!(validate_probe_url("https://127.0.0.1/pay").is_err());
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
        assert!(!price_matches_advertised("100", "$1000 USDC"));
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

    #[test]
    fn classify_probe_details_carries_http_status_and_body_snippet() {
        let outcome = classify_probe_details(StatusCode::NOT_FOUND, "endpoint missing");
        assert_eq!(
            outcome,
            ProbeDetailsOutcome::NotPaymentRequired {
                status: 404,
                body_snippet: Some("endpoint missing".into()),
            }
        );
    }

    #[test]
    fn probe_body_snippet_truncates_long_bodies() {
        let body = "x".repeat(300);
        let snippet = probe_body_snippet(&body).expect("snippet");
        assert!(snippet.len() <= PROBE_BODY_SNIPPET_MAX + 3);
        assert!(snippet.ends_with('…'));
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
    fn probe_history_status_maps_outcomes() {
        let live = ProbeOutcome::Verified {
            amount: Some("1000".into()),
            asset: Some("usdc".into()),
        };
        let (status, _) = probe_history_status(&live, Some("$1,000 USDC"));
        assert_eq!(status, "live");

        let mismatch = ProbeOutcome::Verified {
            amount: Some("2000".into()),
            asset: Some("usdc".into()),
        };
        let (status, _) = probe_history_status(&mismatch, Some("0.001 usdc"));
        assert_eq!(status, "price_mismatch");

        let (status, _) = probe_history_status(&ProbeOutcome::NotPaymentRequired, None);
        assert_eq!(status, "dead");
    }

    #[test]
    fn l4_consecutive_failure_requires_fourteen_non_live_days() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 7, 7).expect("date");
        let dead: Vec<(chrono::NaiveDate, String)> = (0..14)
            .map(|offset| (today - chrono::Duration::days(offset), "dead".to_string()))
            .collect();
        assert!(l4_consecutive_failure_days_met(&dead, today));

        let mixed: Vec<(chrono::NaiveDate, String)> = (0..14)
            .map(|offset| {
                (
                    today - chrono::Duration::days(offset),
                    if offset == 0 {
                        "live".into()
                    } else {
                        "dead".into()
                    },
                )
            })
            .collect();
        assert!(!l4_consecutive_failure_days_met(&mixed, today));

        let short: Vec<(chrono::NaiveDate, String)> = (0..13)
            .map(|offset| (today - chrono::Duration::days(offset), "dead".to_string()))
            .collect();
        assert!(!l4_consecutive_failure_days_met(&short, today));

        let gap: Vec<(chrono::NaiveDate, String)> = (0..14)
            .map(|offset| {
                let day_offset = if offset == 0 { 0 } else { offset + 1 };
                (
                    today - chrono::Duration::days(day_offset),
                    "dead".to_string(),
                )
            })
            .collect();
        assert!(!l4_consecutive_failure_days_met(&gap, today));
    }

    #[test]
    fn k2_snapshot_flags_ignore_prior_catalog_verification() {
        let dead = ProbeOutcome::NotPaymentRequired;
        let (catalog_ev, _, _) = apply_outcome_to_flags(&dead, None, true, 0);
        assert!(catalog_ev, "catalog flag can stay true after one failure");
        let (k2_ev, _, _) = apply_outcome_to_flags(&dead, None, false, 0);
        assert!(!k2_ev, "K2 snapshot must reflect this probe only");
    }

    #[test]
    fn apply_outcome_demotes_after_three_failures() {
        let (endpoint, price, failures) =
            apply_outcome_to_flags(&ProbeOutcome::NotPaymentRequired, Some("1000"), true, 1);
        assert!(endpoint);
        assert!(!price);
        assert_eq!(failures, 2);

        let (endpoint, _, failures) =
            apply_outcome_to_flags(&ProbeOutcome::NotPaymentRequired, Some("1000"), true, 2);
        assert!(!endpoint);
        assert_eq!(failures, 3);
    }
}
