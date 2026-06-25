//! Page components — one module per route.

pub mod admin;
pub mod category;
pub mod home;
pub mod tool_detail;
pub mod tools_list;

pub use admin::AdminToolsPage;
pub use category::CategoryPage;
pub use home::HomePage;
pub use tool_detail::ToolDetailPage;
pub use tools_list::ToolsListPage;