use super::*;

/// Row shape for category listings with live approved-tool counts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct CategoryWithCount {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub description: String,
    pub sort_order: i32,
    pub count: i64,
}

impl CategoryWithCount {
    pub fn into_pair(self) -> (Category, i64) {
        (
            Category {
                id: self.id,
                label: self.label,
                icon: self.icon,
                description: self.description,
                sort_order: self.sort_order,
            },
            self.count,
        )
    }
}

/// Returns the public site settings singleton (slogan, description, MCP endpoint).
#[server(GetSiteSettings, "/api")]
pub async fn get_site_settings() -> Result<SiteSettings, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let settings = sqlx::query_as::<_, SiteSettings>("SELECT * FROM site_settings WHERE id = 1")
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load site settings: {e}")))?;

    Ok(sanitize_site_settings_for_public(settings))
}

/// Admin-only site settings (includes referral defaults and builder code).
#[server(GetAdminSiteSettings, "/api")]
pub async fn get_admin_site_settings() -> Result<SiteSettings, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config)
        .await
        .map_err(ServerFnError::new)?;

    let settings = sqlx::query_as::<_, SiteSettings>("SELECT * FROM site_settings WHERE id = 1")
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load site settings: {e}")))?;

    Ok(settings)
}

/// Parse comma- or newline-separated crawler keywords.
pub(crate) fn parse_search_keywords(raw: &str) -> Vec<String> {
    raw.split([',', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Validate admin site settings input before persisting.
pub(crate) struct SiteSettingsValidationInput<'a> {
    pub site_name: &'a str,
    pub slogan: &'a str,
    pub description: &'a str,
    pub mcp_endpoint: &'a str,
    pub search_keywords: &'a [String],
    pub default_referral_bps: Option<i32>,
    pub default_referral_payout_address: Option<&'a str>,
    pub x402_builder_code: Option<&'a str>,
}

pub(crate) fn validate_update_site_settings_input(
    input: SiteSettingsValidationInput<'_>,
) -> Result<(), &'static str> {
    validate_required_text(input.site_name, 100, "site name must be 1–100 characters")?;
    validate_required_text(input.slogan, 200, "slogan must be 1–200 characters")?;
    validate_required_text(
        input.description,
        500,
        "description must be 1–500 characters",
    )?;
    validate_required_text(
        input.mcp_endpoint,
        200,
        "MCP endpoint must be 1–200 characters",
    )?;
    validate_search_keywords(input.search_keywords)?;
    validate_referral_bps(input.default_referral_bps)?;
    validate_optional_text_len(
        input.default_referral_payout_address,
        200,
        "default referral payout address must be 200 characters or fewer",
    )?;
    validate_optional_text_len(
        input.x402_builder_code,
        100,
        "x402 builder code must be 100 characters or fewer",
    )
}

fn validate_required_text(
    value: &str,
    max_len: usize,
    message: &'static str,
) -> Result<(), &'static str> {
    let value = value.trim();
    (!value.is_empty() && value.chars().count() <= max_len)
        .then_some(())
        .ok_or(message)
}

fn validate_search_keywords(search_keywords: &[String]) -> Result<(), &'static str> {
    validate_keyword_count(search_keywords)?;
    search_keywords
        .iter()
        .try_for_each(|keyword| validate_search_keyword(keyword))
}

fn validate_keyword_count(search_keywords: &[String]) -> Result<(), &'static str> {
    (!search_keywords.is_empty() && search_keywords.len() <= 50)
        .then_some(())
        .ok_or("provide 1–50 search keywords")
}

fn validate_search_keyword(keyword: &str) -> Result<(), &'static str> {
    validate_required_text(keyword, 64, "each keyword must be 1–64 characters")?;
    keyword_chars_are_allowed(keyword.trim())
        .then_some(())
        .ok_or("keywords may only contain letters, numbers, hyphens, and underscores")
}

fn keyword_chars_are_allowed(keyword: &str) -> bool {
    keyword
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn validate_referral_bps(bps: Option<i32>) -> Result<(), &'static str> {
    bps.map_or(Ok(()), |bps| {
        (0..=10_000)
            .contains(&bps)
            .then_some(())
            .ok_or("default referral bps must be 0–10000")
    })
}

fn validate_optional_text_len(
    value: Option<&str>,
    max_len: usize,
    message: &'static str,
) -> Result<(), &'static str> {
    value.map(str::trim).map_or(Ok(()), |text| {
        (text.chars().count() <= max_len)
            .then_some(())
            .ok_or(message)
    })
}

/// Payload for admin site settings updates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateSiteSettingsPayload {
    pub site_name: String,
    pub slogan: String,
    pub description: String,
    pub mcp_endpoint: String,
    pub search_keywords_raw: String,
    pub allow_free_registration: bool,
    pub require_tool_approval: bool,
    pub allow_x402_registration: bool,
    pub default_referral_bps: Option<i32>,
    pub default_referral_payout_address: Option<String>,
    pub x402_builder_code: Option<String>,
}

/// Admin-only update of the `site_settings` singleton (id = 1).
#[server(UpdateSiteSettings, "/api")]
pub async fn update_site_settings(
    payload: UpdateSiteSettingsPayload,
) -> Result<SiteSettings, ServerFnError> {
    let keywords = parse_search_keywords(&payload.search_keywords_raw);
    if let Err(msg) = validate_update_site_settings_input(SiteSettingsValidationInput {
        site_name: &payload.site_name,
        slogan: &payload.slogan,
        description: &payload.description,
        mcp_endpoint: &payload.mcp_endpoint,
        search_keywords: &keywords,
        default_referral_bps: payload.default_referral_bps,
        default_referral_payout_address: payload.default_referral_payout_address.as_deref(),
        x402_builder_code: payload.x402_builder_code.as_deref(),
    }) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config)
        .await
        .map_err(ServerFnError::new)?;

    let settings = sqlx::query_as::<_, SiteSettings>(
        r#"
        UPDATE site_settings
        SET site_name = $1,
            slogan = $2,
            description = $3,
            mcp_endpoint = $4,
            search_keywords = $5,
            allow_free_registration = $6,
            require_tool_approval = $7,
            allow_x402_registration = $8,
            default_referral_bps = $9,
            default_referral_payout_address = $10,
            x402_builder_code = $11,
            updated_at = now()
        WHERE id = 1
        RETURNING *
        "#,
    )
    .bind(payload.site_name.trim())
    .bind(payload.slogan.trim())
    .bind(payload.description.trim())
    .bind(payload.mcp_endpoint.trim())
    .bind(&keywords)
    .bind(payload.allow_free_registration)
    .bind(payload.require_tool_approval)
    .bind(payload.allow_x402_registration)
    .bind(payload.default_referral_bps)
    .bind(
        payload
            .default_referral_payout_address
            .as_deref()
            .map(str::trim),
    )
    .bind(payload.x402_builder_code.as_deref().map(str::trim))
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to update site settings: {e}")))?;

    Ok(settings)
}
