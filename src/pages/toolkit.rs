//! My Toolkit page — signed-in saved tools and exports.

use crate::components::copy_button::CopyButton;
use crate::components::error_state::ErrorState;
use crate::components::login_modal::LoginModal;
use crate::components::site_shell::SiteShell;
use crate::components::skeleton::ToolListSkeleton;
use crate::components::tool_card::ToolCard;
use crate::discovery::compare_href;
use crate::server::functions::{
    list_my_toolkit, update_toolkit_item, MyToolkitPayload, ToolkitExportPayload, ToolkitToolView,
    UpdateToolkitItemPayload,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

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

fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

#[component]
fn ToolkitMetadataForm(item: ToolkitToolView, on_saved: Callback<()>) -> impl IntoView {
    let note = RwSignal::new(item.note.clone().unwrap_or_default());
    let tags = RwSignal::new(item.tags.join(", "));
    let busy = RwSignal::new(false);
    let message = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<String>);
    let slug = item.tool.slug.clone();

    view! {
        <div class="toolkit-metadata">
            <label>
                <span>"Note"</span>
                <textarea
                    prop:value=move || note.get()
                    maxlength="500"
                    placeholder="Why did you save this?"
                    on:input=move |ev| note.set(event_target_value(&ev))
                />
            </label>
            <label>
                <span>"Tags"</span>
                <input
                    type="text"
                    prop:value=move || tags.get()
                    placeholder="base, research"
                    on:input=move |ev| tags.set(event_target_value(&ev))
                />
            </label>
            <div class="toolkit-metadata-actions">
                <button
                    type="button"
                    class="toolkit-secondary-link"
                    disabled=move || busy.get()
                    on:click=move |_| {
                        if busy.get_untracked() {
                            return;
                        }
                        busy.set(true);
                        message.set(None);
                        error.set(None);
                        let slug = slug.clone();
                        let payload = UpdateToolkitItemPayload {
                            slug,
                            note: Some(note.get_untracked()),
                            tags: parse_tags(&tags.get_untracked()),
                        };
                        spawn_local(async move {
                            match update_toolkit_item(payload).await {
                                Ok(()) => {
                                    message.set(Some("Saved".into()));
                                    on_saved.run(());
                                }
                                Err(e) => error.set(Some(e.to_string())),
                            }
                            busy.set(false);
                        });
                    }
                >
                    "Save metadata"
                </button>
                {move || message.get().map(|text| view! {
                    <span class="toolkit-save-status" role="status">{text}</span>
                })}
                {move || error.get().map(|text| view! {
                    <span class="toolkit-save-error" role="alert">{text}</span>
                })}
            </div>
        </div>
    }
}

#[component]
fn ToolkitContent(payload: MyToolkitPayload, on_toolkit_changed: Callback<()>) -> impl IntoView {
    let items = payload.items.clone();
    let compare_slugs: Vec<String> = items
        .iter()
        .take(3)
        .map(|item| item.tool.slug.clone())
        .collect();
    let compare_url = compare_href(&compare_slugs);
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
                <div class="toolkit-header-actions">
                    <a href="/tools" class="toolkit-browse-link">"Browse tools"</a>
                    {if !compare_slugs.is_empty() {
                        view! {
                            <a href=compare_url class="toolkit-secondary-link">"Compare selected"</a>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                </div>
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
                <div>
                    <span>"Recent"</span>
                    <p>"Sorted by update"</p>
                </div>
            </section>

            {if items.is_empty() {
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
                            {items.into_iter().map(|item| {
                                let tool = item.tool.clone();
                                view! {
                                    <article class="toolkit-saved-item">
                                        <ToolCard
                                            tool=tool
                                            initially_starred=true
                                            on_bookmark_changed=Callback::new(move |starred: bool| {
                                                if !starred {
                                                    on_toolkit_changed.run(());
                                                }
                                            })
                                        />
                                        <ToolkitMetadataForm item=item on_saved=on_toolkit_changed/>
                                    </article>
                                }
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
    let show_login = RwSignal::new(false);
    view! {
        <LoginModal show=show_login/>
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
                <p>"Sign in to save tools and export your stack from the directory."</p>
                <div class="toolkit-auth-actions">
                    <button
                        type="button"
                        class="toolkit-primary-link cursor-pointer font-inherit"
                        data-testid="toolkit-sign-in"
                        on:click=move |_| show_login.set(true)
                    >
                        "Sign in"
                    </button>
                </div>
            </section>
        </div>
    }
}

#[component]
pub fn ToolkitPage() -> impl IntoView {
    let retry = RwSignal::new(0u32);
    let refresh_toolkit = Callback::new(move |_| retry.update(|n| *n = n.wrapping_add(1)));
    let toolkit = Resource::new_blocking(
        move || retry.get(),
        |_| async move { list_my_toolkit().await },
    );

    view! {
        <SiteShell>
            <Suspense fallback=|| view! { <div class="toolkit-page"><ToolListSkeleton count=4/></div> }>
                {move || match toolkit.get() {
                    Some(Ok(payload)) => view! {
                        <ToolkitContent payload=payload on_toolkit_changed=refresh_toolkit/>
                    }.into_any(),
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
