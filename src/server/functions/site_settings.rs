use super::*;

use crate::models::FooterLink;

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
    pub mcp_premium_enabled: bool,
    pub mcp_premium_pay_to_address: Option<&'a str>,
    pub mcp_premium_price: Option<&'a str>,
    pub mcp_premium_network: &'a str,
    pub mcp_premium_asset: Option<&'a str>,
    pub mcp_premium_display_price: Option<&'a str>,
    pub hero_title: Option<&'a str>,
    pub hero_subtitle: Option<&'a str>,
    pub about_content: Option<&'a str>,
    pub footer_links: &'a [FooterLink],
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
    )?;
    validate_mcp_premium_settings(
        input.mcp_premium_enabled,
        input.mcp_premium_pay_to_address,
        input.mcp_premium_price,
        input.mcp_premium_network,
        input.mcp_premium_asset,
        input.mcp_premium_display_price,
    )?;
    validate_optional_text_len(
        input.hero_title,
        200,
        "hero title must be 200 characters or fewer",
    )?;
    validate_optional_text_len(
        input.hero_subtitle,
        300,
        "hero subtitle must be 300 characters or fewer",
    )?;
    validate_optional_text_len(
        input.about_content,
        10_000,
        "about content must be 10000 characters or fewer",
    )?;
    validate_footer_links(input.footer_links)
}

fn validate_footer_links(footer_links: &[FooterLink]) -> Result<(), &'static str> {
    (footer_links.len() <= 20)
        .then_some(())
        .ok_or("footer links must be 20 or fewer")?;
    footer_links
        .iter()
        .try_for_each(|link| validate_footer_link(link))
}

fn validate_footer_link(link: &FooterLink) -> Result<(), &'static str> {
    validate_required_text(
        &link.label,
        100,
        "each footer link label must be 1–100 characters",
    )?;
    let url = link.url.trim();
    validate_required_text(url, 500, "each footer link url must be 1–500 characters")?;
    (url.starts_with("http://") || url.starts_with("https://"))
        .then_some(())
        .ok_or("footer link urls must start with http:// or https://")
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

pub(crate) fn validate_mcp_premium_settings(
    enabled: bool,
    pay_to: Option<&str>,
    price: Option<&str>,
    network: &str,
    asset: Option<&str>,
    display_price: Option<&str>,
) -> Result<(), &'static str> {
    let network = network.trim();
    let valid_eip155 = network.starts_with("eip155:")
        && network
            .strip_prefix("eip155:")
            .is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()));
    let valid_solana = network.starts_with("solana:")
        && network.strip_prefix("solana:").is_some_and(|c| !c.is_empty());
    if !(valid_eip155 || valid_solana) {
        return Err("MCP premium network must be eip155:<chainId> or solana:<cluster>");
    }
    validate_optional_text_len(asset, 200, "MCP premium asset must be 200 characters or fewer")?;
    validate_optional_text_len(
        display_price,
        100,
        "MCP premium display price must be 100 characters or fewer",
    )?;
    if !enabled {
        return Ok(());
    }
    let pay_to = pay_to.map(str::trim).filter(|s| !s.is_empty());
    let price = price.map(str::trim).filter(|s| !s.is_empty());
    let pay_to = pay_to.ok_or("MCP premium pay-to address is required when enabled")?;
    let price = price.ok_or("MCP premium price is required when enabled")?;
    if !pay_to.starts_with("0x") || pay_to.len() != 42 {
        return Err("MCP premium pay-to must be a 42-character 0x EVM address");
    }
    validate_optional_text_len(Some(price), 50, "MCP premium price must be 50 characters or fewer")?;
    Ok(())
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
    pub mcp_premium_enabled: bool,
    pub mcp_premium_pay_to_address: Option<String>,
    pub mcp_premium_price: Option<String>,
    pub mcp_premium_network: String,
    pub mcp_premium_asset: Option<String>,
    pub mcp_premium_display_price: Option<String>,
    pub hero_title: Option<String>,
    pub hero_subtitle: Option<String>,
    pub about_content: Option<String>,
    pub footer_links: Vec<FooterLink>,
}
