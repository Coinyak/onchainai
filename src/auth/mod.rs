//! Authentication — GitHub/Google OAuth, email magic links, SIWX, JWT session cookies.

pub mod session;

#[cfg(feature = "ssr")]
pub mod email;
#[cfg(feature = "ssr")]
pub mod google;
#[cfg(feature = "ssr")]
pub mod guard;
#[cfg(feature = "ssr")]
pub mod oauth_state;
#[cfg(feature = "ssr")]
pub mod onboarding;
#[cfg(feature = "ssr")]
pub mod pkce;
#[cfg(feature = "ssr")]
pub mod routes;
#[cfg(feature = "ssr")]
pub mod siwx;
