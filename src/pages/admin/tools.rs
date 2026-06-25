//! Admin tool approval — lists pending crawled/submitted tools for review.

use crate::components::top_nav::TopNav;
use crate::models::Tool;
use crate::server::functions::{check_admin_access, list_pending_tools, set_tool_approval};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;

type ApprovalHandler = Arc<dyn Fn(String, &'static str, Option<String>) + Send + Sync>;

#[component]
pub fn AdminToolsPage() -> impl IntoView {
    let gate = Resource::new(|| (), |_| async move { check_admin_access().await });

    view! {
        <Suspense fallback=|| view! {
            <p class="px-6 py-12 text-[#6B6B6B] text-[14px]">"Checking access..."</p>
        }>
            {move || match gate.get() {
                Some(Ok(_)) => view! { <AdminToolsContent/> }.into_any(),
                Some(Err(_)) => view! {
                    <div class="px-6 py-12 max-w-[720px] mx-auto text-center">
                        <h1 class="text-[28px] font-bold mb-4">"404"</h1>
                        <p class="text-[#6B6B6B]">"Page not found."</p>
                    </div>
                }.into_any(),
                None => ().into_any(),
            }}
        </Suspense>
    }
}

#[component]
fn AdminToolsContent() -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let pending = Resource::new(
        move || refresh.get(),
        |_| async move { list_pending_tools(50).await },
    );

    let reject_slug = RwSignal::new(None::<String>);
    let reject_reason = RwSignal::new(String::new());
    let action_error = RwSignal::new(None::<String>);
    let action_busy = RwSignal::new(false);

    let run_approval: ApprovalHandler = Arc::new(move |slug: String, status: &'static str, reason: Option<String>| {
            if action_busy.get_untracked() {
                return;
            }
            action_busy.set(true);
            action_error.set(None);
            spawn_local(async move {
                let result = set_tool_approval(slug, status.to_string(), reason).await;
                action_busy.set(false);
                match result {
                    Ok(()) => {
                        reject_slug.set(None);
                        reject_reason.set(String::new());
                        refresh.update(|n| *n = n.wrapping_add(1));
                    }
                    Err(e) => action_error.set(Some(e.to_string())),
                }
            });
    });

    let run_approval_for_rows = run_approval.clone();
    let run_approval_for_reject = run_approval.clone();

    view! {
            <TopNav/>
            <div class="px-4 md:px-6 py-8 max-w-[960px] mx-auto">
            <div class="flex items-baseline justify-between gap-4 mb-6">
                <div>
                    <h1 class="text-[20px] font-semibold tracking-tight">"Tool Management"</h1>
                    <p class="text-[#6B6B6B] text-[14px] mt-1">
                        "Review tools awaiting approval before they appear in public search."
                    </p>
                </div>
                <a href="/admin" class="text-[14px] text-[#E76F00] hover:underline">"Admin home"</a>
            </div>

            {move || {
                action_error.get().map(|msg| view! {
                    <p class="mb-4 text-[14px] text-[#C0392B] border border-[#C0392B]/30 rounded-lg px-4 py-2 bg-[#C0392B]/5">
                        {msg}
                    </p>
                })
            }}

            <Suspense fallback=|| view! {
                <p class="text-[#6B6B6B] text-[14px]">"Loading pending tools..."</p>
            }>
                {move || {
                    pending.get().map(|res| match res {
                        Ok(tools) if tools.is_empty() => view! {
                            <div class="rounded-xl border border-[#E5E5E5] bg-[#FAFAFA] px-6 py-10 text-center">
                                <p class="text-[16px] font-medium mb-2">"No pending tools"</p>
                                <p class="text-[#6B6B6B] text-[14px]">
                                    "New crawled or submitted tools will appear here when approval is required."
                                </p>
                            </div>
                        }.into_any(),
                        Ok(tools) => view! {
                            <div class="space-y-4">
                                <p class="text-[14px] text-[#6B6B6B]">
                                    {format!("{} pending", tools.len())}
                                </p>
                                {tools
                                    .into_iter()
                                    .map(|tool| {
                                        view! {
                                            <PendingToolRow
                                                tool=tool
                                                run_approval=run_approval_for_rows.clone()
                                                reject_slug=reject_slug
                                                action_busy=action_busy
                                            />
                                        }
                                    })
                                    .collect::<Vec<_>>()}
                            </div>
                        }.into_any(),
                        Err(e) => view! {
                            <p class="text-[14px] text-[#C0392B]">
                                "Failed to load pending tools: " {e.to_string()}
                            </p>
                        }.into_any(),
                    })
                }}
            </Suspense>

            {move || {
                reject_slug.get().map(|slug| {
                    let slug_for_submit = slug.clone();
                    let run_approval = run_approval_for_reject.clone();
                    view! {
                        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40">
                            <div
                                class="w-full max-w-md rounded-xl bg-white border border-[#E5E5E5] shadow-lg p-5"
                                role="dialog"
                                aria-labelledby="reject-title"
                            >
                                <h3 id="reject-title" class="text-[16px] font-semibold mb-2">
                                    "Reject tool"
                                </h3>
                                <p class="text-[14px] text-[#6B6B6B] mb-4">
                                    "Provide a reason for rejecting "
                                    <span class="font-mono">{slug.clone()}</span>
                                </p>
                                <textarea
                                    class="w-full min-h-[96px] rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px] focus:outline-none focus:border-[#E76F00]"
                                    placeholder="Reason (required)"
                                    prop:value=move || reject_reason.get()
                                    on:input=move |ev| {
                                        reject_reason.set(event_target_value(&ev));
                                    }
                                />
                                <div class="flex justify-end gap-2 mt-4">
                                    <button
                                        type="button"
                                        class="px-3 py-1.5 text-[14px] rounded-lg border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                                        on:click=move |_| {
                                            reject_slug.set(None);
                                            reject_reason.set(String::new());
                                        }
                                    >
                                        "Cancel"
                                    </button>
                                    <button
                                        type="button"
                                        class="px-3 py-1.5 text-[14px] rounded-lg bg-[#C0392B] text-white hover:opacity-90 disabled:opacity-50"
                                        disabled=move || {
                                            action_busy.get()
                                                || reject_reason.get().trim().is_empty()
                                        }
                                        on:click=move |_| {
                                            let reason = reject_reason.get().trim().to_string();
                                            if !reason.is_empty() {
                                                run_approval(
                                                    slug_for_submit.clone(),
                                                    "rejected",
                                                    Some(reason),
                                                );
                                            }
                                        }
                                    >
                                        "Confirm reject"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }
                })
            }}
            </div>
    }
}

#[component]
fn PendingToolRow(
    tool: Tool,
    run_approval: ApprovalHandler,
    reject_slug: RwSignal<Option<String>>,
    action_busy: RwSignal<bool>,
) -> impl IntoView {
    let slug = tool.slug.clone();
    let slug_display = slug.clone();
    let slug_href = slug.clone();
    let slug_for_approve = slug.clone();
    let slug_for_reject = slug;
    let description = tool
        .description
        .clone()
        .unwrap_or_else(|| "No description.".into());
    let tool_type = tool.tool_type.clone();
    let source = tool.source.clone();

    view! {
        <article class="rounded-xl border border-[#E5E5E5] bg-white p-4 md:p-5">
            <div class="flex flex-col md:flex-row md:items-start md:justify-between gap-4">
                <div class="min-w-0 flex-1">
                    <div class="flex flex-wrap items-center gap-2 mb-1">
                        <h2 class="text-[16px] font-semibold">{tool.name.clone()}</h2>
                        <span class="badge badge-neutral text-[12px]">{tool_type}</span>
                        <span class="badge badge-neutral text-[12px]">{source}</span>
                        <span class="badge badge-neutral text-[12px] bg-[#FFF4E6] text-[#E76F00] border-[#E76F00]/20">
                            "Pending"
                        </span>
                    </div>
                    <p class="text-[#6B6B6B] text-[14px] leading-relaxed mb-2">{description}</p>
                    <p class="text-[12px] text-[#999999] font-mono truncate">{slug_display}</p>
                </div>
                <div class="flex flex-wrap gap-2 shrink-0">
                    <a
                        href=format!("/tools/{slug_href}")
                        class="px-3 py-1.5 text-[14px] rounded-lg border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                    >
                        "View"
                    </a>
                    <button
                        type="button"
                        class="px-3 py-1.5 text-[14px] rounded-lg bg-[#2D7D46] text-white hover:opacity-90 disabled:opacity-50"
                        disabled=move || action_busy.get()
                        on:click={
                            let run_approval = run_approval.clone();
                            move |_| run_approval(slug_for_approve.clone(), "approved", None)
                        }
                    >
                        "Approve"
                    </button>
                    <button
                        type="button"
                        class="px-3 py-1.5 text-[14px] rounded-lg border border-[#C0392B] text-[#C0392B] hover:bg-[#C0392B]/5 disabled:opacity-50"
                        disabled=move || action_busy.get()
                        on:click=move |_| reject_slug.set(Some(slug_for_reject.clone()))
                    >
                        "Reject"
                    </button>
                </div>
            </div>
        </article>
    }
}