//! OnchainAI Leptos SSR app shell.
//!
//! Defines the top-level router, the HTML shell, and placeholder page
//! components for all routes required by the website-core milestone.

use crate::components::admin_context::provide_current_user_resource;
use crate::components::site_shell::SiteShell;
use crate::components::top_nav::TopNav;
use crate::pages::{
    AdminCategoriesPage, AdminCommentsPage, AdminCrawlerPage, AdminDashboardPage,
    AdminFeaturedPage, AdminSettingsPage, AdminToolsPage, AdminUsersPage, CategoryPage,
    ComparePage, DashboardPage, HomePage, LoginPage, OnboardingProfilePage, SubmitPage,
    ToolDetailPage, ToolkitPage, ToolsListPage,
};
use crate::server::functions::get_current_user;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Link, Meta, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{FlatRoutes, Route, Router},
    ParamSegment, StaticSegment,
};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
struct HydrationAssetUrls {
    js: String,
    wasm: String,
}

fn file_mtime_secs(path: impl AsRef<Path>) -> Option<u64> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

fn versioned_url(path: String, version: Option<u64>) -> String {
    match version {
        Some(v) => format!("{path}?v={v}"),
        None => path,
    }
}

fn hydration_asset_urls(
    pkg_path: &str,
    output_name: &str,
    version: Option<u64>,
) -> HydrationAssetUrls {
    let pkg_path = pkg_path.trim_end_matches('/');
    let base = if pkg_path.starts_with('/') {
        pkg_path.to_string()
    } else {
        format!("/{pkg_path}")
    };

    HydrationAssetUrls {
        js: versioned_url(format!("{base}/{output_name}.js"), version),
        wasm: versioned_url(format!("{base}/{output_name}.wasm"), version),
    }
}

fn hydration_bundle_version(options: &LeptosOptions) -> Option<u64> {
    let site_root = Path::new(&*options.site_root);
    let pkg_dir = site_root.join(&*options.site_pkg_dir);
    let js = pkg_dir.join(format!("{}.js", options.output_name));
    let wasm = pkg_dir.join(format!("{}.wasm", options.output_name));

    [file_mtime_secs(js), file_mtime_secs(wasm)]
        .into_iter()
        .flatten()
        .max()
}

/// Hydration bootstrap with `?v=` cache busting. Keep `module_or_path` / `hydrate()`
/// aligned with Leptos `HydrationScripts` when upgrading cargo-leptos.
#[component]
fn CacheBustedHydrationScripts(options: LeptosOptions, version: Option<u64>) -> impl IntoView {
    let urls = hydration_asset_urls(&options.site_pkg_dir, &options.output_name, version);
    let js_href = urls.js.clone();
    let script = format!(
        "import({:?}).then((mod) => mod.default({{ module_or_path: {:?} }}).then(() => {{ mod.hydrate(); }})).catch((err) => {{ const message = err && (err.message || String(err)); if (!/aborted/i.test(message)) {{ console.error('OnchainAI hydration failed', err); }} }});",
        urls.js, urls.wasm
    );

    view! {
        <link rel="modulepreload" href=js_href/>
        <link
            rel="preload"
            href=urls.wasm
            r#as="fetch"
            r#type="application/wasm"
            crossorigin=""
        />
        <script type="module">{script}</script>
    }
}

/// Renders the application shell including `<html>`, `<head>`, and `<body>`.
///
/// This is invoked by `leptos_axum::render_app_to_stream_with_context` for SSR.
/// It pulls in the generated CSS, Google Fonts, and global meta tags, then
/// mounts the client-side router inside `<body>`.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    // Hydration requires /pkg/onchainai.js + WASM (onchainai_bg.wasm). When missing (404),
    // injecting HydrationScripts breaks the page — auto-detect bundle on disk.
    let bundle_exists = std::path::Path::new("target/site/pkg/onchainai.js").exists();
    let enable_hydration = match std::env::var("LEPTOS_HYDRATION") {
        Ok(v) if v == "0" || v.eq_ignore_ascii_case("false") => false,
        Ok(v) if v == "1" || v.eq_ignore_ascii_case("true") => bundle_exists,
        _ => bundle_exists,
    };
    let enable_reload = std::env::var("LEPTOS_ENV")
        .map(|v| v == "DEV")
        .unwrap_or(false);
    let options_reload = options.clone();
    let options_hydrate = options.clone();
    let hydration_version = hydration_bundle_version(&options);
    // Safari aggressively caches /pkg/*; bust CSS when the served stylesheet changes.
    let css_href = file_mtime_secs("style/output.css")
        .map(|d| format!("/pkg/onchainai.css?v={d}"))
        .unwrap_or_else(|| "/pkg/onchainai.css".to_string());

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <MetaTags/>
                {enable_reload.then(|| view! { <AutoReload options=options_reload.clone()/> })}
                {enable_hydration.then(|| view! {
                    <CacheBustedHydrationScripts
                        options=options_hydrate.clone()
                        version=hydration_version
                    />
                })}
                <Link rel="icon" href="/favicon.ico" sizes="any"/>
                <Link rel="icon" type_="image/png" href="/brand/onchainai-icon-32.png" sizes="32x32"/>
                <Link rel="apple-touch-icon" href="/brand/onchainai-icon-180.png" sizes="180x180"/>
                <Link rel="manifest" href="/site.webmanifest"/>
                <Stylesheet id="leptos" href=css_href/>
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
    let current_user = ArcOnceResource::new_blocking(async move { get_current_user().await });
    provide_current_user_resource(current_user.clone());

    view! {
        <Title text="OnchainAI"/>
        <Meta name="description" content="Crypto tools, unified. Discover, install, and share crypto MCP, CLI, SDK, API, x402, RWA, and AI agent tools."/>
        <Meta name="color-scheme" content="light only"/>

        <Router>
            <div class="site-app-shell min-h-screen flex flex-col">
                <TopNav/>
                <main class="site-page-body flex-1 min-h-0 flex flex-col">
                <FlatRoutes fallback=|| view! { <NotFoundPage/> }.into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                    <Route path=StaticSegment("tools") view=ToolsListPage/>
                    <Route path=StaticSegment("compare") view=ComparePage/>
                    <Route path=StaticSegment("dashboard") view=DashboardPage/>
                    <Route path=StaticSegment("toolkit") view=ToolkitPage/>
                    <Route path=(StaticSegment("tools"), ParamSegment("slug")) view=ToolDetailPage/>
                    <Route path=(StaticSegment("categories"), ParamSegment("id")) view=CategoryPage/>
                    <Route path=StaticSegment("about") view=AboutPage/>
                    <Route path=StaticSegment("submit") view=SubmitPage/>
                    <Route path=StaticSegment("login") view=LoginPage/>
                    <Route path=(StaticSegment("onboarding"), StaticSegment("profile")) view=OnboardingProfilePage/>
                    <Route path=StaticSegment("admin") view=AdminDashboardPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("tools")) view=AdminToolsPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("featured")) view=AdminFeaturedPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("settings")) view=AdminSettingsPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("crawler")) view=AdminCrawlerPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("categories")) view=AdminCategoriesPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("users")) view=AdminUsersPage/>
                    <Route path=(StaticSegment("admin"), StaticSegment("comments")) view=AdminCommentsPage/>
                </FlatRoutes>
                </main>
            </div>
        </Router>
    }
}

#[component]
fn AboutPage() -> impl IntoView {
    view! {
        <SiteShell>
        <div class="px-6 py-8 max-w-[720px]">
            <h2 class="text-[20px] font-semibold mb-4">"About OnchainAI"</h2>
            <p class="text-[#6B6B6B] mb-8 leading-relaxed">
                "OnchainAI is a crypto tool directory for humans and agents. We crawl public registries and GitHub to surface MCP, CLI, SDK, and API tools in one searchable hub."
            </p>
            <section id="submit" class="scroll-mt-20 border-t border-[#E5E5E5] pt-8">
                <h2 class="text-[20px] font-semibold mb-3">"Submit a tool"</h2>
                <p class="text-[#6B6B6B] text-[14px] leading-relaxed mb-4">
                    "Use the self-service submission flow to suggest a tool for operator review. Add the repo, install command, supported chains, and ownership context before it appears publicly."
                </p>
                <a
                    href="/submit"
                    class="inline-flex items-center justify-center h-10 px-4 rounded-lg bg-[#E76F00] text-white text-[14px] font-medium no-underline hover:bg-[#D96400]"
                >
                    "Go to Submit →"
                </a>
            </section>
        </div>
        </SiteShell>
    }
}

#[component]
fn NotFoundPage() -> impl IntoView {
    view! {
        <SiteShell>
        <div class="px-6 py-12 max-w-[720px] text-center">
            <h1 class="text-[28px] font-bold mb-4">"404"</h1>
            <p class="text-[#6B6B6B]">"Page not found."</p>
        </div>
        </SiteShell>
    }
}

#[cfg(test)]
mod tests {
    use super::{file_mtime_secs, hydration_asset_urls, hydration_bundle_version};
    use leptos::config::LeptosOptions;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static HYDRATION_TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn hydration_test_root() -> PathBuf {
        let id = HYDRATION_TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "onchainai-hydration-test-{}-{}",
            std::process::id(),
            id
        ))
    }

    fn test_leptos_options(site_root: &std::path::Path) -> LeptosOptions {
        LeptosOptions::builder()
            .output_name("onchainai")
            .site_root(site_root.to_string_lossy().into_owned())
            .site_pkg_dir("pkg")
            .build()
    }

    #[test]
    fn hydration_assets_include_cache_buster_query() {
        let urls = hydration_asset_urls("pkg", "onchainai", Some(12345));

        assert_eq!(urls.js, "/pkg/onchainai.js?v=12345");
        assert_eq!(urls.wasm, "/pkg/onchainai.wasm?v=12345");
    }

    #[test]
    fn hydration_bundle_version_none_when_assets_missing() {
        let root = hydration_test_root();
        let pkg = root.join("pkg");
        fs::create_dir_all(&pkg).expect("create pkg dir");

        let version = hydration_bundle_version(&test_leptos_options(&root));
        assert_eq!(version, None);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn hydration_bundle_version_uses_max_mtime_when_both_exist() {
        let root = hydration_test_root();
        let pkg = root.join("pkg");
        fs::create_dir_all(&pkg).expect("create pkg dir");

        let js_path = pkg.join("onchainai.js");
        let wasm_path = pkg.join("onchainai.wasm");
        fs::write(&js_path, "js").expect("write js");
        fs::write(&wasm_path, "wasm").expect("write wasm");

        let js_mtime = file_mtime_secs(&js_path).expect("js mtime");
        let wasm_mtime = file_mtime_secs(&wasm_path).expect("wasm mtime");
        assert!(wasm_mtime >= js_mtime);

        let version = hydration_bundle_version(&test_leptos_options(&root));
        assert_eq!(version, Some(wasm_mtime.max(js_mtime)));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn hydration_bundle_version_returns_single_asset_mtime() {
        let root = hydration_test_root();
        let pkg = root.join("pkg");
        fs::create_dir_all(&pkg).expect("create pkg dir");

        let js_path = pkg.join("onchainai.js");
        fs::write(&js_path, "js").expect("write js");

        let js_mtime = file_mtime_secs(&js_path).expect("js mtime");
        let version = hydration_bundle_version(&test_leptos_options(&root));
        assert_eq!(version, Some(js_mtime));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn official_brand_assets_exist() {
        for path in [
            "public/brand/onchainai-logo.svg",
            "public/brand/onchainai-logo.png",
            "public/brand/onchainai-icon-32.png",
            "public/favicon.ico",
        ] {
            assert!(std::path::Path::new(path).exists(), "missing {path}");
        }
    }
}
