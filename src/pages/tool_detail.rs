//! Tool detail page — full tool info via ToolDetailContent + comments.

use crate::components::comments_section::CommentsSection;
use crate::components::error_state::ErrorState;
use crate::components::site_shell::SiteShell;
use crate::components::skeleton::ToolCardSkeleton;
use crate::components::tool_detail_content::ToolDetailContent;
use crate::components::tool_listing_actions::ToolListingActions;
use crate::models::Tool;
use crate::server::functions::get_tool_by_slug;
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

async fn load_tool(slug: String) -> Result<Tool, ServerFnError> {
    if slug.is_empty() {
        Err(ServerFnError::new("missing slug"))
    } else {
        get_tool_by_slug(slug).await
    }
}

#[component]
pub fn ToolDetailPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let slug = Memo::new(move |_| params.with(|p| p.get("slug").unwrap_or_default()));
    let retry = RwSignal::new(0u32);

    // Build back link preserving filter query (spec: ← All Tools keeps ?function=bridge&sort=hot)
    let back_href = Memo::new(move |_| {
        query.with(|qm| {
            let mut parts: Vec<String> = Vec::new();
            for (k, v) in qm.into_iter() {
                if k != "selected" {
                    parts.push(format!(
                        "{}={}",
                        urlencoding::encode(k),
                        urlencoding::encode(v)
                    ));
                }
            }
            if parts.is_empty() {
                "/tools".to_string()
            } else {
                format!("/tools?{}", parts.join("&"))
            }
        })
    });

    let tool = Resource::new(
        move || (slug.get(), retry.get()),
        |(s, _)| async move { load_tool(s).await },
    );

    view! {
        <SiteShell>
            <div class="detail-page px-4 md:px-8 py-8 max-w-[800px]">
            <Suspense fallback=|| view! { <ToolCardSkeleton/> }>
                {move || match tool.get() {
                    Some(Ok(t)) => {
                        let bh = back_href.get();
                        view! {
                            <a href=bh class="back-link">"← All Tools"</a>
                            <ToolDetailContent tool=t.clone() compact=false/>
                            <ToolListingActions tool=t.clone()/>
                            <CommentsSection slug=slug tool_name=t.name.clone()/>
                        }.into_any()
                    }
                    Some(Err(e)) => view! {
                        <ErrorState
                            message=format!("Tool not found: {e}")
                            on_retry=move || retry.update(|n| *n = n.wrapping_add(1))
                        />
                    }.into_any(),
                    None => view! { <ToolCardSkeleton/> }.into_any(),
                }}
            </Suspense>
            </div>
        </SiteShell>
    }
}
