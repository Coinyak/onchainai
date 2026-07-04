//! Shared server business logic — types, validation, SQL-backed fetch helpers.
//!
//! Axum `/api/v2/*` handlers import from these modules; Leptos `#[server]` RPC
//! wrappers were removed in Phase 3 (Next.js frontend calls JSON API instead).

#![allow(dead_code, unused_imports)]

use crate::models::tool::{sanitize_tool_for_public_response, sanitize_tools_for_public_response};
use crate::models::{
    sanitize_site_settings_for_public, Category, Comment, FeaturedCard, OperatorVerdict,
    ReviewEntry, ReviewRun, SiteSettings, Source, Tool, ToolClaimRequest, ToolOfficialLink,
    ToolReport, ToolSubmission, ToolSubmissionPayload, TOOL_REPORT_REASONS,
};
use crate::trust_verification::{
    official_promotion_allowed, verify_tool_trust, TrustFact, TrustVerificationResult,
};
use std::collections::HashMap;
use uuid::Uuid;

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
use crate::server::fn_error::FnError;
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
    IS_BOOKMARKED_SQL, LIST_APPROVED_TOOLS_SQL, PUBLIC_TOOL_WHERE, RECENT_APPROVED_TOOLS_SQL,
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
use std::fmt::Write as _;

mod site_settings;
pub use site_settings::*;

mod public_tools;
#[cfg(all(feature = "ssr", any(test, feature = "test-helpers")))]
pub use public_tools::server_fn_context_tests;
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
