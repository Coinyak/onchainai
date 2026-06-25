//! Tool comments, upvotes, replies, bookmark controls.

use crate::components::login_modal::LoginModal;
use crate::server::functions::{
    create_comment, get_current_user, get_tool_comments, is_bookmarked, toggle_bookmark,
    toggle_upvote, CommentView,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn CommentsSection(slug: Memo<String>, tool_name: String) -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let show_login = RwSignal::new(false);
    let comment_sort = RwSignal::new("new".to_string());

    let user = Resource::new(|| (), |_| async move { get_current_user().await });
    let comments = Resource::new(
        move || (refresh.get(), comment_sort.get(), slug.get()),
        move |(_, sort, slug)| async move { get_tool_comments(slug, sort).await },
    );
    let bookmarked = Resource::new(
        move || (slug.get(), refresh.get()),
        |(s, _)| async move { is_bookmarked(s).await },
    );
    let tool_name_for_comment = tool_name.clone();

    view! {
        <LoginModal show=show_login/>
        <section class="mt-10 border-t border-[#E5E5E5] pt-8">
            <div class="flex items-center justify-between gap-4 mb-4 flex-wrap">
                <h2 class="text-[20px] font-semibold">"Comments"</h2>
                <select
                    class="text-[13px] border border-[#E5E5E5] rounded-md px-2 py-1 bg-white"
                    prop:value=move || comment_sort.get()
                    on:change=move |ev| {
                        comment_sort.set(event_target_value(&ev));
                        refresh.update(|n| *n = n.wrapping_add(1));
                    }
                >
                    <option value="new">"Newest"</option>
                    <option value="top">"Top"</option>
                </select>
                <Suspense fallback=|| ()>
                    {move || bookmarked.get().map(|res| {
                        let active = res.unwrap_or(false);
                        let slug_bm = slug.get();
                        view! {
                            <button
                                type="button"
                                class="text-[14px] px-3 py-1.5 rounded-md border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                                on:click=move |_| {
                                    let slug_bm = slug_bm.clone();
                                    spawn_local(async move {
                                        match get_current_user().await {
                                            Ok(Some(_)) => {
                                                let _ = toggle_bookmark(slug_bm).await;
                                                refresh.update(|n| *n = n.wrapping_add(1));
                                            }
                                            _ => show_login.set(true),
                                        }
                                    });
                                }
                            >
                                {if active { "Bookmarked ★" } else { "Bookmark ☆" }}
                            </button>
                        }
                    })}
                </Suspense>
            </div>

            <CommentForm
                slug=slug
                tool_name=tool_name_for_comment
                parent_id=None
                user=user
                show_login=show_login
                on_posted=move || refresh.update(|n| *n = n.wrapping_add(1))
            />

            <Suspense fallback=|| view! {
                <p class="text-[#6B6B6B] text-[14px] mt-4">"Loading comments..."</p>
            }>
                {move || match comments.get() {
                    Some(Ok(rows)) if rows.is_empty() => view! {
                        <p class="text-[#6B6B6B] text-[14px] mt-4">"No comments yet. Be the first."</p>
                    }.into_any(),
                    Some(Ok(rows)) => {
                        let tops: Vec<_> = rows.iter().filter(|c| c.parent_id.is_none()).cloned().collect();
                        let tool_name_items = tool_name.clone();
                        view! {
                            <ul class="mt-6 space-y-4">
                                {tops.into_iter().map(|c| {
                                    let mut replies: Vec<_> = rows.iter()
                                        .filter(|r| r.parent_id == Some(c.id))
                                        .cloned()
                                        .collect();
                                    replies.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                                    view! {
                                        <CommentItem
                                            slug=slug
                                            tool_name=tool_name_items.clone()
                                            comment=c
                                            replies=replies
                                            user=user
                                            show_login=show_login
                                            refresh=refresh
                                        />
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any()
                    }
                    Some(Err(e)) => view! {
                        <p class="text-[14px] text-[#C0392B] mt-4">{e.to_string()}</p>
                    }.into_any(),
                    None => ().into_any(),
                }}
            </Suspense>
        </section>
    }
}

#[component]
fn CommentForm(
    slug: Memo<String>,
    tool_name: String,
    parent_id: Option<uuid::Uuid>,
    user: Resource<Result<Option<crate::auth::session::SessionUser>, ServerFnError>>,
    show_login: RwSignal<bool>,
    on_posted: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let content = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let label = if parent_id.is_some() {
        "Reply"
    } else {
        "Post"
    };
    let heading = if parent_id.is_some() {
        "Write a reply"
    } else {
        "Comment on"
    };

    view! {
        <div class="rounded-lg border border-[#E5E5E5] p-4 bg-[#FAFAFA]">
            <label class="block text-[14px] font-medium mb-2" for="comment-input">
                {if parent_id.is_some() {
                    heading.into_any()
                } else {
                    view! { {heading} " " {tool_name.clone()} }.into_any()
                }}
            </label>
            <textarea
                class="w-full min-h-[72px] rounded-md border border-[#E5E5E5] px-3 py-2 text-[14px] bg-white resize-y"
                placeholder=if parent_id.is_some() { "Write a reply..." } else { "Write a comment..." }
                prop:value=move || content.get()
                on:input=move |ev| content.set(event_target_value(&ev))
            />
            {move || error.get().map(|e| view! {
                <p class="text-[13px] text-[#C0392B] mt-2">{e}</p>
            })}
            <div class="mt-3 flex justify-end">
                <button
                    type="button"
                    class="px-4 py-2 rounded-lg bg-[#1A1A1A] text-white text-[14px] font-medium hover:opacity-90 disabled:opacity-50"
                    disabled=move || busy.get()
                    on:click=move |_| {
                        let slug = slug.get();
                        let text = content.get_untracked();
                        let pid = parent_id;
                        if text.trim().is_empty() {
                            return;
                        }
                        busy.set(true);
                        error.set(None);
                        spawn_local(async move {
                            match user.get_untracked() {
                                Some(Ok(Some(_))) => {
                                    match create_comment(slug, text, pid).await {
                                        Ok(_) => {
                                            content.set(String::new());
                                            on_posted();
                                        }
                                        Err(e) => error.set(Some(e.to_string())),
                                    }
                                }
                                _ => show_login.set(true),
                            }
                            busy.set(false);
                        });
                    }
                >
                    {label}
                </button>
            </div>
        </div>
    }
}

#[component]
fn CommentItem(
    slug: Memo<String>,
    tool_name: String,
    comment: CommentView,
    replies: Vec<CommentView>,
    user: Resource<Result<Option<crate::auth::session::SessionUser>, ServerFnError>>,
    show_login: RwSignal<bool>,
    refresh: RwSignal<u32>,
) -> impl IntoView {
    let show_reply = RwSignal::new(false);
    let parent_id = comment.id;
    let on_refresh = move || refresh.update(|n| *n = n.wrapping_add(1));

    view! {
        <li class="rounded-lg border border-[#E5E5E5] p-4">
            <CommentBody comment=comment show_login=show_login on_change=on_refresh/>
            <button
                type="button"
                class="mt-2 text-[13px] text-[#6B6B6B] hover:text-[#1A1A1A]"
                on:click=move |_| show_reply.update(|v| *v = !*v)
            >
                "Reply"
            </button>
            {move || show_reply.get().then(|| view! {
                <div class="mt-3">
                    <CommentForm
                        slug=slug
                        tool_name=tool_name.clone()
                        parent_id=Some(parent_id)
                        user=user
                        show_login=show_login
                        on_posted=move || {
                            show_reply.set(false);
                            on_refresh();
                        }
                    />
                </div>
            })}
            {if !replies.is_empty() {
                view! {
                    <ul class="mt-3 ml-4 space-y-3 border-l border-[#E5E5E5] pl-4">
                        {replies.into_iter().map(|r| view! {
                            <li>
                                <CommentBody comment=r show_login=show_login on_change=on_refresh/>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            } else {
                ().into_any()
            }}
        </li>
    }
}

#[component]
fn CommentBody(
    comment: CommentView,
    show_login: RwSignal<bool>,
    on_change: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let author = comment
        .author_nickname
        .clone()
        .unwrap_or_else(|| "User".into());
    let id = comment.id;

    view! {
        <div class="flex items-start justify-between gap-3">
            <div>
                <div class="text-[14px] font-medium">
                    {author}
                    {if comment.author_is_admin {
                        view! { <span class="ml-2 text-[11px] px-1.5 py-0.5 rounded bg-[#FFF3E6] text-[#E76F00]">"Admin"</span> }.into_any()
                    } else {
                        ().into_any()
                    }}
                </div>
                <p class="text-[14px] text-[#1A1A1A] mt-1 whitespace-pre-wrap">{comment.content.clone()}</p>
            </div>
            <button
                type="button"
                class="text-[13px] text-[#6B6B6B] hover:text-[#1A1A1A] shrink-0"
                on:click=move |_| {
                    spawn_local(async move {
                        match get_current_user().await {
                            Ok(Some(_)) => {
                                let _ = toggle_upvote(id).await;
                                on_change();
                            }
                            _ => show_login.set(true),
                        }
                    });
                }
            >
                {if comment.viewer_upvoted { "▲" } else { "△" }}
                " "
                {comment.upvote_count.to_string()}
            </button>
        </div>
    }
}