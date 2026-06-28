//! My Toolkit page — signed-in saved tools and exports.

use crate::components::copy_button::CopyButton;
use crate::components::error_state::ErrorState;
use crate::components::site_shell::SiteShell;
use crate::components::skeleton::ToolListSkeleton;
use crate::components::tool_card::ToolCard;
use crate::server::functions::{list_my_toolkit, MyToolkitPayload, ToolkitExportPayload};
use leptos::prelude::*;

#[component]
fn ExportPanel(title: &'static str, export: ToolkitExportPayload) -> impl IntoView {
    view! {
        <section class="toolkit-export-panel">
            <div class="toolkit-export-heading">
                <div>
                    <h2>{title}</h2>
                    <p>{export.filename.clone()}</p>
                </div>
                <CopyButton text=export.body.clone() label="Copy export"/>
            </div>
            <pre class="toolkit-export-body">{export.body}</pre>
        </section>
    }
}

#[component]
fn ToolkitContent(payload: MyToolkitPayload) -> impl IntoView {
    let tools = payload.tools.clone();
    view! {
        <div class="toolkit-page">
            <section class="toolkit-header">
                <div>
                    <p class="dashboard-kicker">"My Toolkit"</p>
                    <h1>"Saved tools for your agent stack"</h1>
                    <p>
                        "Your saved OnchainAI tools stay private to your account and can be copied into an agent setup checklist."
                    </p>
                </div>
                <a href="/tools" class="toolkit-browse-link">"Browse tools"</a>
            </section>

            <section class="toolkit-summary" aria-label="Toolkit summary">
                <div>
                    <span>{payload.total.to_string()}</span>
                    <p>"Saved tools"</p>
                </div>
                <div>
                    <span>"2"</span>
                    <p>"Export formats"</p>
                </div>
            </section>

            {if tools.is_empty() {
                view! {
                    <section class="toolkit-empty">
                        <h2>"No saved tools yet"</h2>
                        <p>"Use the Toolkit button on any tool card to build your shortlist."</p>
                        <a href="/tools" class="toolkit-primary-link">"Find tools"</a>
                    </section>
                }.into_any()
            } else {
                view! {
                    <div class="toolkit-layout">
                        <section class="toolkit-list" aria-label="Saved tools">
                            {tools.into_iter().map(|tool| {
                                view! { <ToolCard tool=tool initially_starred=true/> }
                            }).collect_view()}
                        </section>
                        <aside class="toolkit-export-stack">
                            <ExportPanel title="Markdown checklist" export=payload.markdown_export/>
                            <ExportPanel title="JSON payload" export=payload.json_export/>
                        </aside>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn ToolkitSignIn() -> impl IntoView {
    view! {
        <div class="toolkit-page">
            <section class="toolkit-header">
                <div>
                    <p class="dashboard-kicker">"My Toolkit"</p>
                    <h1>"Sign in to save your stack"</h1>
                    <p>"The public dashboard is available without login. Saving tools and exports require your account."</p>
                </div>
                <a href="/dashboard" class="toolkit-browse-link">"Open dashboard"</a>
            </section>
            <section class="toolkit-empty">
                <h2>"Create your personal toolkit"</h2>
                <p>"Continue with GitHub or connect a wallet, then save tools from the directory."</p>
                <div class="toolkit-auth-actions">
                    <a href="/auth/github" class="toolkit-primary-link">"Continue with GitHub"</a>
                    <a href="/login" class="toolkit-secondary-link">"Other sign-in options"</a>
                </div>
            </section>
        </div>
    }
}

#[component]
pub fn ToolkitPage() -> impl IntoView {
    let retry = RwSignal::new(0u32);
    let toolkit = Resource::new_blocking(
        move || retry.get(),
        |_| async move { list_my_toolkit().await },
    );

    view! {
        <SiteShell>
            <Suspense fallback=|| view! { <div class="toolkit-page"><ToolListSkeleton count=4/></div> }>
                {move || match toolkit.get() {
                    Some(Ok(payload)) => view! { <ToolkitContent payload=payload/> }.into_any(),
                    Some(Err(error)) if error.to_string().contains("sign in required")
                        || error.to_string().contains("authentication")
                        || error.to_string().contains("session") => {
                        view! { <ToolkitSignIn/> }.into_any()
                    }
                    Some(Err(error)) => view! {
                        <div class="toolkit-page">
                            <ErrorState
                                message=format!("Toolkit failed to load: {error}")
                                on_retry=move || retry.update(|n| *n = n.wrapping_add(1))
                            />
                        </div>
                    }.into_any(),
                    None => view! { <div class="toolkit-page"><ToolListSkeleton count=4/></div> }.into_any(),
                }}
            </Suspense>
        </SiteShell>
    }
}
