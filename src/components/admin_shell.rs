//! Admin layout — left sidebar with brand + operator navigation.

use leptos::prelude::*;
use leptos_router::hooks::use_location;

struct AdminNavItem {
    href: &'static str,
    label: &'static str,
}

const ADMIN_NAV: &[AdminNavItem] = &[
    AdminNavItem {
        href: "/admin",
        label: "Dashboard",
    },
    AdminNavItem {
        href: "/admin/tools",
        label: "Tools",
    },
    AdminNavItem {
        href: "/admin/comments",
        label: "Comments",
    },
    AdminNavItem {
        href: "/admin/users",
        label: "Users",
    },
    AdminNavItem {
        href: "/admin/categories",
        label: "Categories",
    },
    AdminNavItem {
        href: "/admin/crawler",
        label: "Crawler",
    },
    AdminNavItem {
        href: "/admin/featured",
        label: "Featured",
    },
    AdminNavItem {
        href: "/admin/settings",
        label: "Settings",
    },
];

fn nav_link_class(active: bool) -> &'static str {
    if active {
        "sidebar-link active"
    } else {
        "sidebar-link"
    }
}

fn is_admin_nav_active(pathname: &str, href: &str) -> bool {
    let path = pathname
        .split(['?', '#'])
        .next()
        .unwrap_or(pathname)
        .trim_end_matches('/');
    if href == "/admin" {
        path == "/admin"
    } else {
        path == href || path.starts_with(&format!("{href}/"))
    }
}

#[component]
pub fn AdminShell(children: Children) -> impl IntoView {
    let location = use_location();

    view! {
        <div class="site-layout">
            <aside class="tools-sidebar site-sidebar-chrome">
                <nav class="admin-nav" aria-label="Admin navigation">
                    <div class="admin-nav-heading">"Admin"</div>
                    <ul class="sidebar-list admin-nav-list">
                        {ADMIN_NAV.iter().map(|item| {
                            let href = item.href;
                            let label = item.label;
                            view! {
                                <li>
                                    <a
                                        href=href
                                        class=move || {
                                            let path = location.pathname.get();
                                            nav_link_class(is_admin_nav_active(&path, href))
                                        }
                                    >
                                        <span class="sidebar-title-text">{label}</span>
                                    </a>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </nav>
            </aside>
            <main class="site-main">
                {children()}
            </main>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::is_admin_nav_active;

    #[test]
    fn dashboard_active_only_on_exact_admin_root() {
        assert!(is_admin_nav_active("/admin", "/admin"));
        assert!(!is_admin_nav_active("/admin/tools", "/admin"));
    }

    #[test]
    fn section_active_on_subpaths() {
        assert!(is_admin_nav_active("/admin/tools", "/admin/tools"));
        assert!(is_admin_nav_active(
            "/admin/tools?queue=new",
            "/admin/tools"
        ));
    }
}
