//! Shared tools browser — sidebar filters, sort bar, HOT list, preview overlays.
//! Used by Home (`/`) and ToolsList (`/tools`) per UI_UX_DESIGN §2.

use crate::components::{
    bottom_sheet::BottomSheet, chain_strip::ChainStrip, empty_state::EmptyState,
    error_state::ErrorState, preview_panel::PreviewPanel, search_bar::ToolbarSearch,
    sidebar::Sidebar, skeleton::ToolListSkeleton, tool_card::ToolCard,
};
use crate::discovery::{empty_state_suggestions, EmptyRecoverySummary};
use crate::filter_query::{build_tool_filters, describe_active_filters, ActiveFiltersSummary};
use crate::models::tool::parse_page_value;

use crate::server::functions::{
    get_tool_by_slug, load_browser_data, BrowserDataPayload, LoadBrowserDataRequest, ToolFilters,
    MAX_LIST_TOOLS_LIMIT,
};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use std::collections::HashMap;

const TOOL_PAGE_SIZE: u32 = 50;
const MAX_VISIBLE_TOOLS: u32 = MAX_LIST_TOOLS_LIMIT as u32;
pub const MAX_BROWSER_PAGE: u32 = MAX_VISIBLE_TOOLS / TOOL_PAGE_SIZE;

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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrowserQueryParams {
    pub function: Option<String>,
    pub asset_class: Option<String>,
    pub actor: Option<String>,
    pub tool_type: Option<String>,
    pub status: Option<String>,
    pub pricing: Option<String>,
    pub install_risk: Option<String>,
    pub chain: Option<String>,
    pub sort: String,
    pub search_q: Option<String>,
    pub selected: Option<String>,
    pub page: u32,
}

impl BrowserQueryParams {
    pub fn for_filter_navigation(&self) -> Self {
        Self {
            selected: None,
            page: 1,
            ..self.clone()
        }
    }

    pub fn for_sort(&self, sort: &str) -> Self {
        Self {
            sort: sort.to_string(),
            selected: None,
            page: 1,
            ..self.clone()
        }
    }

    pub fn for_next_page(&self) -> Self {
        Self {
            selected: None,
            page: self.page.saturating_add(1),
            ..self.clone()
        }
    }
}

pub fn build_query_base(base: &BrowserBase, params: &BrowserQueryParams) -> String {
    let mut parts: Vec<String> = Vec::new();
    append_optional_param(
        &mut parts,
        "function",
        category_function_filter(base, params),
    );
    append_optional_param(&mut parts, "asset_class", params.asset_class.as_deref());
    append_optional_param(&mut parts, "actor", params.actor.as_deref());
    append_optional_param(&mut parts, "type", params.tool_type.as_deref());
    append_optional_param(&mut parts, "status", params.status.as_deref());
    append_optional_param(&mut parts, "pricing", params.pricing.as_deref());
    append_optional_param(&mut parts, "install_risk", params.install_risk.as_deref());
    append_optional_param(&mut parts, "chain", params.chain.as_deref());
    append_sort_param(&mut parts, &params.sort);
    append_search_param(&mut parts, params.search_q.as_deref());
    append_optional_param(&mut parts, "selected", params.selected.as_deref());
    append_page_param(&mut parts, params.page);
    query_path(base, parts)
}

pub fn build_filter_navigation_base(base: &BrowserBase, params: &BrowserQueryParams) -> String {
    build_query_base(base, &params.for_filter_navigation())
}

fn category_function_filter<'a>(
    base: &BrowserBase,
    params: &'a BrowserQueryParams,
) -> Option<&'a str> {
    (!matches!(base, BrowserBase::Category(_)))
        .then_some(params.function.as_deref())
        .flatten()
}

fn append_optional_param(parts: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        parts.push(format!("{key}={}", urlencoding::encode(value)));
    }
}

fn append_sort_param(parts: &mut Vec<String>, sort: &str) {
    if sort != "hot" {
        append_optional_param(parts, "sort", Some(sort));
    }
}

fn append_search_param(parts: &mut Vec<String>, search_q: Option<&str>) {
    append_optional_param(parts, "q", search_q.filter(|value| !value.is_empty()));
}

fn append_page_param(parts: &mut Vec<String>, page: u32) {
    if page > 1 {
        parts.push(format!("page={page}"));
    }
}

fn query_path(base: &BrowserBase, parts: Vec<String>) -> String {
    if parts.is_empty() {
        base.path()
    } else {
        format!("{}?{}", base.path(), parts.join("&"))
    }
}

pub fn clamp_browser_page(page: u32) -> u32 {
    page.clamp(1, MAX_BROWSER_PAGE)
}

fn parse_page_param(raw: Option<String>) -> u32 {
    clamp_browser_page(
        raw.and_then(|s| parse_page_value(&decode_page_query_value(&s)))
            .unwrap_or(1),
    )
}

fn decode_page_query_value(value: &str) -> String {
    urlencoding::decode(value)
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| value.to_string())
}

/// Parse `page` from a raw query string (`?page=2&sort=hot` or `page=2`).
pub fn parse_page_from_query_string(query: &str) -> Option<u32> {
    let trimmed = query.trim_start_matches('?');
    trimmed.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if key == "page" {
            parse_page_value(&decode_page_query_value(value)).map(clamp_browser_page)
        } else {
            None
        }
    })
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
pub fn build_sort_href(base: &BrowserBase, params: &BrowserQueryParams, sort: &str) -> String {
    build_query_base(base, &params.for_sort(sort))
}

pub fn build_load_more_href(base: &BrowserBase, params: &BrowserQueryParams) -> String {
    build_query_base(base, &params.for_next_page())
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

/// Cache identity for browser list data (excludes `selected` and `retry_tick`).
///
/// `selected` is intentionally excluded so changing the preview selection does
/// not invalidate the list payload. The preview tool is loaded by a separate
/// Resource keyed on `selected` only, so the list stays stable and the scroll
/// position is preserved.
#[derive(Clone, PartialEq)]
pub struct BrowserCacheKey {
    pub sort: String,
    pub filters: ToolFilters,
    pub search_q: Option<String>,
    pub page: u32,
}

#[derive(Clone)]
struct TaggedBrowserPayload {
    deps: BrowserCacheKey,
    data: BrowserDataPayload,
}

/// Choose list payload for render: hydrated resource wins; cache only while pending.
/// Invariant: Leptos `Resource` invalidates `Some(Ok)` when `page_cache_key` changes.
fn browser_data_for_render(
    page_state: &Option<Result<BrowserDataPayload, ServerFnError>>,
    cache_key: &BrowserCacheKey,
    cached: Option<&TaggedBrowserPayload>,
) -> Option<BrowserDataPayload> {
    match page_state.as_ref() {
        Some(Ok(data)) => Some(data.clone()),
        None => cached
            .filter(|tagged| tagged.deps == *cache_key)
            .map(|tagged| tagged.data.clone()),
        Some(Err(_)) => None,
    }
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
    let pricing = Memo::new(move |_| query.with(|q| q.get("pricing").map(|s| s.to_string())));
    let install_risk =
        Memo::new(move |_| query.with(|q| q.get("install_risk").map(|s| s.to_string())));
    let chain = Memo::new(move |_| query.with(|q| q.get("chain").map(|s| s.to_string())));
    let sort = Memo::new(move |_| {
        query
            .with(|q| q.get("sort").map(|s| s.to_string()))
            .unwrap_or_else(|| "hot".into())
    });
    let search_q = Memo::new(move |_| query.with(|q| q.get("q").map(|s| s.to_string())));
    let selected = Memo::new(move |_| query.with(|q| q.get("selected").map(|s| s.to_string())));
    // Router query only — do not read window.location (SSR/hydration divergence).
    let page_number =
        Memo::new(move |_| parse_page_param(query.with(|q| q.get("page").map(|s| s.to_string()))));

    let browser_query_params = Memo::new(move |_| BrowserQueryParams {
        function: function.get(),
        asset_class: asset_class.get(),
        actor: actor.get(),
        tool_type: tool_type.get(),
        status: status.get(),
        pricing: pricing.get(),
        install_risk: install_risk.get(),
        chain: chain.get(),
        sort: sort.get(),
        search_q: search_q.get(),
        selected: selected.get(),
        page: page_number.get(),
    });
    let query_base =
        Memo::new(move |_| build_query_base(&base.get_value(), &browser_query_params.get()));
    let filter_revision = Memo::new(move |_| {
        format!(
            "f={}|ac={}|a={}|t={}|st={}|p={}|r={}|c={}",
            function.get().unwrap_or_default(),
            asset_class.get().unwrap_or_default(),
            actor.get().unwrap_or_default(),
            tool_type.get().unwrap_or_default(),
            status.get().unwrap_or_default(),
            pricing.get().unwrap_or_default(),
            install_risk.get().unwrap_or_default(),
            chain.get().unwrap_or_default(),
        )
    });
    let filter_query_base = Memo::new(move |_| {
        build_filter_navigation_base(&base.get_value(), &browser_query_params.get())
    });

    let filters = Memo::new(move |_| {
        build_tool_filters(
            function.get(),
            asset_class.get(),
            actor.get(),
            tool_type.get(),
            status.get(),
            pricing.get(),
            install_risk.get(),
            chain.get(),
        )
    });

    let retry_tick = RwSignal::new(0u32);
    let page_cache_key = Memo::new(move |_| BrowserCacheKey {
        sort: sort.get(),
        filters: filters.get(),
        search_q: search_q.get(),
        page: page_number.get(),
    });
    let page_fetch_deps = Memo::new(move |_| (page_cache_key.get(), retry_tick.get()));

    // Stale-while-revalidate cache for in-flight refetches (not serialized on hydrate).
    let cached_browser_data = RwSignal::<Option<TaggedBrowserPayload>>::new(None);

    let page: Resource<Result<BrowserDataPayload, ServerFnError>> = Resource::new_blocking(
        move || page_fetch_deps.get(),
        move |(cache_key, _retry_tick)| async move {
            let data = load_browser_data(LoadBrowserDataRequest {
                sort: cache_key.sort.clone(),
                filters: cache_key.filters.clone(),
                search_q: cache_key.search_q.clone(),
                // `selected` is excluded from the list fetch so preview
                // selection changes do not invalidate the list payload.
                selected: None,
                page: cache_key.page,
            })
            .await?;
            if page_cache_key.get_untracked() == cache_key {
                cached_browser_data.set(Some(TaggedBrowserPayload {
                    deps: cache_key,
                    data: data.clone(),
                }));
            }
            Ok(data)
        },
    );

    // Separate lightweight resource for the preview tool, keyed on `selected`
    // only. This decouples preview loading from the list so changing the
    // selected tool does not cause the list to collapse into skeletons or
    // the scroll position to jump.
    let preview_tool = Resource::new_blocking(
        move || selected.get(),
        move |slug| async move {
            match slug {
                Some(s) if !s.is_empty() => get_tool_by_slug(s).await.map(Some),
                _ => Ok(None),
            }
        },
    );

    Effect::new(move || {
        let current = page_cache_key.get();
        if let Some(tagged) = cached_browser_data.get() {
            if tagged.deps != current {
                cached_browser_data.set(None);
            }
        }
    });

    let sort_hot =
        Memo::new(move |_| build_sort_href(&base.get_value(), &browser_query_params.get(), "hot"));
    let sort_new =
        Memo::new(move |_| build_sort_href(&base.get_value(), &browser_query_params.get(), "new"));
    let sort_comments = Memo::new(move |_| {
        build_sort_href(&base.get_value(), &browser_query_params.get(), "comments")
    });

    let children_fallback = children.clone();

    view! {
        <div class="tools-layout" data-tools-browser="">
            <Suspense fallback=move || {
                let browser_base = base.get_value();
                view! {
                    <Sidebar
                        base=browser_base.clone()
                        categories=vec![]
                        query_base=filter_query_base.get()
                        filter_revision=filter_revision
                        active_function=function.get()
                        active_asset_class=asset_class.get()
                        active_actor=actor.get()
                        active_type=tool_type.get()
                        active_status=status.get()
                        active_pricing=pricing.get()
                        active_install_risk=install_risk.get()
                        default_function_open=matches!(browser_base, BrowserBase::Tools)
                    />
                    <div class="tools-main">
                        {children_fallback.as_ref().map(|content| view! { <div class="tools-prepend">{content()}</div> })}
                        <ToolListSkeleton count=6/>
                    </div>
                }
            }>
                {move || {
                    let cache_key = page_cache_key.get();
                    let page_state = page.get();
                    let browser_data = browser_data_for_render(
                        &page_state,
                        &cache_key,
                        cached_browser_data.get().as_ref(),
                    );
                    match (page_state, browser_data) {
                        (Some(Err(e)), _) => {
                            let browser_base = base.get_value();
                            view! {
                                <Sidebar
                                    base=browser_base.clone()
                                    categories=vec![]
                                    query_base=filter_query_base.get()
                                    filter_revision=filter_revision
                                    active_function=function.get()
                                    active_asset_class=asset_class.get()
                                    active_actor=actor.get()
                                    active_type=tool_type.get()
                                    active_status=status.get()
                                    active_pricing=pricing.get()
                                    active_install_risk=install_risk.get()
                                    default_function_open=matches!(browser_base, BrowserBase::Tools)
                                />
                                <div class="tools-main">
                                    {children.as_ref().map(|content| view! { <div class="tools-prepend">{content()}</div> })}
                                    <ErrorState
                                        message=e.to_string()
                                        on_retry=move || retry_tick.update(|n| *n = n.wrapping_add(1))
                                    />
                                </div>
                            }.into_any()
                        }
                        (_, Some(data)) => {
                            let qb = query_base.get();
                            let filter_qb = filter_query_base.get();
                            let browser_base = base.get_value();
                            view! {
                                <Sidebar
                                    base=browser_base.clone()
                                    categories=data.categories.clone()
                                    query_base=filter_qb.clone()
                                    filter_revision=filter_revision
                                    active_function=function.get()
                                    active_asset_class=asset_class.get()
                                    active_actor=actor.get()
                                    active_type=tool_type.get()
                                    active_status=status.get()
                                    active_pricing=pricing.get()
                                    active_install_risk=install_risk.get()
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
                                            pricing.get(),
                                            install_risk.get(),
                                            chain.get(),
                                            search_q.get(),
                                            Some(sort.get()),
                                        );
                                        let function_labels: std::collections::HashMap<String, String> =
                                            data.categories.iter().map(|(c, _)| (c.id.clone(), c.label.clone())).collect();
                                        let filter_lines = describe_active_filters(&filter_summary, &function_labels);
                                        let suggestions = empty_state_suggestions(
                                            &browser_base.path(),
                                            &EmptyRecoverySummary {
                                                function: filter_summary.function.clone(),
                                                asset_class: filter_summary.asset_class.clone(),
                                                actor: filter_summary.actor.clone(),
                                                tool_type: filter_summary.tool_type.clone(),
                                                status: filter_summary.status.clone(),
                                                pricing: filter_summary.pricing.clone(),
                                                install_risk: filter_summary.install_risk.clone(),
                                                chain: filter_summary.chain.clone(),
                                                search: filter_summary.search.clone(),
                                                sort: filter_summary.sort.clone(),
                                            },
                                        );
                                        let clear_href = if filter_summary.has_active_filters() {
                                            browser_base.path()
                                        } else {
                                            String::new()
                                        };
                                        view! {
                                            <EmptyState filter_lines=filter_lines suggestions=suggestions clear_href=clear_href/>
                                        }.into_any()
                                    } else {
                                        let comment_counts = data.comment_counts.clone();
                                        let tools = data.tools.clone();
                                        let tools_len = tools.len();
                                        let browser_base_list = browser_base.clone();
                                        let qb_list = qb.clone();
                                        view! {
                                            <div class="tool-list">
                                                <For
                                                    each=move || tools.clone()
                                                    key=|tool| tool.slug.clone()
                                                    children=move |t| {
                                                        let slug = t.slug.clone();
                                                        let preview = with_selected(&browser_base_list, &qb_list, &slug);
                                                        let sel = selected.get().map(|s| s == slug).unwrap_or(false);
                                                        let count = comment_count_for_slug(&comment_counts, &slug);
                                                        view! {
                                                            <ToolCard
                                                                tool=t
                                                                preview_href=preview
                                                                is_selected=sel
                                                                comment_count=count
                                                            />
                                                        }
                                                    }
                                                />
                                            </div>
                                            {if should_show_load_more(tools_len, data.total, page_number.get()) {
                                                let next_href = build_load_more_href(
                                                    &browser_base,
                                                    &browser_query_params.get(),
                                                );
                                                view! {
                                                    <div class="load-more-row">
                                                        <a href=next_href class="load-more-btn">
                                                            "Load more"
                                                        </a>
                                                        <span class="load-more-count">
                                                            "Showing "{tools_len}" of "{data.total}
                                                        </span>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                ().into_any()
                                            }}
                                        }.into_any()
                                    }}
                                    {match preview_tool.get() {
                                        Some(Ok(Some(tool))) => {
                                            let close = without_selected(&browser_base, &qb);
                                            let full = format!("/tools/{}", tool.slug);
                                            view! {
                                                <div class="preview-desktop">
                                                    <PreviewPanel tool=tool.clone() close_href=close.clone() full_page_href=full.clone()/>
                                                </div>
                                                <div class="preview-mobile">
                                                    <BottomSheet tool=tool close_href=close full_page_href=full/>
                                                </div>
                                            }.into_any()
                                        }
                                        Some(Ok(None)) => ().into_any(),
                                        Some(Err(_)) => ().into_any(),
                                        None => ().into_any(),
                                    }}
                                </div>
                            }.into_any()
                        }
                        _ => {
                            let browser_base = base.get_value();
                            view! {
                                <Sidebar
                                    base=browser_base.clone()
                                    categories=vec![]
                                    query_base=filter_query_base.get()
                                    filter_revision=filter_revision
                                    active_function=function.get()
                                    active_asset_class=asset_class.get()
                                    active_actor=actor.get()
                                    active_type=tool_type.get()
                                    active_status=status.get()
                                    active_pricing=pricing.get()
                                    active_install_risk=install_risk.get()
                                    default_function_open=matches!(browser_base, BrowserBase::Tools)
                                />
                                <div class="tools-main">
                                    {children.as_ref().map(|content| view! { <div class="tools-prepend">{content()}</div> })}
                                    <ToolListSkeleton count=6/>
                                </div>
                            }.into_any()
                        }
                    }
                }}
            </Suspense>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query_params() -> BrowserQueryParams {
        BrowserQueryParams {
            sort: "hot".into(),
            page: 1,
            ..BrowserQueryParams::default()
        }
    }

    fn assert_contains_all(text: &str, fragments: &[&str]) {
        for fragment in fragments {
            assert!(text.contains(fragment), "missing {fragment} in {text}");
        }
    }

    fn assert_page_cases(cases: &[(&str, Option<u32>)]) {
        for (query, expected) in cases {
            assert_eq!(parse_page_from_query_string(query), *expected);
        }
    }

    fn assert_parse_page_param_cases(cases: &[(Option<&str>, u32)]) {
        for (raw, expected) in cases {
            assert_eq!(parse_page_param(raw.map(str::to_string)), *expected);
        }
    }

    fn assert_clamp_cases(cases: &[(u32, u32)]) {
        for (raw, expected) in cases {
            assert_eq!(clamp_browser_page(*raw), *expected);
        }
    }

    #[test]
    fn category_path_omits_function_param_but_keeps_chain() {
        let q = build_query_base(
            &BrowserBase::Category("bridge".into()),
            &BrowserQueryParams {
                function: Some("bridge".into()),
                chain: Some("ethereum,solana".into()),
                ..query_params()
            },
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
            &BrowserQueryParams {
                function: Some("bridge,swap".into()),
                tool_type: Some("mcp".into()),
                selected: Some("zapper".into()),
                ..query_params()
            },
        );
        assert!(q.starts_with("/?"));
        assert!(q.contains("function=bridge%2Cswap") || q.contains("function=bridge,swap"));
        assert_contains_all(&q, &["type=mcp", "selected=zapper"]);
    }

    #[test]
    fn query_base_keeps_page_only_after_first_page() {
        let first = build_query_base(
            &BrowserBase::Tools,
            &BrowserQueryParams {
                chain: Some("base".into()),
                search_q: Some("wallet".into()),
                ..query_params()
            },
        );
        assert_eq!(first, "/tools?chain=base&q=wallet");

        let second = build_query_base(
            &BrowserBase::Tools,
            &BrowserQueryParams {
                chain: Some("base".into()),
                search_q: Some("wallet".into()),
                page: 2,
                ..query_params()
            },
        );
        assert_eq!(second, "/tools?chain=base&q=wallet&page=2");
    }

    #[test]
    fn load_more_href_increments_page_and_drops_preview_selection() {
        let href = build_load_more_href(
            &BrowserBase::Tools,
            &BrowserQueryParams {
                function: Some("bridge".into()),
                tool_type: Some("mcp".into()),
                pricing: Some("x402".into()),
                chain: Some("base".into()),
                sort: "comments".into(),
                search_q: Some("agent".into()),
                selected: Some("selected-tool".into()),
                page: 2,
                ..query_params()
            },
        );
        assert_eq!(
            href,
            "/tools?function=bridge&type=mcp&pricing=x402&chain=base&sort=comments&q=agent&page=3"
        );
    }

    #[test]
    fn filter_navigation_base_omits_pagination_and_preview_selection() {
        let href = build_filter_navigation_base(
            &BrowserBase::Tools,
            &BrowserQueryParams {
                function: Some("bridge".into()),
                tool_type: Some("mcp".into()),
                chain: Some("base".into()),
                sort: "comments".into(),
                search_q: Some("agent".into()),
                ..query_params()
            },
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
        let cases = [(0, 50), (1, 50), (2, 100), (3, 150), (99, 500)];
        for (page, expected) in cases {
            assert_eq!(visible_limit_for_page(page), expected);
        }
    }

    #[test]
    fn parse_page_from_query_string_reads_page_param() {
        assert_page_cases(&[
            ("?page=2&sort=hot", Some(2)),
            ("page=3", Some(3)),
            ("?page=%32", Some(2)),
            ("?sort=hot", None),
            ("?page=0", None),
            ("?page=abc", None),
            ("?page=-1", None),
            ("?page=999", Some(MAX_BROWSER_PAGE)),
        ]);
    }

    #[test]
    fn parse_page_param_falls_back_for_invalid_values() {
        assert_parse_page_param_cases(&[
            (Some("2"), 2),
            (Some("%32"), 2),
            (Some("abc"), 1),
            (Some("0"), 1),
            (Some("-1"), 1),
            (None, 1),
            (Some("99"), MAX_BROWSER_PAGE),
        ]);
    }

    #[test]
    fn clamp_browser_page_bounds_visible_window() {
        assert_clamp_cases(&[(0, 1), (1, 1), (10, 10), (11, MAX_BROWSER_PAGE)]);
    }

    fn sample_cache_key(page: u32) -> BrowserCacheKey {
        BrowserCacheKey {
            sort: "hot".into(),
            filters: ToolFilters::default(),
            search_q: None,
            page,
        }
    }

    fn sample_browser_data(total: i64) -> BrowserDataPayload {
        BrowserDataPayload {
            categories: vec![],
            chains: vec![],
            total,
            tools: vec![],
            comment_counts: HashMap::new(),
            preview_tool: None,
        }
    }

    #[test]
    fn browser_data_for_render_uses_hydrated_resource_without_cache() {
        let current = sample_cache_key(2);
        let data = sample_browser_data(100);
        let page_state = Some(Ok(data.clone()));
        let resolved = browser_data_for_render(&page_state, &current, None);
        assert_eq!(resolved.map(|d| d.total), Some(100));
    }

    #[test]
    fn browser_data_for_render_ignores_cache_when_resource_ready() {
        let current = sample_cache_key(2);
        let hydrated = sample_browser_data(100);
        let stale = TaggedBrowserPayload {
            deps: sample_cache_key(1),
            data: sample_browser_data(50),
        };
        let page_state = Some(Ok(hydrated));
        let resolved = browser_data_for_render(&page_state, &current, Some(&stale));
        assert_eq!(resolved.map(|d| d.total), Some(100));
    }

    #[test]
    fn browser_data_for_render_reuses_cache_while_pending() {
        let current = sample_cache_key(1);
        let cached = TaggedBrowserPayload {
            deps: current.clone(),
            data: sample_browser_data(50),
        };
        let resolved = browser_data_for_render(&None, &current, Some(&cached));
        assert_eq!(resolved.map(|d| d.total), Some(50));
    }

    #[test]
    fn browser_data_for_render_returns_none_on_error() {
        let current = sample_cache_key(1);
        let cached = TaggedBrowserPayload {
            deps: current.clone(),
            data: sample_browser_data(50),
        };
        let page_state = Some(Err(ServerFnError::ServerError("boom".into())));
        assert!(browser_data_for_render(&page_state, &current, Some(&cached)).is_none());
    }

    #[test]
    fn browser_data_for_render_ignores_cache_when_deps_mismatch_and_pending() {
        let current = sample_cache_key(1);
        let stale = TaggedBrowserPayload {
            deps: sample_cache_key(2),
            data: sample_browser_data(100),
        };
        assert!(browser_data_for_render(&None, &current, Some(&stale)).is_none());
    }

    #[test]
    fn browser_data_for_render_returns_none_when_pending_without_cache() {
        let current = sample_cache_key(1);
        assert!(browser_data_for_render(&None, &current, None).is_none());
    }

    #[test]
    fn browser_data_for_render_rejects_stale_cache_on_sort_mismatch() {
        let current = sample_cache_key(1);
        let mut stale_key = sample_cache_key(1);
        stale_key.sort = "new".into();
        let stale = TaggedBrowserPayload {
            deps: stale_key,
            data: sample_browser_data(50),
        };
        assert!(browser_data_for_render(&None, &current, Some(&stale)).is_none());
    }

    #[test]
    fn browser_data_for_render_rejects_stale_cache_on_filter_mismatch() {
        let current = sample_cache_key(1);
        let mut stale_key = sample_cache_key(1);
        stale_key.filters.function = vec!["bridge".into()];
        let stale = TaggedBrowserPayload {
            deps: stale_key,
            data: sample_browser_data(50),
        };
        assert!(browser_data_for_render(&None, &current, Some(&stale)).is_none());
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
        let filters = BrowserQueryParams {
            function: Some("bridge,swap".into()),
            tool_type: Some("mcp".into()),
            ..query_params()
        };
        let from_new = build_sort_href(&BrowserBase::Tools, &filters, "new");
        assert_eq!(from_new.matches("sort=").count(), 1);
        assert!(
            from_new.contains("function=bridge%2Cswap")
                || from_new.contains("function=bridge,swap")
        );
        assert!(from_new.contains("sort=new"));

        let to_hot = build_sort_href(&BrowserBase::Tools, &filters, "hot");
        assert!(!to_hot.contains("sort="));
        assert!(
            to_hot.contains("function=bridge%2Cswap") || to_hot.contains("function=bridge,swap")
        );
        assert!(to_hot.contains("type=mcp"));
    }

    #[test]
    fn sort_href_omits_selected_preview() {
        let href = build_sort_href(&BrowserBase::Tools, &query_params(), "new");
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
