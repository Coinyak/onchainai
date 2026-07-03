//! Axum JSON API (`/api/v2/*`) — mirrors Leptos server functions for frontend separation.

pub mod admin_review;
pub mod admin_users_comments;
pub mod auth;
pub mod blueprints;
pub mod comments_bookmarks;
pub mod crawler_admin;
pub mod error;
pub mod public_tools;
pub mod reports_claims;
pub mod site_settings;
pub mod submissions;
pub mod taxonomy_featured;
pub mod user;
pub mod workbench;

use axum::Router;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(public_tools::router(state.clone()))
        .merge(user::router(state.clone()))
        .merge(comments_bookmarks::router(state.clone()))
        .merge(blueprints::router(state.clone()))
        .merge(admin_review::router(state.clone()))
        .merge(taxonomy_featured::router(state.clone()))
        .merge(admin_users_comments::router(state.clone()))
        .merge(crawler_admin::router(state.clone()))
        .merge(site_settings::router(state.clone()))
        .merge(submissions::router(state.clone()))
        .merge(workbench::router(state.clone()))
        .merge(reports_claims::router(state))
}
