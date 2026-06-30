use super::*;

/// Public trust view for tool detail — facts only, no raw scores.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolTrustView {
    pub tool: Tool,
    pub official_links: Vec<ToolOfficialLink>,
    pub trust_facts: Vec<TrustFact>,
}

/// Operator workbench bundle for a selected tool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminToolWorkbenchView {
    pub tool: Tool,
    pub official_links: Vec<ToolOfficialLink>,
    pub trust: TrustVerificationResult,
    pub timeline: Vec<ReviewEntry>,
    pub verdicts: Vec<OperatorVerdict>,
    pub official_promotion_allowed: bool,
}

/// Workbench summary counts for top promotion rail.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminWorkbenchSummary {
    pub cards: Vec<WorkbenchSummaryCard>,
}

/// Payload to verify an official link independently.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerifyOfficialLinkPayload {
    pub link_id: uuid::Uuid,
    pub verification_status: String,
    pub evidence_strength: String,
    pub official_badge_allowed: bool,
    pub verification_method: Option<String>,
    pub notes: Option<String>,
}

/// Public trust view — explainable facts without raw trust score.
#[server(GetToolTrustView, "/api")]
pub async fn get_tool_trust_view(slug: String) -> Result<ToolTrustView, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let tool = sqlx::query_as::<_, Tool>(APPROVED_TOOL_BY_SLUG_SQL)
        .bind(slug.trim())
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load tool trust view: {e}")))?;

    let official_links = list_public_official_links(&pool, tool.id).await?;
    let trust = verify_tool_trust(&tool, &official_links);

    Ok(ToolTrustView {
        tool: sanitize_tool_for_public_response(tool),
        official_links,
        trust_facts: trust.trust_facts,
    })
}

/// Admin workbench summary counts for top promotion rail.
#[server(GetAdminWorkbenchSummary, "/api")]
pub async fn get_admin_workbench_summary() -> Result<AdminWorkbenchSummary, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let counts = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        r#"
        SELECT
          COUNT(*) FILTER (
            WHERE approval_status = 'pending'
              AND last_reviewed_at IS NULL
              AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE claim_state = 'claim_pending' AND quarantined_at IS NULL
          )::bigint,
          COUNT(*) FILTER (
            WHERE approval_status = 'approved'
              AND status = 'community'
              AND claim_state = 'claimed'
              AND quarantined_at IS NULL
          )::bigint,
          (SELECT COUNT(*)::bigint
             FROM tools t
            WHERE t.approval_status = 'approved'
              AND t.quarantined_at IS NULL
              AND t.status IN ('verified', 'official')
              AND NOT EXISTS (
                SELECT 1 FROM featured_cards fc
                WHERE fc.tool_id = t.id AND fc.is_active = true
              ))
        FROM tools
        "#,
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to load workbench summary: {e}")))?;

    Ok(AdminWorkbenchSummary {
        cards: build_summary_cards(counts.0, counts.1, counts.2, counts.3),
    })
}

/// Admin workbench detail for one selected tool.
#[server(GetAdminToolWorkbench, "/api")]
pub async fn get_admin_tool_workbench(
    slug: String,
) -> Result<AdminToolWorkbenchView, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let tool = sqlx::query_as::<_, Tool>("SELECT * FROM tools WHERE slug = $1")
        .bind(slug.trim())
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to load tool workbench: {e}")))?;

    let (trust, official_links) = compute_tool_trust(&pool, &tool).await?;
    let review_timeline = load_tool_review_timeline(&pool, tool.id).await?;
    let promotion_ok = official_promotion_allowed(&tool, &official_links, &trust);

    Ok(AdminToolWorkbenchView {
        tool: redact_tool_for_admin(tool),
        official_links,
        trust,
        timeline: review_timeline.entries,
        verdicts: review_timeline.operator_verdicts,
        official_promotion_allowed: promotion_ok,
    })
}

/// Verify an official link independently (admin only).
#[server(VerifyToolOfficialLink, "/api")]
pub async fn verify_tool_official_link(
    payload: VerifyOfficialLinkPayload,
) -> Result<ToolOfficialLink, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let admin = require_admin(&parts, &pool, &config).await?;

    const STATUSES: &[&str] = &["candidate", "claimed", "verified", "rejected"];
    const STRENGTHS: &[&str] = &["weak", "medium", "strong"];
    if !STATUSES.contains(&payload.verification_status.as_str()) {
        return Err(ServerFnError::new("invalid verification status"));
    }
    if !STRENGTHS.contains(&payload.evidence_strength.as_str()) {
        return Err(ServerFnError::new("invalid evidence strength"));
    }
    if payload.official_badge_allowed && payload.verification_status != "verified" {
        return Err(ServerFnError::new(
            "official badge requires verified link status",
        ));
    }

    verify_official_link(
        &pool,
        VerifyOfficialLinkInput {
            link_id: payload.link_id,
            verification_status: payload.verification_status,
            evidence_strength: payload.evidence_strength,
            official_badge_allowed: payload.official_badge_allowed,
            verification_method: payload.verification_method,
            notes: payload.notes,
            operator_id: admin.id,
        },
    )
    .await
}
