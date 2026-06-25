//! Authentication — GitHub OAuth, email magic links, SIWX, JWT session cookies.

pub mod session;

pub mod siwx_client;

#[cfg(feature = "ssr")]
pub mod email;
#[cfg(feature = "ssr")]
pub mod guard;
#[cfg(feature = "ssr")]
pub mod onboarding;
#[cfg(feature = "ssr")]
pub mod pkce;
#[cfg(feature = "ssr")]
pub mod routes;
#[cfg(feature = "ssr")]
pub mod siwx;