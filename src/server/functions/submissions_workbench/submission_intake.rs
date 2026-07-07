use super::*;

/// Payload for public tool submission intake.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubmitToolInput {
    pub name: String,
    pub description: String,
    pub tool_type: String,
    pub function: String,
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
    pub npm_package: Option<String>,
    pub mcp_endpoint: Option<String>,
    pub install_command: Option<String>,
    pub chains_raw: String,
    pub category_suggestion: Option<String>,
    pub official_team_claim: bool,
    pub verification_note: Option<String>,
}

/// Validate optional https URL (localhost http allowed for dev).
pub(crate) fn validate_optional_https_url(value: Option<&str>) -> Result<(), &'static str> {
    let Some(raw) = value.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    if raw.len() > 500 {
        return Err("URL must be at most 500 characters");
    }
    if raw.starts_with("https://") {
        return Ok(());
    }
    if is_dev_loopback_http_url(raw) {
        return Ok(());
    }
    Err("URLs must use https:// (http://localhost allowed in dev)")
}

pub(crate) fn is_dev_loopback_http_url(raw: &str) -> bool {
    ["http://localhost", "http://127.0.0.1"]
        .iter()
        .any(|prefix| {
            raw.strip_prefix(prefix).is_some_and(|rest| {
                rest.is_empty()
                    || rest.starts_with(':')
                    || rest.starts_with('/')
                    || rest.starts_with('?')
                    || rest.starts_with('#')
            })
        })
}

/// Parse comma-separated chain list from submission form.
pub(crate) fn parse_submission_chains(raw: &str) -> Vec<String> {
    let parsed: Vec<String> = raw
        .split([',', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .take(20)
        .collect();
    crate::chains::canonicalize_chain_values(&parsed)
}

/// Intake gate: minimally plausible submission (relevance gates public approval, not intake).
pub(crate) fn submission_is_minimally_plausible(input: &SubmitToolInput) -> bool {
    let name = input.name.trim();
    let description = input.description.trim();
    if name.len() < 2 || name.len() > 100 {
        return false;
    }
    if description.len() < 20 || description.len() > 500 {
        return false;
    }
    if !SUBMIT_TOOL_TYPES.contains(&input.tool_type.trim()) {
        return false;
    }
    if !SUBMIT_FUNCTIONS.contains(&input.function.trim()) {
        return false;
    }
    let has_link = [
        input.repo_url.as_deref(),
        input.homepage.as_deref(),
        input.npm_package.as_deref(),
        input.mcp_endpoint.as_deref(),
    ]
    .into_iter()
    .any(|v| v.is_some_and(|s| !s.trim().is_empty()));
    has_link
}

/// Validate submission form input.
pub(crate) fn validate_submit_tool_input(input: &SubmitToolInput) -> Result<(), &'static str> {
    validate_submission_plausibility(input)?;
    validate_submission_urls(input)?;
    validate_optional_len(
        input.npm_package.as_deref(),
        200,
        "npm package must be at most 200 characters",
    )?;
    validate_install_command(input.install_command.as_deref())?;
    validate_optional_len(
        input.verification_note.as_deref(),
        1000,
        "verification note must be at most 1000 characters",
    )?;
    validate_optional_len(
        input.category_suggestion.as_deref(),
        100,
        "category suggestion must be at most 100 characters",
    )?;
    Ok(())
}

fn validate_submission_plausibility(input: &SubmitToolInput) -> Result<(), &'static str> {
    submission_is_minimally_plausible(input)
        .then_some(())
        .ok_or("submission must include name (2–100), description (20–500), valid type/function, and at least one link")
}

fn validate_submission_urls(input: &SubmitToolInput) -> Result<(), &'static str> {
    validate_optional_https_url(input.repo_url.as_deref())?;
    validate_optional_https_url(input.homepage.as_deref())?;
    validate_optional_https_url(input.mcp_endpoint.as_deref())
}

fn trimmed_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|s| !s.is_empty())
}

fn validate_optional_len(
    value: Option<&str>,
    max_len: usize,
    message: &'static str,
) -> Result<(), &'static str> {
    if trimmed_optional(value).is_some_and(|text| text.len() > max_len) {
        Err(message)
    } else {
        Ok(())
    }
}

fn validate_install_command(value: Option<&str>) -> Result<(), &'static str> {
    let Some(command) = trimmed_optional(value) else {
        return Ok(());
    };
    validate_install_command_len(command)?;
    validate_install_command_line(command)
}

fn validate_install_command_len(command: &str) -> Result<(), &'static str> {
    (command.len() <= 500)
        .then_some(())
        .ok_or("install command must be at most 500 characters")
}

fn validate_install_command_line(command: &str) -> Result<(), &'static str> {
    (!command.contains(['\n', '\r']))
        .then_some(())
        .ok_or("install command must be a single line")
}

/// Run relevance and install safety scanners on submission intake.
#[cfg(feature = "ssr")]
pub(crate) fn scan_submission(input: &SubmitToolInput) -> SubmissionScanResult {
    let chains = parse_submission_chains(&input.chains_raw);
    let relevance = assess_relevance(&RelevanceInput {
        name: input.name.trim(),
        description: Some(input.description.trim()),
        tool_type: input.tool_type.trim(),
        repo_url: input.repo_url.as_deref().map(str::trim),
        homepage: input.homepage.as_deref().map(str::trim),
        npm_package: input.npm_package.as_deref().map(str::trim),
        mcp_endpoint: input.mcp_endpoint.as_deref().map(str::trim),
        chains: &chains,
        source: "user_submission",
        keywords: &[],
    });
    let install = assess_install(
        input.install_command.as_deref().map(str::trim),
        input.npm_package.as_deref().map(str::trim),
    );
    SubmissionScanResult {
        crypto_relevance_score: relevance.score,
        relevance_status: relevance.status,
        install_risk_level: install.risk_level,
    }
}

fn submission_payload(
    input: SubmitToolInput,
    chains: Vec<String>,
    slug: String,
) -> ToolSubmissionPayload {
    ToolSubmissionPayload {
        name: input.name.trim().to_string(),
        description: input.description.trim().to_string(),
        tool_type: input.tool_type.trim().to_string(),
        function: input.function.trim().to_string(),
        repo_url: normalized_optional_string(input.repo_url.as_deref()),
        homepage: normalized_optional_string(input.homepage.as_deref()),
        npm_package: normalized_optional_string(input.npm_package.as_deref()),
        mcp_endpoint: normalized_optional_string(input.mcp_endpoint.as_deref()),
        install_command: normalized_optional_string(input.install_command.as_deref()),
        chains,
        category_suggestion: normalized_optional_string(input.category_suggestion.as_deref()),
        official_team_claim: input.official_team_claim,
        verification_note: normalized_optional_string(input.verification_note.as_deref()),
        slug,
        x402_endpoint_url: None,
    }
}

fn normalized_optional_string(value: Option<&str>) -> Option<String> {
    trimmed_optional(value).map(str::to_string)
}

#[cfg(feature = "ssr")]
async fn duplicate_submission_count(pool: &sqlx::PgPool, slug: &str) -> Result<i64, FnError> {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint FROM (
          SELECT slug FROM tools WHERE lower(slug) = lower($1)
          UNION ALL
          SELECT payload->>'slug' FROM tool_submissions
            WHERE status IN ('pending', 'needs_info')
              AND lower(payload->>'slug') = lower($1)
        ) d
        "#,
    )
    .bind(slug)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("duplicate check failed: {e}")))
}

fn reject_duplicate_submission(duplicate_count: i64) -> Result<(), FnError> {
    if duplicate_count > 0 {
        Err(FnError::new(
            "a similar tool is already listed or pending review",
        ))
    } else {
        Ok(())
    }
}
