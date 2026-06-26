//! Admin user management — ban, admin role, delete.

use crate::pages::admin::admin_page_shell;
use crate::server::functions::{
    delete_user, list_admin_users, set_user_admin, set_user_banned, AdminUserView,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

#[component]
pub fn AdminUsersPage() -> impl IntoView {
    admin_page_shell(move || view! { <AdminUsersContent/> })
}

#[component]
fn AdminUsersContent() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let refresh = RwSignal::new(0u32);
    let users = Resource::new(
        move || (search.get(), refresh.get()),
        |(q, _)| async move {
            let query = if q.trim().is_empty() { None } else { Some(q) };
            list_admin_users(query, 50).await
        },
    );
    let action_error = RwSignal::new(None::<String>);
    let action_busy = RwSignal::new(false);

    let run_action = move |user_id: Uuid, action: &'static str| {
        if action_busy.get_untracked() {
            return;
        }
        action_busy.set(true);
        action_error.set(None);
        spawn_local(async move {
            let result = match action {
                "ban" => set_user_banned(user_id, true).await,
                "unban" => set_user_banned(user_id, false).await,
                "make_admin" => set_user_admin(user_id, true).await,
                "remove_admin" => set_user_admin(user_id, false).await,
                "delete" => delete_user(user_id).await,
                _ => Ok(()),
            };
            action_busy.set(false);
            match result {
                Ok(()) => refresh.update(|n| *n = n.wrapping_add(1)),
                Err(e) => action_error.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <div class="px-4 md:px-6 py-8 max-w-[960px] mx-auto">
            <div class="mb-6">
                <h1 class="text-[20px] font-semibold tracking-tight">"User Management"</h1>
                <p class="text-[#6B6B6B] text-[14px] mt-1">
                    "Ban users, grant admin access, or remove accounts."
                </p>
            </div>

            <input
                type="search"
                class="w-full max-w-sm mb-4 rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px]"
                placeholder="Search nickname or auth method..."
                prop:value=move || search.get()
                on:input=move |ev| search.set(event_target_value(&ev))
            />

            {move || action_error.get().map(|msg| view! {
                <p class="text-[14px] text-[#C0392B] mb-4">{msg}</p>
            })}

            <Suspense fallback=|| view! {
                <p class="text-[#6B6B6B] text-[14px]">"Loading users..."</p>
            }>
                {move || match users.get() {
                    Some(Ok(rows)) if rows.is_empty() => view! {
                        <p class="text-[#6B6B6B] text-[14px]">"No users found."</p>
                    }.into_any(),
                    Some(Ok(rows)) => view! {
                        <div class="space-y-3">
                            {rows.into_iter().map(|user| view! {
                                <UserRow user=user run_action=run_action busy=action_busy/>
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
fn UserRow(
    user: AdminUserView,
    run_action: impl Fn(Uuid, &'static str) + Copy + 'static,
    busy: RwSignal<bool>,
) -> impl IntoView {
    let nickname = user.nickname.clone().unwrap_or_else(|| "Unnamed".into());
    let user_id = user.id;
    let auth = user.auth_method.clone();

    view! {
        <div class="rounded-lg border border-[#E5E5E5] p-4 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
            <div>
                <div class="text-[14px] font-medium">
                    {nickname}
                    {if user.is_admin {
                        view! { <span class="ml-2 text-[11px] px-1.5 py-0.5 rounded bg-[#FFF3E6] text-[#E76F00]">"Admin"</span> }.into_any()
                    } else {
                        ().into_any()
                    }}
                    {if user.is_banned {
                        view! { <span class="ml-2 text-[11px] px-1.5 py-0.5 rounded bg-[#FDEDED] text-[#C0392B]">"Banned"</span> }.into_any()
                    } else {
                        ().into_any()
                    }}
                </div>
                <div class="text-[12px] text-[#6B6B6B] mt-1">
                    {auth}
                    " · "
                    {user.comment_count}
                    " comments · "
                    {user.bookmark_count}
                    " bookmarks"
                </div>
            </div>
            <div class="flex flex-wrap gap-2">
                {if user.is_banned {
                    view! {
                        <ActionButton label="Unban" action="unban" user_id busy run_action/>
                    }.into_any()
                } else {
                    view! {
                        <ActionButton label="Ban" action="ban" user_id busy run_action/>
                    }.into_any()
                }}
                {if user.is_admin {
                    view! {
                        <ActionButton label="Remove Admin" action="remove_admin" user_id busy run_action/>
                    }.into_any()
                } else {
                    view! {
                        <ActionButton label="Make Admin" action="make_admin" user_id busy run_action/>
                    }.into_any()
                }}
                <ActionButton label="Delete" action="delete" user_id busy run_action danger=true/>
            </div>
        </div>
    }
}

#[component]
fn ActionButton(
    label: &'static str,
    action: &'static str,
    user_id: Uuid,
    busy: RwSignal<bool>,
    run_action: impl Fn(Uuid, &'static str) + Copy + 'static,
    #[prop(optional)] danger: bool,
) -> impl IntoView {
    let class = if danger {
        "text-[13px] px-2 py-1 rounded border border-[#E5E5E5] text-[#C0392B] hover:bg-[#FAFAFA] disabled:opacity-50"
    } else {
        "text-[13px] px-2 py-1 rounded border border-[#E5E5E5] hover:bg-[#FAFAFA] disabled:opacity-50"
    };
    view! {
        <button
            type="button"
            class=class
            disabled=move || busy.get()
            on:click=move |_| run_action(user_id, action)
        >
            {label}
        </button>
    }
}
