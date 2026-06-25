//! Page components — one module per route.

pub mod admin;
pub mod category;
pub mod home;
pub mod login;
pub mod onboarding;
pub mod tool_detail;
pub mod tools_list;

pub use admin::{
    AdminCategoriesPage, AdminCommentsPage, AdminCrawlerPage, AdminSettingsPage,
    AdminToolsPage, AdminUsersPage,
};
pub use category::CategoryPage;
pub use home::HomePage;
pub use login::LoginPage;
pub use onboarding::OnboardingProfilePage;
pub use tool_detail::ToolDetailPage;
pub use tools_list::ToolsListPage;