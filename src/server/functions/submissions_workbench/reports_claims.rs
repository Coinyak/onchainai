use super::*;

/// Payload for reporting a published listing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportToolInput {
    pub slug: String,
    pub reason: String,
    pub details: Option<String>,
}

/// Payload for requesting project claim with proof-oriented fields.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClaimToolInput {
    pub slug: String,
    pub verification_note: String,
    pub contact_email: Option<String>,
    pub team_name: Option<String>,
    pub github_url: Option<String>,
    pub website_url: Option<String>,
    pub x_url: Option<String>,
    pub proof_links: Vec<String>,
}

/// Validate report reason against allowlist.
pub(crate) fn validate_report_reason(reason: &str) -> Result<(), &'static str> {
    if TOOL_REPORT_REASONS.iter().any(|(k, _)| *k == reason) {
        Ok(())
    } else {
        Err("invalid report reason")
    }
}

/// Validate report details length.
pub(crate) fn validate_report_details(details: Option<&str>) -> Result<(), &'static str> {
    if let Some(text) = details.map(str::trim).filter(|s| !s.is_empty()) {
        if text.len() > 1000 {
            return Err("report details must be at most 1000 characters");
        }
    }
    Ok(())
}

pub(crate) const MAX_CLAIM_PROOF_LINKS: usize = 10;
pub(crate) const MAX_CLAIM_VERIFICATION_NOTE_TOTAL: usize = 4000;

/// Validate optional proof URLs for claim flow.
pub(crate) fn validate_claim_proof_urls(urls: &[String]) -> Result<(), &'static str> {
    let non_empty = urls.iter().filter(|u| !u.trim().is_empty()).count();
    if non_empty > MAX_CLAIM_PROOF_LINKS {
        return Err("at most 10 proof links allowed");
    }
    for url in urls {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            continue;
        }
        validate_claim_proof_url(trimmed)?;
    }
    Ok(())
}

fn validate_claim_proof_url(url: &str) -> Result<(), &'static str> {
    validate_claim_proof_url_scheme(url)?;
    validate_claim_proof_url_len(url)
}

fn validate_claim_proof_url_scheme(url: &str) -> Result<(), &'static str> {
    claim_proof_url_scheme_allowed(url)
        .then_some(())
        .ok_or("proof links must use https:// (http://localhost allowed in dev)")
}

fn claim_proof_url_scheme_allowed(url: &str) -> bool {
    url.starts_with("https://") || is_dev_loopback_http_url(url)
}

fn validate_claim_proof_url_len(url: &str) -> Result<(), &'static str> {
    (url.len() <= 500)
        .then_some(())
        .ok_or("proof link must be at most 500 characters")
}

/// Build the stored verification note after team name and proof links are appended.
pub(crate) fn build_claim_proof_note(input: &ClaimToolInput) -> Result<String, &'static str> {
    let mut proof_note = input.verification_note.trim().to_string();
    if !input.proof_links.is_empty() {
        let links = input
            .proof_links
            .iter()
            .map(|u| u.trim())
            .filter(|u| !u.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if !links.is_empty() {
            proof_note = format!("{proof_note}\n\nProof links:\n{links}");
        }
    }
    if let Some(team) = input
        .team_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        proof_note = format!("Team: {team}\n{proof_note}");
    }
    if proof_note.len() > MAX_CLAIM_VERIFICATION_NOTE_TOTAL {
        return Err("verification note must be at most 4000 characters after formatting");
    }
    Ok(proof_note)
}

/// Validate claim request input with proof-oriented fields.
pub(crate) fn validate_claim_tool_input(input: &ClaimToolInput) -> Result<(), &'static str> {
    validate_claim_slug(&input.slug)?;
    validate_claim_note(&input.verification_note)?;
    validate_claim_email(input.contact_email.as_deref())?;
    validate_claim_team(input.team_name.as_deref())?;
    validate_claim_urls(input)?;
    build_claim_proof_note(input)?;
    Ok(())
}

fn validate_claim_slug(slug: &str) -> Result<(), &'static str> {
    let slug = slug.trim();
    (!slug.is_empty() && slug.len() <= 120)
        .then_some(())
        .ok_or("tool slug is required")
}

fn validate_claim_note(note: &str) -> Result<(), &'static str> {
    let note = note.trim();
    (!note.is_empty())
        .then_some(())
        .ok_or("verification note is required for claim requests")?;
    (20..=2000)
        .contains(&note.len())
        .then_some(())
        .ok_or("verification note must be 20–2000 characters")
}

fn validate_claim_email(email: Option<&str>) -> Result<(), &'static str> {
    let Some(email) = normalized_claim_optional(email) else {
        return Ok(());
    };
    (email.len() <= 200 && email.contains('@'))
        .then_some(())
        .ok_or("contact email is invalid")
}

fn validate_claim_team(team: Option<&str>) -> Result<(), &'static str> {
    let Some(team) = normalized_claim_optional(team) else {
        return Ok(());
    };
    (team.len() <= 200)
        .then_some(())
        .ok_or("team name must be at most 200 characters")
}

fn validate_claim_urls(input: &ClaimToolInput) -> Result<(), &'static str> {
    validate_optional_https_url(input.github_url.as_deref())?;
    validate_optional_https_url(input.website_url.as_deref())?;
    validate_optional_https_url(input.x_url.as_deref())?;
    validate_claim_proof_urls(&input.proof_links)
}

fn normalized_claim_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|s| !s.is_empty())
}

#[cfg(feature = "ssr")]
async fn claim_tool_by_slug(pool: &sqlx::PgPool, slug: &str) -> Result<Tool, FnError> {
    sqlx::query_as::<_, Tool>(APPROVED_TOOL_BY_SLUG_SQL)
        .bind(slug.trim())
        .fetch_optional(pool)
        .await
        .map_err(|e| FnError::new(format!("failed to resolve tool: {e}")))?
        .ok_or_else(|| FnError::new("tool not found"))
}

fn validate_claim_state_available(tool: &Tool) -> Result<(), FnError> {
    match tool.claim_state.as_str() {
        "claimed" => Err(FnError::new("this listing is already claimed")),
        "claim_pending" => Err(FnError::new("a claim request is already pending review")),
        _ => Ok(()),
    }
}

#[cfg(feature = "ssr")]
async fn insert_claim_request_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tool_id: Uuid,
    user_id: Uuid,
    proof_note: &str,
    contact_email: Option<String>,
) -> Result<ToolClaimRequest, FnError> {
    sqlx::query_as::<_, ToolClaimRequest>(
        r#"
        INSERT INTO tool_claim_requests (tool_id, requested_by, verification_note, contact_email, status)
        VALUES ($1, $2, $3, $4, 'pending')
        RETURNING *
        "#,
    )
    .bind(tool_id)
    .bind(user_id)
    .bind(proof_note)
    .bind(contact_email)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| FnError::new(format!("failed to save claim request: {e}")))
}

#[cfg(feature = "ssr")]
async fn insert_claim_official_links(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tool_id: Uuid,
    input: &ClaimToolInput,
) -> Result<(), FnError> {
    insert_claim_official_link(tx, tool_id, ClaimOfficialLinkCandidate::github(input)).await?;
    insert_claim_official_link(tx, tool_id, ClaimOfficialLinkCandidate::website(input)).await?;
    insert_claim_official_link(tx, tool_id, ClaimOfficialLinkCandidate::x(input)).await
}

struct ClaimOfficialLinkCandidate<'a> {
    link_type: &'static str,
    url: Option<&'a str>,
    label: &'static str,
    source: &'static str,
}

impl<'a> ClaimOfficialLinkCandidate<'a> {
    fn github(input: &'a ClaimToolInput) -> Self {
        Self::new(
            "github",
            input.github_url.as_deref(),
            "Claimed GitHub",
            "claim:github",
        )
    }

    fn website(input: &'a ClaimToolInput) -> Self {
        Self::new(
            "website",
            input.website_url.as_deref(),
            "Claimed Website",
            "claim:website",
        )
    }

    fn x(input: &'a ClaimToolInput) -> Self {
        Self::new("x", input.x_url.as_deref(), "Claimed X", "claim:x")
    }

    fn new(
        link_type: &'static str,
        url: Option<&'a str>,
        label: &'static str,
        source: &'static str,
    ) -> Self {
        Self {
            link_type,
            url,
            label,
            source,
        }
    }
}

#[cfg(feature = "ssr")]
async fn insert_claim_official_link(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tool_id: Uuid,
    candidate: ClaimOfficialLinkCandidate<'_>,
) -> Result<(), FnError> {
    let Some(url) = normalized_claim_optional(candidate.url) else {
        return Ok(());
    };
    insert_candidate_official_link(
        tx,
        tool_id,
        candidate.link_type,
        url,
        candidate.label,
        candidate.source,
    )
    .await
}

#[cfg(feature = "ssr")]
async fn mark_claim_pending(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tool_id: Uuid,
) -> Result<(), FnError> {
    sqlx::query("UPDATE tools SET claim_state = 'claim_pending', updated_at = now() WHERE id = $1")
        .bind(tool_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| FnError::new(format!("failed to update claim state: {e}")))?;
    Ok(())
}
