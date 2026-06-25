//! Admin comment moderation — delete and ban.

use crate::components::top_nav::TopNav;
use crate::server::functions::{
    check_admin_access, delete_admin_comment, delete_comment_and_ban_user,
    list_admin_comments, AdminCommentView,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

#[component]
pub fn AdminCommentsPage() -> impl IntoView {
    let gate = Resource::new(|| (), |_| async move { check_admin_access().await });

    view! {
        <Suspense fallback=|| view! {
            <p class="px-6 py-12 text-[#6B6B6B] text-[14px]">"Checking access..."</p>
        }>
            {move || match gate.get() {
                Some(Ok(_)) => view! { <AdminCommentsContent/> }.into_any(),
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
fn AdminCommentsContent() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let refresh = RwSignal::new(0u32);
    let comments = Resource::new(
        move || (search.get(), refresh.get()),
        |(q, _)| async move {
            let query = if q.trim().is_empty() { None } else { Some(q) };
            list_admin_comments(query, 50).await
        },
    );
    let action_error = RwSignal::new(None::<String>);
    let action_busy = RwSignal::new(false);

    let run_delete = move |comment_id: Uuid, ban: bool| {
        if action_busy.get_untracked() {
            return;
        }
        action_busy.set(true);
        action_error.set(None);
        spawn_local(async move {
            let result = if ban {
                delete_comment_and_ban_user(comment_id).await
            } else {
                delete_admin_comment(comment_id).await
            };
            action_busy.set(false);
            match result {
                Ok(()) => refresh.update(|n| *n = n.wrapping_add(1)),
                Err(e) => action_error.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <TopNav/>
        <div class="px-4 md:px-6 py-8 max-w-[960px] mx-auto">
            <div class="flex items-baseline justify-between gap-4 mb-6">
                <div>
                    <h1 class="text-[20px] font-semibold tracking-tight">"Comment Management"</h1>
                    <p class="text-[#6B6B6B] text-[14px] mt-1">
                        "Remove spam comments or ban repeat offenders."
                    </p>
                </div>
                <a href="/admin" class="text-[14px] text-[#E76F00] hover:underline">"Admin home"</a>
            </div>

            <input
                type="search"
                class="w-full max-w-sm mb-4 rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px]"
                placeholder="Search content, author, or tool..."
                prop:value=move || search.get()
                on:input=move |ev| search.set(event_target_value(&ev))
            />

            {move || action_error.get().map(|msg| view! {
                <p class="text-[14px] text-[#C0392B] mb-4">{msg}</p>
            })}

            <Suspense fallback=|| view! {
                <p class="text-[#6B6B6B] text-[14px]">"Loading comments..."</p>
            }>
                {move || match comments.get() {
                    Some(Ok(rows)) if rows.is_empty() => view! {
                        <p class="text-[#6B6B6B] text-[14px]">"No comments found."</p>
                    }.into_any(),
                    Some(Ok(rows)) => view! {
                        <div class="space-y-4">
                            {rows.into_iter().map(|c| view! {
                                <CommentRow comment=c run_delete=run_delete busy=action_busy/>
                            }).collect_view()}
                        </div>
                    }.into_any(),
                    Some(Err(e)) => view! {
                        <p class="text-[14px] text-[#C0392B]">{e.to_string()}</p>
                    }.into_any(),
                    None => ().into_any(),
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn CommentRow(
    comment: AdminCommentView,
    run_delete: impl Fn(Uuid, bool) + Copy + 'static,
    busy: RwSignal<bool>,
) -> impl IntoView {
    let author = comment
        .author_nickname
        .clone()
        .unwrap_or_else(|| "User".into());
    let comment_id = comment.id;
    let tool_href = format!("/tools/{}", comment.tool_slug);

    view! {
        <div class="rounded-lg border border-[#E5E5E5] p-4">
            <div class="text-[13px] text-[#6B6B6B] mb-2">
                {author}
                {if comment.author_is_banned {
                    view! { <span class="ml-2 text-[#C0392B]">"(banned)"</span> }.into_any()
                } else {
                    ().into_any()
                }}
                " on "
                <a href=tool_href class="text-[#E76F00] hover:underline">{comment.tool_name.clone()}</a>
            </div>
            <p class="text-[14px] whitespace-pre-wrap">{comment.content.clone()}</p>
            <div class="mt-3 flex gap-2">
                <button
                    type="button"
                    class="text-[13px] px-2 py-1 rounded border border-[#E5E5E5] hover:bg-[#FAFAFA] disabled:opacity-50"
                    disabled=move || busy.get()
                    on:click=move |_| run_delete(comment_id, false)
                >
                    "Delete"
                </button>
                <button
                    type="button"
                    class="text-[13px] px-2 py-1 rounded border border-[#E5E5E5] text-[#C0392B] hover:bg-[#FAFAFA] disabled:opacity-50"
                    disabled=move || busy.get()
                    on:click=move |_| run_delete(comment_id, true)
                >
                    "Delete + Ban User"
                </button>
            </div>
        </div>
    }
}