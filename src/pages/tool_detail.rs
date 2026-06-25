//! Tool detail page — full tool info, max-width 720px.

use crate::components::top_nav::TopNav;
use crate::server::functions::get_tool_by_slug;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn ToolDetailPage() -> impl IntoView {
    let params = use_params_map();
    let slug = Memo::new(move |_| params.with(|p| p.get("slug").unwrap_or_default()));

    let tool = Resource::new(
        move || slug.get(),
        |s| async move {
            if s.is_empty() {
                Err(ServerFnError::new("missing slug"))
            } else {
                get_tool_by_slug(s).await
            }
        },
    );

    view! {
        <TopNav/>
        <div class="detail-page max-w-[720px] mx-auto px-4 py-8">
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                {move || {
                    tool.get().map(|res| match res {
                        Ok(t) => {
                            let install = t.install_command.clone().unwrap_or_default();
                            let desc = t
                                .description
                                .clone()
                                .unwrap_or_else(|| "No description.".into());
                            view! {
                                <a href="/tools" class="back-link">"← Back to tools"</a>
                                <header class="detail-header">
                                    <h1 class="detail-title">{t.name.clone()}</h1>
                                    <div class="tool-badges">
                                        <span class="badge badge-neutral">{t.tool_type.to_uppercase()}</span>
                                        <span class="badge badge-neutral">{t.status.clone()}</span>
                                    </div>
                                </header>
                                <p class="detail-desc">{desc}</p>
                                <div class="detail-meta">
                                    <span>{"★ "}{t.stars}</span>
                                    {if !t.chains.is_empty() {
                                        view! {
                                            <span class="tool-chains">
                                                {t.chains
                                                    .iter()
                                                    .map(|c| view! { <span class="chain-pill">{c.clone()}</span> })
                                                    .collect_view()}
                                            </span>
                                        }
                                        .into_any()
                                    } else {
                                        ().into_any()
                                    }}
                                </div>
                                {if !install.is_empty() {
                                    view! {
                                        <section class="install-section">
                                            <h2 class="text-[20px] font-semibold mb-2">"Install"</h2>
                                            <div class="tool-install">
                                                <code class="install-cmd">{install.clone()}</code>
                                                <button type="button" class="copy-btn" data-copy=install>
                                                    "Copy"
                                                </button>
                                            </div>
                                        </section>
                                    }
                                    .into_any()
                                } else {
                                    ().into_any()
                                }}
                                {if let Some(url) = t.repo_url.clone() {
                                    view! {
                                        <p class="mt-4">
                                            <a href=url target="_blank" rel="noopener" class="external-link">
                                                "GitHub"
                                            </a>
                                        </p>
                                    }
                                    .into_any()
                                } else {
                                    ().into_any()
                                }}
                            }
                            .into_any()
                        }
                        Err(_) => view! {
                            <h1 class="text-[28px] font-bold">"404"</h1>
                            <p class="text-[#6B6B6B]">"Tool not found."</p>
                        }
                        .into_any(),
                    })
                }}
            </Suspense>
        </div>
    }
}