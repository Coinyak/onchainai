//! Tool detail page — trust facts, official links, comments.

use crate::components::comments_section::CommentsSection;
use crate::components::error_state::ErrorState;
use crate::components::site_shell::SiteShell;
use crate::components::skeleton::ToolCardSkeleton;
use crate::components::tool_detail_content::ToolDetailContent;
use crate::components::tool_listing_actions::ToolListingActions;
use crate::server::functions::get_tool_trust_view;
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

#[component]
pub fn ToolDetailPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let slug = Memo::new(move |_| params.with(|p| p.get("slug").unwrap_or_default()));
    let retry = RwSignal::new(0u32);

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

    let trust_view = Resource::new(
        move || (slug.get(), retry.get()),
        |(s, _)| async move {
            if s.is_empty() {
                Err(ServerFnError::new("missing slug"))
            } else {
                get_tool_trust_view(s).await
            }
        },
    );

    view! {
        <SiteShell>
            <div class="detail-page px-4 md:px-8 py-8 max-w-[800px]">
            <Suspense fallback=|| view! { <ToolCardSkeleton/> }>
                {move || match trust_view.get() {
                    Some(Ok(view)) => {
                        let bh = back_href.get();
                        let tool = view.tool.clone();
                        view! {
                            <a href=bh class="back-link">"← All Tools"</a>
                            <ToolDetailContent
                                tool=tool.clone()
                                compact=false
                                trust_facts=view.trust_facts.clone()
                                official_links=view.official_links.clone()
                                add_mcp_query_base="/tools".into()
                            />
                            <ToolListingActions tool=tool.clone()/>
                            <CommentsSection slug=slug tool_name=tool.name.clone()/>
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
