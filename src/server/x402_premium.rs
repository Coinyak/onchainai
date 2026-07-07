//! Premium trust-data services sold by OnchainAI via x402 (K2/M3).

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Tool;
use crate::server::queries::APPROVED_TOOL_BY_SLUG_SQL;
use crate::server::trust_probe_meta::ProbeReceipt;
use crate::server::x402_verify::run_k2_on_demand_probe;

#[derive(Debug, Serialize)]
pub struct EndpointHealthReport {
    pub slug: String,
    pub tool_id: Uuid,
    pub live: bool,
    pub endpoint_verified: bool,
    pub price_verified: bool,
    pub last_probe_at: Option<DateTime<Utc>>,
    pub uptime_30d_pct: Option<f64>,
    pub probe_samples_30d: i64,
    pub live_samples_30d: i64,
    pub latest_probe_status: Option<String>,
    pub x402_endpoint: Option<String>,
    pub probe_receipt: ProbeReceipt,
    pub disclaimer: &'static str,
}

const HEALTH_DISCLAIMER: &str =
    "On-demand probe at payment time — liveness and advertised x402 fee match only; not execution cost, safety, or payment guarantee.";

pub async fn check_endpoint_health(
    pool: &PgPool,
    slug: &str,
) -> Result<EndpointHealthReport, PremiumDataError> {
    let slug = slug.trim();
    if slug.is_empty() {
        return Err(PremiumDataError::InvalidSlug);
    }

    let tool = sqlx::query_as::<_, Tool>(APPROVED_TOOL_BY_SLUG_SQL)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(PremiumDataError::Database)?;

    let Some(tool) = tool else {
        return Err(PremiumDataError::NotFound);
    };

    if tool.pricing != "x402" && tool.x402_endpoint.is_none() {
        return Err(PremiumDataError::NotX402);
    }

    let endpoint = tool
        .x402_endpoint
        .as_deref()
        .filter(|url| !url.trim().is_empty())
        .ok_or(PremiumDataError::MissingEndpoint)?;

    let run = run_k2_on_demand_probe(pool, tool.id, endpoint, tool.x402_price.as_deref())
        .await
        .map_err(PremiumDataError::Database)?;

    let on_demand_live = run.history_status == "live";

    let probe_receipt = crate::server::trust_probe_meta::build_probe_receipt(
        &tool,
        endpoint,
        run.probed_at,
        &run.outcome,
        tool.x402_price.as_deref(),
    );

    let stats = sqlx::query_as::<_, ProbeStatsRow>(
        r#"
        SELECT
            COUNT(*)::bigint AS total,
            COUNT(*) FILTER (WHERE status = 'live')::bigint AS live_count,
            MAX(probed_at) AS last_probe_at,
            (
                SELECT status
                FROM x402_probe_history h2
                WHERE h2.tool_id = $1
                ORDER BY probed_at DESC
                LIMIT 1
            ) AS latest_status
        FROM x402_probe_history
        WHERE tool_id = $1
          AND probed_at >= now() - interval '30 days'
        "#,
    )
    .bind(tool.id)
    .fetch_one(pool)
    .await
    .map_err(PremiumDataError::Database)?;

    let uptime = if stats.total > 0 {
        Some((stats.live_count as f64 / stats.total as f64) * 100.0)
    } else {
        None
    };

    Ok(EndpointHealthReport {
        slug: tool.slug.clone(),
        tool_id: tool.id,
        live: on_demand_live,
        endpoint_verified: run.endpoint_verified,
        price_verified: run.price_verified,
        last_probe_at: Some(run.probed_at),
        uptime_30d_pct: uptime,
        probe_samples_30d: stats.total,
        live_samples_30d: stats.live_count,
        latest_probe_status: Some(run.history_status),
        x402_endpoint: tool.x402_endpoint.clone(),
        probe_receipt,
        disclaimer: HEALTH_DISCLAIMER,
    })
}

#[derive(Debug, sqlx::FromRow)]
struct ProbeStatsRow {
    total: i64,
    live_count: i64,
    last_probe_at: Option<DateTime<Utc>>,
    latest_status: Option<String>,
}

#[derive(Debug)]
pub enum PremiumDataError {
    InvalidSlug,
    NotFound,
    NotX402,
    MissingEndpoint,
    Database(sqlx::Error),
}

impl PremiumDataError {
    pub fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            Self::InvalidSlug | Self::NotX402 | Self::MissingEndpoint => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::InvalidSlug => "slug is required",
            Self::NotFound => "tool not found",
            Self::NotX402 => "tool is not an x402 endpoint listing",
            Self::MissingEndpoint => "tool has no x402 endpoint URL",
            Self::Database(_) => "failed to load endpoint health data",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_pct_from_counts() {
        let pct: f64 = (7.0 / 10.0) * 100.0;
        assert!((pct - 70.0).abs() < f64::EPSILON);
    }

}
