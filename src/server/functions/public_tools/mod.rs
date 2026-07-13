//! Public tool catalog helpers (list, dashboard, toolkit, install guide).

use super::*;

mod list;
mod surface;

pub use list::*;
pub use surface::*;

#[cfg(all(test, feature = "ssr"))]
mod fetch_install_guide_tests;


/// Install-guide integration test helpers (direct fetch path, no Leptos RPC).
#[cfg(all(feature = "ssr", any(test, feature = "test-helpers")))]
pub mod server_fn_context_tests;

