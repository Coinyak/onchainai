//! Leptos server functions — public API used by pages and components.
// Goal harness deliverable AC2/AC5 — partial ToolFilters deserialize-safe
// harness-round-11: 2026-06-27T11:00:00Z-functions
//!
//! These functions are auto-registered by the Leptos runtime and are
//! available to both server-rendered and hydrated components.

// Server fns are invoked via Leptos macro registration; silence lib-build dead_code noise.
#![allow(dead_code, unused_imports)]

use crate::auth::session::{optional_session_result, SessionUser};
use crate::models::tool::{sanitize_tool_for_public_response, sanitize_tools_for_public_response};
use crate::models::{
    sanitize_site_settings_for_public, Category, Comment, FeaturedCard, OperatorVerdict,
    ReviewEntry, ReviewRun, SiteSettings, Source, Tool, ToolClaimRequest, ToolOfficialLink,
    ToolReport, ToolSubmission, ToolSubmissionPayload, TOOL_REPORT_REASONS,
};
use crate::trust_verification::{
    official_promotion_allowed, verify_tool_trust, TrustFact, TrustVerificationResult,
};
use leptos::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::auth::guard::{require_admin, require_user};
#[cfg(feature = "ssr")]
use crate::auth::session::session_from_parts;
#[cfg(feature = "ssr")]
use crate::config::Config;
#[cfg(feature = "ssr")]
use crate::crawler::normalizer::base_slug;
#[cfg(feature = "ssr")]
use crate::crawler::relevance::{assess_relevance, RelevanceInput};
#[cfg(feature = "ssr")]
use crate::crawler::{self, default_source_registry_url};
#[cfg(feature = "ssr")]
use crate::install_safety::assess_install;
#[cfg(feature = "ssr")]
use crate::server::operator_review_transition::{
    plan_operator_review, validate_demote_official_gate, validate_demote_verified_gate,
    validate_review_approval_gate, OperatorReviewGate,
};
#[cfg(feature = "ssr")]
use crate::server::queries::{
    list_tools_order_clause, DashboardCountAxis, APPROVED_TOOLS_BY_SLUGS_SQL,
    APPROVED_TOOL_BY_SLUG_SQL, APPROVED_TOOL_ID_BY_SLUG_SQL, BOOKMARKED_SLUGS_SQL,
    CATEGORIES_WITH_COUNTS_SQL, CHAIN_COUNTS_SQL, COUNT_APPROVED_TOOLS_SQL,
    DASHBOARD_FUNCTION_COUNTS_SQL, DASHBOARD_METRICS_SQL, DASHBOARD_X402_TOOLS_SQL,
    IS_BOOKMARKED_SQL, LIST_APPROVED_TOOLS_SQL, RECENT_APPROVED_TOOLS_SQL,
    SEARCH_APPROVED_TOOLS_SQL, TOOL_COMMENTS_NEW_SORT_SQL, TOOL_COMMENTS_TOP_SORT_SQL,
    TOOL_COMMENT_COUNTS_BY_SLUGS_SQL, TOOL_COMMENT_COUNT_BY_SLUG_SQL, USER_TOOLKIT_SQL,
};
#[cfg(feature = "ssr")]
use crate::server::rate_limit::{check_user_rate_limit, UserRateLimitAction};
#[cfg(feature = "ssr")]
use crate::server::review_persistence::{
    apply_operator_review_in_tx, compute_tool_trust, insert_candidate_official_link,
    list_official_links, list_public_official_links, load_tool_review_timeline,
    validate_mark_official_gate, verify_official_link, LegacyReviewEventInput,
    VerifyOfficialLinkInput,
};
use crate::server::secret_redaction::redact_secrets;
#[cfg(feature = "ssr")]
use crate::server::secret_redaction::redact_tool_for_admin;
use crate::workbench::{build_summary_cards, WorkbenchSummaryCard};
#[cfg(feature = "ssr")]
use axum::http::request::Parts;
use std::fmt::Write as _;

#[cfg(feature = "ssr")]
fn request_context() -> Result<(Parts, sqlx::PgPool, Config), ServerFnError> {
    let parts = use_context::<Parts>()
        .ok_or_else(|| ServerFnError::new("request context not available"))?;
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;
    let config =
        use_context::<Config>().ok_or_else(|| ServerFnError::new("configuration not available"))?;
    Ok((parts, pool, config))
}

/// Backfill the non-HttpOnly session hint for users who logged in before PR #10.
#[cfg(feature = "ssr")]
fn append_session_hint_if_missing(parts: &Parts, config: &Config) {
    use crate::auth::session::{
        cookie_secure_for_domain, session_hint_present, set_session_hint_cookie,
    };
    use axum::http::header;
    use leptos::prelude::*;
    use leptos_axum::ResponseOptions;

    let cookie_header = parts
        .headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if session_hint_present(cookie_header) {
        return;
    }

    let secure = cookie_secure_for_domain(&config.siwx_domain);
    let hint = set_session_hint_cookie(config.siwx_session_ttl, secure);
    let Ok(value) = hint.parse::<axum::http::HeaderValue>() else {
        tracing::warn!("session hint cookie rejected by header parser");
        return;
    };
    let Some(opts) = use_context::<ResponseOptions>() else {
        tracing::warn!("session hint backfill skipped: ResponseOptions missing");
        return;
    };
    opts.append_header(header::SET_COOKIE, value);
}

/// Current signed-in user, if any (from session cookie).
#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<Option<SessionUser>, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    let user = optional_session_result(
        session_from_parts(&parts, &pool, &config.jwt_secret, &config.jwt_issuer()).await,
    )?;
    #[cfg(feature = "ssr")]
    if user.is_some() {
        append_session_hint_if_missing(&parts, &config);
    }
    Ok(user)
}

/// Admin gate — returns the admin session or a generic "not found" error.
#[server(CheckAdminAccess, "/api")]
pub async fn check_admin_access() -> Result<SessionUser, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config)
        .await
        .map_err(ServerFnError::new)
}

mod site_settings;
pub use site_settings::*;

mod public_tools;
pub use public_tools::*;

mod admin_review;
pub use admin_review::*;

mod crawler_admin;
pub use crawler_admin::*;

mod comments_bookmarks;
pub use comments_bookmarks::*;

mod taxonomy_featured;
pub use taxonomy_featured::*;

mod admin_users_comments;
pub use admin_users_comments::*;

mod submissions_workbench;
pub use submissions_workbench::*;

mod function_tests;
pub use function_tests::*;
