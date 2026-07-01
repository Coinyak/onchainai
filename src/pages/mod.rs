//! Page components — one module per route.

pub mod admin;
pub mod category;
pub mod compare;
pub mod dashboard;
pub mod home;
pub mod login;
pub mod onboarding;
pub mod submit;
pub mod tool_detail;
pub mod toolkit;
pub mod tools_list;

pub use admin::{
    AdminCategoriesPage, AdminCommentsPage, AdminCrawlerPage, AdminDashboardPage,
    AdminFeaturedPage, AdminSettingsPage, AdminToolsPage, AdminUsersPage,
};
pub use category::CategoryPage;
pub use compare::ComparePage;
pub use dashboard::DashboardPage;
pub use home::HomePage;
pub use login::LoginPage;
pub use onboarding::OnboardingProfilePage;
pub use submit::SubmitPage;
pub use tool_detail::ToolDetailPage;
pub use toolkit::ToolkitPage;
pub use tools_list::ToolsListPage;
