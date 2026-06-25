//! OnchainAI Leptos SSR app shell.
//!
//! Defines the top-level router, the HTML shell, and placeholder page
//! components for all routes required by the website-core milestone.

use crate::components::top_nav::TopNav;
use crate::pages::admin::admin_page_shell;
use crate::pages::{
    AdminCategoriesPage, AdminCommentsPage, AdminCrawlerPage, AdminSettingsPage, AdminToolsPage,
    AdminUsersPage, CategoryPage, HomePage, LoginPage, OnboardingProfilePage, ToolDetailPage,
    ToolsListPage,
};
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Link, Meta, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{FlatRoutes, Route, Router},
    ParamSegment, StaticSegment,
};

/// Renders the application shell including `<html>`, `<head>`, and `<body>`.
///
/// This is invoked by `leptos_axum::render_app_to_stream_with_context` for SSR.
/// It pulls in the generated CSS, Google Fonts, and global meta tags, then
/// mounts the client-side router inside `<body>`.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <MetaTags/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <Stylesheet id="leptos" href="/pkg/onchainai.css"/>
                <Link rel="preconnect" href="https://fonts.googleapis.com"/>
                <Link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous"/>
                <Link
                    href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap"
                    rel="stylesheet"
                />
            </head>
            <body class="bg-white text-[#1A1A1A] antialiased">
                <App/>
            </body>
        </html>
    }
}

/// Top-level Leptos component: meta context + router.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="OnchainAI"/>
        <Meta name="description" content="Crypto tools, unified. Discover, install, and share crypto MCP, CLI, SDK, API, x402, RWA, and AI agent tools."/>
        <Meta name="color-scheme" content="light only"/>

        <Router>
            <main class="min-h-screen">
                <FlatRoutes fallback=|| view! { <NotFoundPage/> }.into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                    <Route path=StaticSegment("tools") view=ToolsListPage/>
                    <Route path=(StaticSegment("tools"), ParamSegment("slug")) view=ToolDetailPage/>
                    <Route path=(StaticSegment("categories"), ParamSegment("id")) view=CategoryPage/>
                    <Route path=StaticSegment("about") view=AboutPage/>
                    <Route path=StaticSegment("login") view=LoginPage/>
                    <Route path=(StaticSegment("onboarding"), StaticSegment("profile")) view=OnboardingProfilePage/>
                    <Route path=StaticSegment("admin") view=AdminHomePage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("tools")) view=AdminToolsPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("settings")) view=AdminSettingsPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("crawler")) view=AdminCrawlerPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("categories")) view=AdminCategoriesPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("users")) view=AdminUsersPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("comments")) view=AdminCommentsPage/>
                </FlatRoutes>
            </main>
        </Router>
    }
}

#[component]
fn AboutPage() -> impl IntoView {
    view! {
        <TopNav/>
        <div class="px-6 py-8 max-w-[720px] mx-auto">
            <h2 class="text-[20px] font-semibold mb-4">"About OnchainAI"</h2>
            <p class="text-[#6B6B6B] mb-8 leading-relaxed">
                "OnchainAI is a crypto tool directory for humans and agents. We crawl public registries and GitHub to surface MCP, CLI, SDK, and API tools in one searchable hub."
            </p>
            <section id="submit" class="scroll-mt-20 border-t border-[#E5E5E5] pt-8">
                <h2 class="text-[20px] font-semibold mb-3">"Submit a tool"</h2>
                <p class="text-[#6B6B6B] text-[14px] leading-relaxed mb-4">
                    "MVP does not include self-service registration yet. To list a tool, open a GitHub issue with the repo URL, install command, and supported chains."
                </p>
                <a
                    href="https://github.com/hoyeon4315-cpu/onchainai/issues/new"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="inline-flex items-center justify-center h-10 px-4 rounded-lg bg-[#E76F00] text-white text-[14px] font-medium no-underline hover:opacity-90"
                >
                    "Open GitHub issue →"
                </a>
            </section>
        </div>
    }
}

#[component]
fn AdminHomePage() -> impl IntoView {
    admin_page_shell(|| view! {
        <div class="px-6 py-8 max-w-[960px] mx-auto">
            <h2 class="text-[20px] font-semibold mb-4">"Admin"</h2>
            <p class="text-[#6B6B6B] text-[14px] mb-6">
                "Manage crawled tools, site settings, and moderation."
            </p>
            <nav class="flex flex-col gap-2 max-w-[320px]">
                <a
                    href="/admin/tools"
                    class="rounded-lg border border-[#E5E5E5] px-4 py-3 text-[14px] font-medium hover:bg-[#FAFAFA]"
                >
                    "Tool Management"
                    <span class="block text-[12px] text-[#6B6B6B] font-normal mt-0.5">
                        "Approve or reject pending tools"
                    </span>
                </a>
                <a
                    href="/admin/settings"
                    class="rounded-lg border border-[#E5E5E5] px-4 py-3 text-[14px] font-medium hover:bg-[#FAFAFA]"
                >
                    "Site Settings"
                    <span class="block text-[12px] text-[#6B6B6B] font-normal mt-0.5">
                        "Slogan, keywords, approval rules"
                    </span>
                </a>
                <a
                    href="/admin/crawler"
                    class="rounded-lg border border-[#E5E5E5] px-4 py-3 text-[14px] font-medium hover:bg-[#FAFAFA]"
                >
                    "Crawler Control"
                    <span class="block text-[12px] text-[#6B6B6B] font-normal mt-0.5">
                        "Source status and manual runs"
                    </span>
                </a>
                <a
                    href="/admin/categories"
                    class="rounded-lg border border-[#E5E5E5] px-4 py-3 text-[14px] font-medium hover:bg-[#FAFAFA]"
                >
                    "Categories"
                    <span class="block text-[12px] text-[#6B6B6B] font-normal mt-0.5">
                        "Add, edit, or remove function categories"
                    </span>
                </a>
                <a
                    href="/admin/users"
                    class="rounded-lg border border-[#E5E5E5] px-4 py-3 text-[14px] font-medium hover:bg-[#FAFAFA]"
                >
                    "Users"
                    <span class="block text-[12px] text-[#6B6B6B] font-normal mt-0.5">
                        "Ban, admin roles, account deletion"
                    </span>
                </a>
                <a
                    href="/admin/comments"
                    class="rounded-lg border border-[#E5E5E5] px-4 py-3 text-[14px] font-medium hover:bg-[#FAFAFA]"
                >
                    "Comments"
                    <span class="block text-[12px] text-[#6B6B6B] font-normal mt-0.5">
                        "Moderate spam and abusive content"
                    </span>
                </a>
            </nav>
        </div>
    })
}

#[component]
fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="px-6 py-12 max-w-[720px] mx-auto text-center">
            <h1 class="text-[28px] font-bold mb-4">"404"</h1>
            <p class="text-[#6B6B6B]">"Page not found."</p>
        </div>
    }
}
