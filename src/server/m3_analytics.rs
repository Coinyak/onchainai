//! M3 analytics — price history and x402 trends from probe history data.
//!
//! Aggregates `x402_probe_history` into price timelines and ecosystem trends.
//! Free discovery/metadata (OD-FTG §2). Spec: docs/MVP_DESIGN.md §11 (M3 roadmap).

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

const DEFAULT_DAYS: i64 = 30;
const MAX_DAYS: i64 = 90;

#[derive(Debug)]
pub enum AnalyticsError {
    NotFound,
    NotX402,
    Database(sqlx::Error),
}

impl AnalyticsError {
    pub fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::NotX402 => StatusCode::BAD_REQUEST,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::NotFound => "tool not found",
            Self::NotX402 => "tool is not an x402 listing",
            Self::Database(_) => "failed to load analytics data",
        }
    }
}

fn clamp_days(days: Option<i64>) -> i32 {
    days.unwrap_or(DEFAULT_DAYS).clamp(1, MAX_DAYS) as i32
}

// ─── get_price_history ───

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PriceHistoryPoint {
    pub probed_at: DateTime<Utc>,
    pub status: String,
    pub actual_price: Option<String>,
    pub latency_ms: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PriceHistoryResponse {
    pub slug: String,
    pub tool_id: Uuid,
    pub days: i64,
    pub history: Vec<PriceHistoryPoint>,
    pub live_count: i64,
    pub total_count: i64,
    pub disclaimer: &'static str,
}

const PRICE_HISTORY_DISCLAIMER: &str =
    "Price history from x402 probe records at time T. Returns up to 500 most recent probes; \
     total_count and live_count cover the full window. Actual prices are from 402 handshake \
     responses — advertised prices may differ. Not financial advice.";

#[derive(Debug, sqlx::FromRow)]
struct ProbeWindowCounts {
    total_count: i64,
    live_count: i64,
}

pub async fn get_price_history(
    pool: &PgPool,
    slug: &str,
    days: Option<i64>,
) -> Result<PriceHistoryResponse, AnalyticsError> {
    use crate::models::Tool;
    use crate::server::queries::APPROVED_TOOL_BY_SLUG_SQL;

    let slug = slug.trim();
    if slug.is_empty() {
        return Err(AnalyticsError::NotFound);
    }

    let tool = sqlx::query_as::<_, Tool>(APPROVED_TOOL_BY_SLUG_SQL)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(AnalyticsError::Database)?;

    let Some(tool) = tool else {
        return Err(AnalyticsError::NotFound);
    };

    if tool.pricing != "x402" && tool.x402_endpoint.is_none() {
        return Err(AnalyticsError::NotX402);
    }

    let days = clamp_days(days);

    let counts = sqlx::query_as::<_, ProbeWindowCounts>(
        r#"
        SELECT
            COUNT(*)::bigint AS total_count,
            COUNT(*) FILTER (WHERE status = 'live')::bigint AS live_count
        FROM x402_probe_history
        WHERE tool_id = $1
          AND probed_at >= now() - make_interval(days => $2)
        "#,
    )
    .bind(tool.id)
    .bind(days)
    .fetch_one(pool)
    .await
    .map_err(AnalyticsError::Database)?;

    let history = sqlx::query_as::<_, PriceHistoryPoint>(
        r#"
        SELECT probed_at, status, actual_price, latency_ms
        FROM x402_probe_history
        WHERE tool_id = $1
          AND probed_at >= now() - make_interval(days => $2)
        ORDER BY probed_at DESC
        LIMIT 500
        "#,
    )
    .bind(tool.id)
    .bind(days)
    .fetch_all(pool)
    .await
    .map_err(AnalyticsError::Database)?;

    Ok(PriceHistoryResponse {
        slug: tool.slug.clone(),
        tool_id: tool.id,
        days: i64::from(days),
        history,
        live_count: counts.live_count,
        total_count: counts.total_count,
        disclaimer: PRICE_HISTORY_DISCLAIMER,
    })
}

// ─── get_x402_trends ───

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ToolTrendRow {
    pub slug: String,
    pub tool_id: Uuid,
    pub total_probes: i64,
    pub live_probes: i64,
    pub live_rate_pct: Option<f64>,
    pub latest_price: Option<String>,
    pub latest_status: Option<String>,
    pub latest_probe_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct X402TrendsResponse {
    pub days: i64,
    pub tools: Vec<ToolTrendRow>,
    pub total_tools: i64,
    pub disclaimer: &'static str,
}

const TRENDS_DISCLAIMER: &str =
    "Aggregated x402 probe trends at time T. Live rate = live probes / total probes per tool. \
     Latest price is the most recent actual 402 handshake amount within the window. Not financial advice.";

pub async fn get_x402_trends(
    pool: &PgPool,
    days: Option<i64>,
) -> Result<X402TrendsResponse, AnalyticsError> {
    let days = clamp_days(days);

    let tools = sqlx::query_as::<_, ToolTrendRow>(
        r#"
        SELECT
            t.slug,
            t.id AS tool_id,
            COUNT(h.id)::bigint AS total_probes,
            COUNT(h.id) FILTER (WHERE h.status = 'live')::bigint AS live_probes,
            CASE WHEN COUNT(h.id) > 0
                THEN (COUNT(h.id) FILTER (WHERE h.status = 'live')::float / COUNT(h.id)::float) * 100.0
                ELSE NULL
            END AS live_rate_pct,
            (
                SELECT h2.actual_price
                FROM x402_probe_history h2
                WHERE h2.tool_id = t.id
                  AND h2.actual_price IS NOT NULL
                  AND h2.probed_at >= now() - make_interval(days => $1)
                ORDER BY h2.probed_at DESC
                LIMIT 1
            ) AS latest_price,
            (
                SELECT h3.status
                FROM x402_probe_history h3
                WHERE h3.tool_id = t.id
                  AND h3.probed_at >= now() - make_interval(days => $1)
                ORDER BY h3.probed_at DESC
                LIMIT 1
            ) AS latest_status,
            MAX(h.probed_at) AS latest_probe_at
        FROM tools t
        INNER JOIN x402_probe_history h ON h.tool_id = t.id
        WHERE h.probed_at >= now() - make_interval(days => $1)
          AND (t.pricing = 'x402' OR t.x402_endpoint IS NOT NULL)
        GROUP BY t.slug, t.id
        ORDER BY live_probes DESC, total_probes DESC
        LIMIT 50
        "#,
    )
    .bind(days)
    .fetch_all(pool)
    .await
    .map_err(AnalyticsError::Database)?;

    let total_tools = tools.len() as i64;

    Ok(X402TrendsResponse {
        days: i64::from(days),
        tools,
        total_tools,
        disclaimer: TRENDS_DISCLAIMER,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_days_defaults_to_30() {
        assert_eq!(clamp_days(None), 30);
    }

    #[test]
    fn clamp_days_clamps_to_range() {
        assert_eq!(clamp_days(Some(0)), 1);
        assert_eq!(clamp_days(Some(100)), 90);
        assert_eq!(clamp_days(Some(7)), 7);
    }

    #[test]
    fn price_history_disclaimer_documents_capped_history() {
        assert!(PRICE_HISTORY_DISCLAIMER.contains("500"));
        assert!(PRICE_HISTORY_DISCLAIMER.contains("total_count"));
    }
}
