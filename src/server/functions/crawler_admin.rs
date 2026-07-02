use super::*;

/// Known crawler sources for the admin dashboard (merged with DB rows).
pub(crate) const CRAWLER_SOURCE_DEFS: &[(&str, &str)] = &[
    ("cryptoskill", "Every 6h"),
    ("github", "Hourly (+30m offset)"),
    ("npm", "Hourly"),
    ("web3-mcp-hub", "Every 12h"),
];

/// Admin crawler row — source status plus schedule hint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrawlerSourceView {
    pub id: Option<uuid::Uuid>,
    pub name: String,
    pub url: String,
    pub schedule: String,
    pub schedule_minutes: i32,
    pub enabled: bool,
    pub last_crawled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub crawl_status: String,
    pub items_found: i32,
    pub error_message: Option<String>,
}

/// Payload for admin crawler source schedule updates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateCrawlerSourcePayload {
    pub schedule_minutes: i32,
    pub enabled: bool,
}

/// Default schedule interval (minutes) for known crawler sources without a DB row.
pub(crate) fn default_schedule_minutes_for_source(name: &str) -> i32 {
    match name {
        "npm" | "github" => 60,
        "cryptoskill" => 360,
        "web3-mcp-hub" => 720,
        _ => 360,
    }
}

/// Human-readable schedule label from interval minutes.
pub(crate) fn format_schedule_minutes(minutes: i32) -> String {
    if minutes < 60 {
        return format!("Every {minutes}m");
    }
    if minutes % 60 == 0 {
        let hours = minutes / 60;
        return if hours == 1 {
            "Every 1h".into()
        } else {
            format!("Every {hours}h")
        };
    }
    format!("Every {minutes}m")
}

/// Validate crawler schedule update input.
pub(crate) fn validate_update_crawler_source(schedule_minutes: i32) -> Result<(), &'static str> {
    (5..=10_080)
        .contains(&schedule_minutes)
        .then_some(())
        .ok_or("schedule must be 5–10080 minutes")
}

/// Build crawler source rows for admin views (shared by dashboard and crawler page).
#[cfg(feature = "ssr")]
pub(crate) async fn list_crawler_sources_inner(
    pool: &sqlx::PgPool,
) -> Result<Vec<CrawlerSourceView>, ServerFnError> {
    let rows = sqlx::query_as::<_, Source>("SELECT * FROM sources ORDER BY name ASC")
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to list sources: {e}")))?;

    let mut by_name: std::collections::HashMap<String, Source> =
        rows.into_iter().map(|r| (r.name.clone(), r)).collect();

    let mut views = Vec::with_capacity(CRAWLER_SOURCE_DEFS.len() + 1);
    for (name, _schedule) in CRAWLER_SOURCE_DEFS {
        let url = default_source_registry_url(name).to_string();
        if let Some(row) = by_name.remove(*name) {
            views.push(CrawlerSourceView {
                id: Some(row.id),
                name: row.name,
                url: row.url,
                schedule: format_schedule_minutes(row.schedule_minutes),
                schedule_minutes: row.schedule_minutes,
                enabled: row.enabled,
                last_crawled_at: row.last_crawled_at,
                crawl_status: row.crawl_status,
                items_found: row.items_found,
                error_message: row.error_message,
            });
        } else {
            let schedule_minutes = default_schedule_minutes_for_source(name);
            views.push(CrawlerSourceView {
                id: None,
                name: (*name).into(),
                url,
                schedule: format_schedule_minutes(schedule_minutes),
                schedule_minutes,
                enabled: true,
                last_crawled_at: None,
                crawl_status: "pending".into(),
                items_found: 0,
                error_message: None,
            });
        }
    }

    for (_, row) in by_name {
        views.push(CrawlerSourceView {
            id: Some(row.id),
            name: row.name,
            url: row.url,
            schedule: format_schedule_minutes(row.schedule_minutes),
            schedule_minutes: row.schedule_minutes,
            enabled: row.enabled,
            last_crawled_at: row.last_crawled_at,
            crawl_status: row.crawl_status,
            items_found: row.items_found,
            error_message: row.error_message,
        });
    }

    Ok(views)
}

/// List crawler source status (admin).
#[server(ListCrawlerSources, "/api")]
pub async fn list_crawler_sources() -> Result<Vec<CrawlerSourceView>, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;
    list_crawler_sources_inner(&pool).await
}

/// Validate manual crawler trigger input.
pub(crate) fn validate_trigger_crawler_source(source: &str) -> Result<(), &'static str> {
    match source {
        "npm" | "cryptoskill" | "web3-mcp-hub" | "github" | "sync_stars" => Ok(()),
        _ => Err("unknown crawler source"),
    }
}

/// Update crawler source schedule and enabled flag (admin).
#[cfg(feature = "ssr")]
pub(crate) async fn update_crawler_source_inner(
    pool: &sqlx::PgPool,
    id: uuid::Uuid,
    schedule_minutes: i32,
    enabled: bool,
) -> Result<Source, ServerFnError> {
    let row = sqlx::query_as::<_, Source>(
        r#"
        UPDATE sources
        SET schedule_minutes = $1,
            enabled = $2,
            updated_at = now()
        WHERE id = $3
        RETURNING *
        "#,
    )
    .bind(schedule_minutes)
    .bind(enabled)
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to update crawler source: {e}")))?;

    row.ok_or_else(|| ServerFnError::new("crawler source not found"))
}

/// Update crawler source schedule and enabled flag (admin).
#[server(UpdateCrawlerSource, "/api")]
pub async fn update_crawler_source(
    id: uuid::Uuid,
    payload: UpdateCrawlerSourcePayload,
) -> Result<Source, ServerFnError> {
    if let Err(msg) = validate_update_crawler_source(payload.schedule_minutes) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    update_crawler_source_inner(&pool, id, payload.schedule_minutes, payload.enabled).await
}

/// Manually trigger a crawler job in the background (admin).
#[server(TriggerCrawlerSource, "/api")]
pub async fn trigger_crawler_source(source: String) -> Result<(), ServerFnError> {
    if let Err(msg) = validate_trigger_crawler_source(&source) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let pool_bg = pool.clone();
    let source_bg = source.clone();
    tokio::spawn(async move {
        crawler::trigger_source(&pool_bg, &source_bg).await;
    });

    Ok(())
}
