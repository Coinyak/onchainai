//! Admin category management — CRUD for function categories.

use crate::components::top_nav::TopNav;
use crate::server::functions::{
    check_admin_access, create_category, delete_category, list_admin_categories, update_category,
    AdminCategoryView,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;

#[component]
pub fn AdminCategoriesPage() -> impl IntoView {
    let gate = Resource::new(|| (), |_| async move { check_admin_access().await });

    view! {
        <Suspense fallback=|| view! {
            <p class="px-6 py-12 text-[#6B6B6B] text-[14px]">"Checking access..."</p>
        }>
            {move || match gate.get() {
                Some(Ok(_)) => view! { <AdminCategoriesContent/> }.into_any(),
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
fn AdminCategoriesContent() -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let categories = Resource::new(
        move || refresh.get(),
        |_| async move { list_admin_categories().await },
    );
    let action_error = RwSignal::new(None::<String>);
    let action_busy = RwSignal::new(false);
    let show_create = RwSignal::new(false);

    view! {
        <TopNav/>
        <div class="px-4 md:px-6 py-8 max-w-[960px] mx-auto">
            <div class="flex items-baseline justify-between gap-4 mb-6">
                <div>
                    <h1 class="text-[20px] font-semibold tracking-tight">"Category Management"</h1>
                    <p class="text-[#6B6B6B] text-[14px] mt-1">
                        "Manage function categories shown on the home page and sidebar."
                    </p>
                </div>
                <a href="/admin" class="text-[14px] text-[#E76F00] hover:underline">"Admin home"</a>
            </div>

            {move || action_error.get().map(|msg| view! {
                <p class="text-[14px] text-[#C0392B] mb-4">{msg}</p>
            })}

            <button
                type="button"
                class="mb-4 text-[14px] px-3 py-1.5 rounded-md bg-[#1A1A1A] text-white hover:opacity-90"
                on:click=move |_| show_create.update(|v| *v = !*v)
            >
                {move || if show_create.get() { "Cancel" } else { "+ Add Category" }}
            </button>

            <Show when=move || show_create.get()>
                <CategoryForm
                    mode="create"
                    initial=None
                    busy=action_busy
                    error=action_error
                    on_done=move || {
                        show_create.set(false);
                        refresh.update(|n| *n = n.wrapping_add(1));
                    }
                />
            </Show>

            <Suspense fallback=|| view! {
                <p class="text-[#6B6B6B] text-[14px]">"Loading categories..."</p>
            }>
                {move || match categories.get() {
                    Some(Ok(rows)) => view! {
                        <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                            {rows.into_iter().map(|cat| view! {
                                <CategoryCard
                                    category=cat
                                    busy=action_busy
                                    error=action_error
                                    refresh=refresh
                                />
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
fn CategoryCard(
    category: AdminCategoryView,
    busy: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    refresh: RwSignal<u32>,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    let category = Arc::new(category);

    view! {
        <div class="rounded-lg border border-[#E5E5E5] p-4">
            {move || {
                if editing.get() {
                    view! {
                        <CategoryForm
                            mode="edit"
                            initial=Some((*category).clone())
                            busy=busy
                            error=error
                            on_done=move || {
                                editing.set(false);
                                refresh.update(|n| *n = n.wrapping_add(1));
                            }
                        />
                    }.into_any()
                } else {
                    let cat = (*category).clone();
                    let id = cat.id.clone();
                    let has_tools = cat.tool_count > 0;
                    view! {
                        <div class="font-medium text-[14px]">{cat.label.clone()}</div>
                        <div class="text-[12px] text-[#6B6B6B] mt-1">
                            {cat.id.clone()}
                            " · "
                            {cat.tool_count}
                            " tools · order "
                            {cat.sort_order}
                        </div>
                        <p class="text-[13px] text-[#6B6B6B] mt-2 line-clamp-2">{cat.description.clone()}</p>
                        <div class="mt-3 flex gap-2">
                            <button
                                type="button"
                                class="text-[13px] px-2 py-1 rounded border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                                disabled=move || busy.get()
                                on:click=move |_| editing.set(true)
                            >
                                "Edit"
                            </button>
                            <button
                                type="button"
                                class="text-[13px] px-2 py-1 rounded border border-[#E5E5E5] text-[#C0392B] hover:bg-[#FAFAFA] disabled:opacity-50"
                                disabled=move || busy.get() || has_tools
                                on:click=move |_| {
                                    if busy.get_untracked() || has_tools {
                                        return;
                                    }
                                    busy.set(true);
                                    error.set(None);
                                    let id_del = id.clone();
                                    spawn_local(async move {
                                        match delete_category(id_del).await {
                                            Ok(()) => refresh.update(|n| *n = n.wrapping_add(1)),
                                            Err(e) => error.set(Some(e.to_string())),
                                        }
                                        busy.set(false);
                                    });
                                }
                            >
                                "Delete"
                            </button>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

#[component]
fn CategoryForm(
    mode: &'static str,
    initial: Option<AdminCategoryView>,
    busy: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    on_done: impl Fn() + Send + Sync + Copy + 'static,
) -> impl IntoView {
    let id = RwSignal::new(initial.as_ref().map(|c| c.id.clone()).unwrap_or_default());
    let label = RwSignal::new(
        initial
            .as_ref()
            .map(|c| c.label.clone())
            .unwrap_or_default(),
    );
    let icon = RwSignal::new(
        initial
            .as_ref()
            .map(|c| c.icon.clone())
            .unwrap_or_else(|| "box".into()),
    );
    let description = RwSignal::new(
        initial
            .as_ref()
            .map(|c| c.description.clone())
            .unwrap_or_default(),
    );
    let sort_order = RwSignal::new(initial.as_ref().map(|c| c.sort_order).unwrap_or(99));
    let id_readonly = mode == "edit";

    view! {
        <div class="rounded-lg border border-[#E5E5E5] p-4 mb-4 bg-[#FAFAFA]">
            <div class="grid gap-3 sm:grid-cols-2">
                <label class="block text-[13px]">
                    "ID (slug)"
                    <input
                        class="mt-1 w-full rounded border border-[#E5E5E5] px-2 py-1.5 text-[14px]"
                        prop:readonly=move || id_readonly
                        prop:value=move || id.get()
                        on:input=move |ev| id.set(event_target_value(&ev))
                    />
                </label>
                <label class="block text-[13px]">
                    "Label"
                    <input
                        class="mt-1 w-full rounded border border-[#E5E5E5] px-2 py-1.5 text-[14px]"
                        prop:value=move || label.get()
                        on:input=move |ev| label.set(event_target_value(&ev))
                    />
                </label>
                <label class="block text-[13px]">
                    "Icon (Lucide name)"
                    <input
                        class="mt-1 w-full rounded border border-[#E5E5E5] px-2 py-1.5 text-[14px]"
                        prop:value=move || icon.get()
                        on:input=move |ev| icon.set(event_target_value(&ev))
                    />
                </label>
                <label class="block text-[13px]">
                    "Sort order"
                    <input
                        type="number"
                        class="mt-1 w-full rounded border border-[#E5E5E5] px-2 py-1.5 text-[14px]"
                        prop:value=move || sort_order.get().to_string()
                        on:input=move |ev| {
                            if let Ok(n) = event_target_value(&ev).parse::<i32>() {
                                sort_order.set(n);
                            }
                        }
                    />
                </label>
            </div>
            <label class="block text-[13px] mt-3">
                "Description"
                <textarea
                    class="mt-1 w-full rounded border border-[#E5E5E5] px-2 py-1.5 text-[14px] min-h-[72px]"
                    prop:value=move || description.get()
                    on:input=move |ev| description.set(event_target_value(&ev))
                />
            </label>
            <button
                type="button"
                class="mt-3 text-[14px] px-4 py-2 rounded-lg bg-[#1A1A1A] text-white hover:opacity-90 disabled:opacity-50"
                disabled=move || busy.get()
                on:click=move |_| {
                    if busy.get_untracked() {
                        return;
                    }
                    busy.set(true);
                    error.set(None);
                    let id_v = id.get_untracked();
                    let label_v = label.get_untracked();
                    let icon_v = icon.get_untracked();
                    let desc_v = description.get_untracked();
                    let order_v = sort_order.get_untracked();
                    spawn_local(async move {
                        let result = if mode == "create" {
                            create_category(id_v, label_v, icon_v, desc_v, order_v).await.map(|_| ())
                        } else {
                            update_category(id_v, label_v, icon_v, desc_v, order_v).await.map(|_| ())
                        };
                        busy.set(false);
                        match result {
                            Ok(()) => on_done(),
                            Err(e) => error.set(Some(e.to_string())),
                        }
                    });
                }
            >
                {if mode == "create" { "Create" } else { "Save" }}
            </button>
        </div>
    }
}
