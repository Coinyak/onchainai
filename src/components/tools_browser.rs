//! Shared tools browser — sidebar filters, sort bar, HOT list, preview overlays.
//! Used by Home (`/`) and ToolsList (`/tools`) per UI_UX_DESIGN §2.

use crate::components::{
    bottom_sheet::BottomSheet, chain_strip::ChainStrip, empty_state::EmptyState,
    error_state::ErrorState, preview_panel::PreviewPanel, search_bar::ToolbarSearch,
    sidebar::Sidebar, skeleton::ToolListSkeleton, tool_card::ToolCard, top_nav::SidebarBrand,
};
use crate::filter_query::{build_tool_filters, describe_active_filters, ActiveFiltersSummary};
use crate::models::{Category, Tool};
use crate::server::functions::{
    count_tools, get_categories, get_chain_counts, get_tool_by_slug, get_tool_comment_counts,
    list_tools_v1, ToolFilters, ToolListRequest, MAX_LIST_TOOLS_LIMIT,
};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use std::collections::HashMap;

const TOOL_PAGE_SIZE: u32 = 50;
const MAX_VISIBLE_TOOLS: u32 = MAX_LIST_TOOLS_LIMIT as u32;

#[derive(Clone, PartialEq, Eq)]
pub enum BrowserBase {
    Home,
    Tools,
    Category(String),
}

impl BrowserBase {
    pub fn path(&self) -> String {
        match self {
            BrowserBase::Home => "/".into(),
            BrowserBase::Tools => "/tools".into(),
            BrowserBase::Category(id) => format!("/categories/{id}"),
        }
    }

    /// Function filter from route (category pages) or query string elsewhere.
    pub fn function_from_query(&self, from_query: Option<String>) -> Option<String> {
        match self {
            BrowserBase::Category(id) => Some(id.clone()),
            _ => from_query,
        }
    }
}

/// Category page link preserving non-function query params (chain, sort, q, …).
pub fn category_href(cat_id: &str, query_base: &str) -> String {
    let params: Vec<&str> = query_base
        .split('?')
        .nth(1)
        .unwrap_or("")
        .split('&')
        .filter(|p| !p.is_empty() && !p.starts_with("function="))
        .collect();
    if params.is_empty() {
        format!("/categories/{cat_id}")
    } else {
        format!("/categories/{cat_id}?{}", params.join("&"))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_query_base(
    base: &BrowserBase,
    function: Option<String>,
    asset_class: Option<String>,
    actor: Option<String>,
    tool_type: Option<String>,
    status: Option<String>,
    chain: Option<String>,
    sort: String,
    search_q: Option<String>,
    selected: Option<String>,
    page: u32,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !matches!(base, BrowserBase::Category(_)) {
        if let Some(v) = function {
            parts.push(format!("function={}", urlencoding::encode(&v)));
        }
    }
    if let Some(v) = asset_class {
        parts.push(format!("asset_class={}", urlencoding::encode(&v)));
    }
    if let Some(v) = actor {
        parts.push(format!("actor={}", urlencoding::encode(&v)));
    }
    if let Some(v) = tool_type {
        parts.push(format!("type={}", urlencoding::encode(&v)));
    }
    if let Some(v) = status {
        parts.push(format!("status={}", urlencoding::encode(&v)));
    }
    if let Some(v) = chain {
        parts.push(format!("chain={}", urlencoding::encode(&v)));
    }
    if sort != "hot" {
        parts.push(format!("sort={}", urlencoding::encode(&sort)));
    }
    if let Some(v) = search_q.filter(|s| !s.is_empty()) {
        parts.push(format!("q={}", urlencoding::encode(v.as_str())));
    }
    if let Some(v) = selected {
        parts.push(format!("selected={}", urlencoding::encode(&v)));
    }
    if page > 1 {
        parts.push(format!("page={page}"));
    }
    if parts.is_empty() {
        base.path()
    } else {
        format!("{}?{}", base.path(), parts.join("&"))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_filter_navigation_base(
    base: &BrowserBase,
    function: Option<String>,
    asset_class: Option<String>,
    actor: Option<String>,
    tool_type: Option<String>,
    status: Option<String>,
    chain: Option<String>,
    sort: String,
    search_q: Option<String>,
) -> String {
    build_query_base(
        base,
        function,
        asset_class,
        actor,
        tool_type,
        status,
        chain,
        sort,
        search_q,
        None,
        1,
    )
}

fn parse_page_param(raw: Option<String>) -> u32 {
    raw.and_then(|s| s.parse::<u32>().ok())
        .filter(|page| *page > 0)
        .unwrap_or(1)
}

/// Parse `page` from a raw query string (`?page=2&sort=hot` or `page=2`).
pub fn parse_page_from_query_string(query: &str) -> Option<u32> {
    let trimmed = query.trim_start_matches('?');
    trimmed.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if key == "page" {
            value.parse::<u32>().ok().filter(|page| *page > 0)
        } else {
            None
        }
    })
}

#[cfg(feature = "hydrate")]
fn page_from_browser_url() -> Option<u32> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    parse_page_from_query_string(&search)
}

pub fn visible_limit_for_page(page: u32) -> i64 {
    let page = page.max(1);
    let limit = page.saturating_mul(TOOL_PAGE_SIZE).min(MAX_VISIBLE_TOOLS);
    i64::from(limit)
}

/// Whether the load-more control should appear (respects total, UI cap, and page growth).
pub fn should_show_load_more(shown: usize, total: i64, page: u32) -> bool {
    let shown = shown as i64;
    let total = total.max(0);
    let max_visible = i64::from(MAX_VISIBLE_TOOLS);
    if shown >= total {
        return false;
    }
    if shown >= max_visible {
        return false;
    }
    let current_limit = visible_limit_for_page(page);
    let next_limit = visible_limit_for_page(page.saturating_add(1));
    if current_limit >= max_visible && next_limit == current_limit {
        return false;
    }
    true
}

pub fn with_selected(base_path: &BrowserBase, base: &str, slug: &str) -> String {
    let root = base_path.path();
    if base == root || base.is_empty() {
        format!("{root}?selected={slug}")
    } else if base.contains('?') {
        format!("{base}&selected={slug}")
    } else {
        format!("{base}?selected={slug}")
    }
}

/// Sort toolbar link — rebuilds query via `build_query_base` (no duplicate `sort=` params).
#[allow(clippy::too_many_arguments)]
pub fn build_sort_href(
    base: &BrowserBase,
    function: Option<String>,
    asset_class: Option<String>,
    actor: Option<String>,
    tool_type: Option<String>,
    status: Option<String>,
    chain: Option<String>,
    sort: &str,
    search_q: Option<String>,
) -> String {
    build_query_base(
        base,
        function,
        asset_class,
        actor,
        tool_type,
        status,
        chain,
        sort.to_string(),
        search_q,
        None,
        1,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_load_more_href(
    base: &BrowserBase,
    function: Option<String>,
    asset_class: Option<String>,
    actor: Option<String>,
    tool_type: Option<String>,
    status: Option<String>,
    chain: Option<String>,
    sort: String,
    search_q: Option<String>,
    _selected: Option<String>,
    page: u32,
) -> String {
    build_query_base(
        base,
        function,
        asset_class,
        actor,
        tool_type,
        status,
        chain,
        sort,
        search_q,
        None,
        page.saturating_add(1),
    )
}

pub fn without_selected(base_path: &BrowserBase, base: &str) -> String {
    let root = base_path.path();
    let trimmed = base.trim_start_matches('?');
    let query = if base.starts_with(&root) {
        base.strip_prefix(&root)
            .unwrap_or("")
            .trim_start_matches('?')
    } else {
        trimmed
    };
    let parts: Vec<&str> = query
        .split('&')
        .filter(|p| !p.is_empty() && !p.starts_with("selected="))
        .collect();
    if parts.is_empty() {
        root.to_string()
    } else {
        format!("{root}?{}", parts.join("&"))
    }
}

fn comment_count_for_slug(comment_counts: &HashMap<String, i64>, slug: &str) -> i64 {
    comment_counts.get(slug).copied().unwrap_or(0)
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct BrowserData {
    categories: Vec<(Category, i64)>,
    chains: Vec<(String, i64)>,
    total: i64,
    tools: Vec<Tool>,
    comment_counts: HashMap<String, i64>,
    preview_tool: Option<Tool>,
}

async fn load_browser_data(
    sort: String,
    filters: ToolFilters,
    search_q: Option<String>,
    selected: Option<String>,
    page: u32,
) -> Result<BrowserData, ServerFnError> {
    // Round 1: these queries are independent, so run them concurrently — one
    // network round-trip to the DB instead of five (the DB is remote; latency,
    // not execution time, dominates page load).
    let preview_fut = async {
        match selected.filter(|s| !s.is_empty()) {
            Some(s) => get_tool_by_slug(s).await.ok(),
            None => None,
        }
    };
    let (categories, chains, total, tools, preview_tool) = futures::join!(
        get_categories(),
        get_chain_counts(12),
        count_tools(filters.clone()),
        list_tools_v1(ToolListRequest {
            sort,
            offset: 0,
            limit: visible_limit_for_page(page),
            filters,
            query: search_q,
        }),
        preview_fut,
    );
    let categories = categories?;
    let chains = chains?;
    let total = total?;
    let tools = tools?;

    // Round 2: comment counts need the resolved tool slugs.
    let slugs = tools.iter().map(|t| t.slug.clone()).collect();
    let comment_counts: HashMap<String, i64> =
        get_tool_comment_counts(slugs).await?.into_iter().collect();
    Ok(BrowserData {
        categories,
        chains,
        total,
        tools,
        comment_counts,
        preview_tool,
    })
}

#[component]
pub fn ToolsBrowser(
    base: BrowserBase,
    #[prop(optional)] show_toolbar_search: bool,
    #[prop(optional)] children: Option<ChildrenFn>,
) -> impl IntoView {
    let base = StoredValue::new(base);
    let query = use_query_map();
    let function = Memo::new(move |_| {
        base.get_value()
            .function_from_query(query.with(|q| q.get("function").map(|s| s.to_string())))
    });
    let asset_class =
        Memo::new(move |_| query.with(|q| q.get("asset_class").map(|s| s.to_string())));
    let actor = Memo::new(move |_| query.with(|q| q.get("actor").map(|s| s.to_string())));
    let tool_type = Memo::new(move |_| query.with(|q| q.get("type").map(|s| s.to_string())));
    let status = Memo::new(move |_| query.with(|q| q.get("status").map(|s| s.to_string())));
    let chain = Memo::new(move |_| query.with(|q| q.get("chain").map(|s| s.to_string())));
    let sort = Memo::new(move |_| {
        query
            .with(|q| q.get("sort").map(|s| s.to_string()))
            .unwrap_or_else(|| "hot".into())
    });
    let search_q = Memo::new(move |_| query.with(|q| q.get("q").map(|s| s.to_string())));
    let selected = Memo::new(move |_| query.with(|q| q.get("selected").map(|s| s.to_string())));
    let page_number = Memo::new(move |_| {
        let from_router = query.with(|q| q.get("page").map(|s| s.to_string()));
        #[cfg(feature = "hydrate")]
        if from_router.is_none() {
            if let Some(page) = page_from_browser_url() {
                return page;
            }
        }
        parse_page_param(from_router)
    });

    let query_base = Memo::new(move |_| {
        build_query_base(
            &base.get_value(),
            function.get(),
            asset_class.get(),
            actor.get(),
            tool_type.get(),
            status.get(),
            chain.get(),
            sort.get(),
            search_q.get(),
            selected.get(),
            page_number.get(),
        )
    });
    let filter_query_base = Memo::new(move |_| {
        build_filter_navigation_base(
            &base.get_value(),
            function.get(),
            asset_class.get(),
            actor.get(),
            tool_type.get(),
            status.get(),
            chain.get(),
            sort.get(),
            search_q.get(),
        )
    });

    let filters = Memo::new(move |_| {
        build_tool_filters(
            function.get(),
            asset_class.get(),
            actor.get(),
            tool_type.get(),
            status.get(),
            chain.get(),
        )
    });

    let retry_tick = RwSignal::new(0u32);
    let page_deps = Memo::new(move |_| {
        (
            sort.get(),
            filters.get(),
            search_q.get(),
            selected.get(),
            page_number.get(),
            retry_tick.get(),
        )
    });

    let page = Resource::new_blocking(
        move || page_deps.get(),
        |(sort, filters, search_q, selected, page_number, _)| async move {
            load_browser_data(sort, filters, search_q, selected, page_number).await
        },
    );

    // Keep the last successful payload visible while the resource refetches so full-page
    // `?page=N` navigation does not flash an empty/partial list during hydration.
    let cached_browser_data = RwSignal::<Option<BrowserData>>::new(None);
    Effect::new(move || {
        if let Some(Ok(data)) = page.get() {
            cached_browser_data.set(Some(data));
        }
    });

    let sort_hot = Memo::new(move |_| {
        build_sort_href(
            &base.get_value(),
            function.get(),
            asset_class.get(),
            actor.get(),
            tool_type.get(),
            status.get(),
            chain.get(),
            "hot",
            search_q.get(),
        )
    });
    let sort_new = Memo::new(move |_| {
        build_sort_href(
            &base.get_value(),
            function.get(),
            asset_class.get(),
            actor.get(),
            tool_type.get(),
            status.get(),
            chain.get(),
            "new",
            search_q.get(),
        )
    });
    let sort_comments = Memo::new(move |_| {
        build_sort_href(
            &base.get_value(),
            function.get(),
            asset_class.get(),
            actor.get(),
            tool_type.get(),
            status.get(),
            chain.get(),
            "comments",
            search_q.get(),
        )
    });

    let children_fallback = children.clone();

    view! {
        <div class="tools-layout" data-tools-browser="">
            <Suspense fallback=move || view! {
                <aside class="tools-sidebar site-sidebar-chrome">
                    <SidebarBrand/>
                    <p class="sidebar-empty">"Loading filters…"</p>
                </aside>
                <div class="tools-main">
                    {children_fallback.as_ref().map(|content| view! { <div class="tools-prepend">{content()}</div> })}
                    <ToolListSkeleton count=6/>
                </div>
            }>
                {move || {
                    let resolved = match page.get() {
                        Some(Ok(data)) => Some(data),
                        Some(Err(_)) => None,
                        None => cached_browser_data.get(),
                    };
                    match (page.get(), resolved) {
                        (Some(Err(e)), _) => view! {
                            <aside class="tools-sidebar site-sidebar-chrome">
                                <SidebarBrand/>
                                <p class="sidebar-empty">"Loading filters…"</p>
                            </aside>
                            <div class="tools-main">
                                {children.as_ref().map(|content| view! { <div class="tools-prepend">{content()}</div> })}
                                <ErrorState
                                    message=e.to_string()
                                    on_retry=move || retry_tick.update(|n| *n = n.wrapping_add(1))
                                />
                            </div>
                        }.into_any(),
                        (_, Some(data)) => {
                            let qb = query_base.get();
                            let filter_qb = filter_query_base.get();
                            let browser_base = base.get_value();
                            view! {
                                <Sidebar
                                    base=browser_base.clone()
                                    categories=data.categories.clone()
                                    query_base=filter_qb.clone()
                                    active_function=function.get()
                                    active_asset_class=asset_class.get()
                                    active_actor=actor.get()
                                    active_type=tool_type.get()
                                    active_status=status.get()
                                    default_function_open=matches!(browser_base, BrowserBase::Tools)
                                />
                                <div class="tools-main">
                                    {children.as_ref().map(|content| view! { <div class="tools-prepend">{content()}</div> })}
                                    <ChainStrip
                                        base=browser_base.clone()
                                        query_base=filter_qb.clone()
                                        active_chain=chain.get()
                                        chain_counts=data.chains.clone()
                                    />
                                    <div class="tools-toolbar sticky-toolbar">
                                        {if show_toolbar_search {
                                            view! { <ToolbarSearch base=browser_base.clone() initial_q=search_q.get().unwrap_or_default()/> }.into_any()
                                        } else {
                                            ().into_any()
                                        }}
                                        <div class="toolbar-sort">
                                            <a href=move || sort_hot.get() class=move || if sort.get() == "hot" { "sort-link active" } else { "sort-link" }>"HOT ↓"</a>
                                            <a href=move || sort_new.get() class=move || if sort.get() == "new" { "sort-link active" } else { "sort-link" }>"New"</a>
                                            <a href=move || sort_comments.get() class=move || if sort.get() == "comments" { "sort-link active" } else { "sort-link" }>"Comments"</a>
                                        </div>
                                        <span class="tool-count">{data.total}" tools"</span>
                                    </div>
                                    {if data.tools.is_empty() {
                                        let filter_summary = ActiveFiltersSummary::from_query(
                                            function.get(),
                                            asset_class.get(),
                                            actor.get(),
                                            tool_type.get(),
                                            status.get(),
                                            chain.get(),
                                            search_q.get(),
                                            Some(sort.get()),
                                        );
                                        let function_labels: std::collections::HashMap<String, String> =
                                            data.categories.iter().map(|(c, _)| (c.id.clone(), c.label.clone())).collect();
                                        let filter_lines = describe_active_filters(&filter_summary, &function_labels);
                                        let clear_href = if filter_summary.has_active_filters() {
                                            browser_base.path()
                                        } else {
                                            String::new()
                                        };
                                        view! {
                                            <EmptyState filter_lines=filter_lines clear_href=clear_href/>
                                        }.into_any()
                                    } else {
                                        let comment_counts = data.comment_counts.clone();
                                        view! {
                                            <div class="tool-list">
                                                {data.tools.clone().into_iter().map(|t| {
                                                    let slug = t.slug.clone();
                                                    let preview = with_selected(&browser_base, &qb, &slug);
                                                    let sel = selected.get().map(|s| s == slug).unwrap_or(false);
                                                    let count = comment_count_for_slug(&comment_counts, &slug);
                                                    view! { <ToolCard tool=t preview_href=preview is_selected=sel comment_count=count/> }
                                                }).collect_view()}
                                            </div>
                                            {if should_show_load_more(data.tools.len(), data.total, page_number.get()) {
                                                let next_href = build_load_more_href(
                                                    &browser_base,
                                                    function.get(),
                                                    asset_class.get(),
                                                    actor.get(),
                                                    tool_type.get(),
                                                    status.get(),
                                                    chain.get(),
                                                    sort.get(),
                                                    search_q.get(),
                                                    selected.get(),
                                                    page_number.get(),
                                                );
                                                view! {
                                                    <div class="load-more-row">
                                                        <a href=next_href class="load-more-btn">
                                                            "Load more"
                                                        </a>
                                                        <span class="load-more-count">
                                                            "Showing "{data.tools.len()}" of "{data.total}
                                                        </span>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                ().into_any()
                                            }}
                                        }.into_any()
                                    }}
                                    {data.preview_tool.clone().map(|tool| {
                                        let close = without_selected(&browser_base, &qb);
                                        let full = format!("/tools/{}", tool.slug);
                                        view! {
                                            <div class="preview-desktop">
                                                <PreviewPanel tool=tool.clone() close_href=close.clone() full_page_href=full.clone()/>
                                            </div>
                                            <div class="preview-mobile">
                                                <BottomSheet tool=tool close_href=close full_page_href=full/>
                                            </div>
                                        }
                                    })}
                                </div>
                            }.into_any()
                        }
                        (None, None) => view! {
                            <aside class="tools-sidebar site-sidebar-chrome">
                                <SidebarBrand/>
                                <p class="sidebar-empty">"Loading filters…"</p>
                            </aside>
                            <div class="tools-main">
                                {children.as_ref().map(|content| view! { <div class="tools-prepend">{content()}</div> })}
                                <ToolListSkeleton count=6/>
                            </div>
                        }.into_any(),
                        (Some(Ok(_)), None) => view! {
                            <aside class="tools-sidebar site-sidebar-chrome">
                                <SidebarBrand/>
                                <p class="sidebar-empty">"Loading filters…"</p>
                            </aside>
                            <div class="tools-main">
                                {children.as_ref().map(|content| view! { <div class="tools-prepend">{content()}</div> })}
                                <ToolListSkeleton count=6/>
                            </div>
                        }.into_any(),
                    }
                }}
            </Suspense>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_path_omits_function_param_but_keeps_chain() {
        let q = build_query_base(
            &BrowserBase::Category("bridge".into()),
            Some("bridge".into()),
            None,
            None,
            None,
            None,
            Some("ethereum,solana".into()),
            "hot".into(),
            None,
            None,
            1,
        );
        assert_eq!(q, "/categories/bridge?chain=ethereum%2Csolana");
    }

    #[test]
    fn category_href_preserves_chain_sort() {
        let href = category_href(
            "swap",
            "/categories/bridge?chain=ethereum&sort=new&function=bridge",
        );
        assert_eq!(href, "/categories/swap?chain=ethereum&sort=new");
    }

    #[test]
    fn home_query_includes_multi_filters_and_selected() {
        let q = build_query_base(
            &BrowserBase::Home,
            Some("bridge,swap".into()),
            None,
            None,
            Some("mcp".into()),
            None,
            None,
            "hot".into(),
            None,
            Some("zapper".into()),
            1,
        );
        assert!(q.starts_with("/?"));
        assert!(q.contains("function=bridge%2Cswap") || q.contains("function=bridge,swap"));
        assert!(q.contains("type=mcp"));
        assert!(q.contains("selected=zapper"));
    }

    #[test]
    fn query_base_keeps_page_only_after_first_page() {
        let first = build_query_base(
            &BrowserBase::Tools,
            None,
            None,
            None,
            None,
            None,
            Some("base".into()),
            "hot".into(),
            Some("wallet".into()),
            None,
            1,
        );
        assert_eq!(first, "/tools?chain=base&q=wallet");

        let second = build_query_base(
            &BrowserBase::Tools,
            None,
            None,
            None,
            None,
            None,
            Some("base".into()),
            "hot".into(),
            Some("wallet".into()),
            None,
            2,
        );
        assert_eq!(second, "/tools?chain=base&q=wallet&page=2");
    }

    #[test]
    fn load_more_href_increments_page_and_drops_preview_selection() {
        let href = build_load_more_href(
            &BrowserBase::Tools,
            Some("bridge".into()),
            None,
            None,
            Some("mcp".into()),
            None,
            Some("base".into()),
            "comments".into(),
            Some("agent".into()),
            Some("selected-tool".into()),
            2,
        );
        assert_eq!(
            href,
            "/tools?function=bridge&type=mcp&chain=base&sort=comments&q=agent&page=3"
        );
    }

    #[test]
    fn filter_navigation_base_omits_pagination_and_preview_selection() {
        let href = build_filter_navigation_base(
            &BrowserBase::Tools,
            Some("bridge".into()),
            None,
            None,
            Some("mcp".into()),
            None,
            Some("base".into()),
            "comments".into(),
            Some("agent".into()),
        );
        assert_eq!(
            href,
            "/tools?function=bridge&type=mcp&chain=base&sort=comments&q=agent"
        );
        assert!(!href.contains("selected="));
        assert!(!href.contains("page="));
    }

    #[test]
    fn visible_limit_for_page_is_bounded() {
        assert_eq!(visible_limit_for_page(0), 50);
        assert_eq!(visible_limit_for_page(1), 50);
        assert_eq!(visible_limit_for_page(2), 100);
        assert_eq!(visible_limit_for_page(3), 150);
        assert_eq!(visible_limit_for_page(99), 500);
    }

    #[test]
    fn parse_page_from_query_string_reads_page_param() {
        assert_eq!(parse_page_from_query_string("?page=2&sort=hot"), Some(2));
        assert_eq!(parse_page_from_query_string("page=3"), Some(3));
        assert_eq!(parse_page_from_query_string("?sort=hot"), None);
        assert_eq!(parse_page_from_query_string("?page=0"), None);
    }

    #[test]
    fn load_more_page_two_requests_cumulative_limit() {
        assert_eq!(visible_limit_for_page(2), 100);
        assert!(should_show_load_more(100, 500, 2));
    }

    #[test]
    fn tools_path_clear_is_root() {
        assert_eq!(without_selected(&BrowserBase::Tools, "/tools"), "/tools");
        assert_eq!(
            without_selected(&BrowserBase::Tools, "/tools?function=swap&selected=foo"),
            "/tools?function=swap"
        );
    }

    #[test]
    fn sort_href_rebuilds_without_duplicate_sort_param() {
        let filters = (
            Some("bridge,swap".into()),
            None,
            None,
            Some("mcp".into()),
            None,
            None,
        );
        let from_new = build_sort_href(
            &BrowserBase::Tools,
            filters.0.clone(),
            filters.1.clone(),
            filters.2.clone(),
            filters.3.clone(),
            filters.4.clone(),
            filters.5.clone(),
            "new",
            None,
        );
        assert_eq!(from_new.matches("sort=").count(), 1);
        assert!(
            from_new.contains("function=bridge%2Cswap")
                || from_new.contains("function=bridge,swap")
        );
        assert!(from_new.contains("sort=new"));

        let to_hot = build_sort_href(
            &BrowserBase::Tools,
            filters.0,
            filters.1,
            filters.2,
            filters.3,
            filters.4,
            filters.5,
            "hot",
            None,
        );
        assert!(!to_hot.contains("sort="));
        assert!(
            to_hot.contains("function=bridge%2Cswap") || to_hot.contains("function=bridge,swap")
        );
        assert!(to_hot.contains("type=mcp"));
    }

    #[test]
    fn sort_href_omits_selected_preview() {
        let href = build_sort_href(
            &BrowserBase::Tools,
            None,
            None,
            None,
            None,
            None,
            None,
            "new",
            None,
        );
        assert!(!href.contains("selected="));
    }

    #[test]
    fn should_show_load_more_hides_when_all_shown() {
        assert!(!should_show_load_more(50, 50, 1));
    }

    #[test]
    fn should_show_load_more_hides_at_visible_cap() {
        assert!(!should_show_load_more(500, 1000, 10));
    }

    #[test]
    fn should_show_load_more_shows_when_more_available() {
        assert!(should_show_load_more(50, 200, 1));
        assert!(should_show_load_more(450, 1000, 9));
    }

    #[test]
    fn should_show_load_more_hides_when_page_cannot_grow() {
        assert!(!should_show_load_more(480, 1000, 10));
    }

    #[test]
    fn should_show_load_more_shows_from_empty_first_page() {
        assert!(should_show_load_more(0, 100, 1));
    }

    #[test]
    fn should_show_load_more_hides_when_total_non_positive() {
        assert!(!should_show_load_more(0, 0, 1));
        assert!(!should_show_load_more(0, -5, 1));
    }

    #[test]
    fn should_show_load_more_treats_page_zero_like_first_page() {
        assert!(should_show_load_more(50, 200, 0));
        assert!(!should_show_load_more(50, 50, 0));
    }

    #[test]
    fn should_show_load_more_shows_one_below_cap_with_room_to_grow() {
        assert!(should_show_load_more(499, 1000, 9));
    }

    #[test]
    fn missing_comment_count_defaults_to_zero() {
        let mut counts = HashMap::new();
        counts.insert("aave".to_string(), 3);
        assert_eq!(comment_count_for_slug(&counts, "aave"), 3);
        assert_eq!(comment_count_for_slug(&counts, "uniswap"), 0);
    }
}
