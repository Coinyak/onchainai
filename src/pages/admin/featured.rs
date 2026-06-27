//! Admin featured carousel management.

use crate::pages::admin::admin_page_shell;
use crate::server::functions::{
    create_featured_card, delete_featured_card, list_featured_cards, search_tools_for_picker,
    update_featured_card, AdminFeaturedCardView, FeaturedCardInput, ToolPickerItem,
    UpdateFeaturedCardInput,
};
#[cfg(feature = "hydrate")]
use crate::server::functions::{upload_featured_image, UploadFeaturedImageInput};
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

#[component]
pub fn AdminFeaturedPage() -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let cards = Resource::new(
        move || refresh.get(),
        |_| async move { list_featured_cards().await },
    );
    let action_error = RwSignal::new(None::<String>);
    let action_busy = RwSignal::new(false);
    let show_create = RwSignal::new(false);
    let editing_id = RwSignal::new(None::<Uuid>);

    admin_page_shell(move || {
        view! {
            <div class="px-4 md:px-6 py-8 max-w-[1100px] mx-auto">
                <div class="mb-6">
                    <h1 class="text-[20px] font-semibold tracking-tight">"Featured Carousel"</h1>
                    <p class="text-[#6B6B6B] text-[14px] mt-1">
                        "Manage highlight cards shown on the home page below the hero."
                    </p>
                </div>

                {move || action_error.get().map(|msg| view! {
                    <p class="text-[14px] text-[#C0392B] mb-4">{msg}</p>
                })}

                <button
                    type="button"
                    class="mb-4 text-[14px] px-3 py-1.5 rounded-md bg-[#1A1A1A] text-white hover:opacity-90"
                    on:click=move |_| {
                        editing_id.set(None);
                        show_create.update(|v| *v = !*v);
                    }
                >
                    {move || if show_create.get() { "Cancel" } else { "+ Add featured card" }}
                </button>

                <Show when=move || show_create.get()>
                    <FeaturedCardForm
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
                    <p class="text-[#6B6B6B] text-[14px]">"Loading featured cards..."</p>
                }>
                    {move || match cards.get() {
                        Some(Ok(rows)) => view! {
                            <div class="overflow-x-auto rounded-lg border border-[#E5E5E5]">
                                <table class="w-full text-left text-[14px]">
                                    <thead class="bg-[#FAFAFA] text-[#6B6B6B]">
                                        <tr>
                                            <th class="px-4 py-3 font-medium">"Preview"</th>
                                            <th class="px-4 py-3 font-medium">"Tool"</th>
                                            <th class="px-4 py-3 font-medium">"Headline"</th>
                                            <th class="px-4 py-3 font-medium">"Order"</th>
                                            <th class="px-4 py-3 font-medium">"Active"</th>
                                            <th class="px-4 py-3 font-medium">"Actions"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {rows.into_iter().map(|card| {
                                            let card_for_edit = card.clone();
                                            view! {
                                                <FeaturedCardRow
                                                    card=card
                                                    editing_id=editing_id
                                                    action_busy=action_busy
                                                    action_error=action_error
                                                    refresh=refresh
                                                />
                                                <Show when=move || editing_id.get() == Some(card_for_edit.id)>
                                                    <tr class="border-t border-[#E5E5E5] bg-[#FAFAFA]">
                                                        <td colspan="6" class="px-4 py-4">
                                                            <FeaturedCardForm
                                                                mode="edit"
                                                                initial=Some(card_for_edit.clone())
                                                                busy=action_busy
                                                                error=action_error
                                                                on_done=move || {
                                                                    editing_id.set(None);
                                                                    refresh.update(|n| *n = n.wrapping_add(1));
                                                                }
                                                            />
                                                        </td>
                                                    </tr>
                                                </Show>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        }.into_any(),
                        Some(Err(e)) => view! {
                            <p class="text-[14px] text-[#C0392B]">"Failed to load featured cards: " {e.to_string()}</p>
                        }.into_any(),
                        None => ().into_any(),
                    }}
                </Suspense>
            </div>
        }
    })
}

#[component]
fn FeaturedCardRow(
    card: AdminFeaturedCardView,
    editing_id: RwSignal<Option<Uuid>>,
    action_busy: RwSignal<bool>,
    action_error: RwSignal<Option<String>>,
    refresh: RwSignal<u32>,
) -> impl IntoView {
    let id = card.id;
    let image_url = card.image_url.clone();
    let tool_name = card.tool_name.clone();
    let tool_slug = card.tool_slug.clone();
    let headline = card.headline.clone().unwrap_or_else(|| "—".into());
    let sort_order = card.sort_order;
    let is_active = card.is_active;

    view! {
        <tr class="border-t border-[#E5E5E5]">
            <td class="px-4 py-3">
                <img src=image_url class="h-12 w-20 object-cover rounded-md border border-[#E5E5E5]" alt="Featured card preview"/>
            </td>
            <td class="px-4 py-3">
                <div class="font-medium">{tool_name}</div>
                <div class="text-[#6B6B6B] text-[13px]">"/tools/"{tool_slug}</div>
            </td>
            <td class="px-4 py-3">{headline}</td>
            <td class="px-4 py-3">{sort_order}</td>
            <td class="px-4 py-3">{if is_active { "Yes" } else { "No" }}</td>
            <td class="px-4 py-3">
                <div class="flex gap-2">
                    <button
                        type="button"
                        class="text-[13px] px-2 py-1 rounded border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                        on:click=move |_| editing_id.set(Some(id))
                    >
                        "Edit"
                    </button>
                    <button
                        type="button"
                        class="text-[13px] px-2 py-1 rounded border border-[#C0392B]/30 text-[#C0392B] hover:bg-[#C0392B]/5"
                        disabled=move || action_busy.get()
                        on:click=move |_| {
                            if action_busy.get_untracked() {
                                return;
                            }
                            action_busy.set(true);
                            action_error.set(None);
                            spawn_local(async move {
                                let result = delete_featured_card(id).await;
                                action_busy.set(false);
                                match result {
                                    Ok(()) => refresh.update(|n| *n = n.wrapping_add(1)),
                                    Err(e) => action_error.set(Some(e.to_string())),
                                }
                            });
                        }
                    >
                        "Delete"
                    </button>
                </div>
            </td>
        </tr>
    }
}

#[component]
fn FeaturedCardForm(
    mode: &'static str,
    initial: Option<AdminFeaturedCardView>,
    busy: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    on_done: impl Fn() + Send + Sync + Copy + 'static,
) -> impl IntoView {
    let card_id = initial.as_ref().map(|c| c.id);
    let tool_id = RwSignal::new(
        initial
            .as_ref()
            .map(|c| c.tool_id)
            .unwrap_or_else(Uuid::nil),
    );
    let tool_label = RwSignal::new(
        initial
            .as_ref()
            .map(|c| format!("{} ({})", c.tool_name, c.tool_slug))
            .unwrap_or_default(),
    );
    let image_url = RwSignal::new(
        initial
            .as_ref()
            .map(|c| c.image_url.clone())
            .unwrap_or_default(),
    );
    let headline = RwSignal::new(
        initial
            .as_ref()
            .and_then(|c| c.headline.clone())
            .unwrap_or_default(),
    );
    let subtitle = RwSignal::new(
        initial
            .as_ref()
            .and_then(|c| c.subtitle.clone())
            .unwrap_or_default(),
    );
    let sort_order = RwSignal::new(initial.as_ref().map(|c| c.sort_order).unwrap_or(0));
    let is_active = RwSignal::new(initial.as_ref().map(|c| c.is_active).unwrap_or(true));

    let tool_query = RwSignal::new(String::new());
    let picker_results = Resource::new(
        move || tool_query.get(),
        |q| async move {
            if q.trim().is_empty() {
                Ok(Vec::<ToolPickerItem>::new())
            } else {
                search_tools_for_picker(q, 8).await
            }
        },
    );

    let on_pick = move |item: ToolPickerItem| {
        tool_id.set(item.id);
        tool_label.set(format!("{} ({})", item.name, item.slug));
        tool_query.set(String::new());
    };

    let on_upload = move |ev: leptos::ev::Event| {
        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::JsCast;

            let input: web_sys::HtmlInputElement = event_target(&ev);
            let Some(file_list) = input.files() else {
                return;
            };
            let Some(file) = file_list.get(0) else {
                return;
            };

            let filename = file.name();
            let content_type = if file.type_().is_empty() {
                "image/png".to_string()
            } else {
                file.type_()
            };

            busy.set(true);
            error.set(None);

            let reader = web_sys::FileReader::new().expect("FileReader");
            let reader_for_result = reader.clone();
            let busy_for_load = busy;
            let error_for_load = error;
            let image_url_for_load = image_url;

            let onload =
                wasm_bindgen::closure::Closure::wrap(Box::new(move |_evt: web_sys::Event| {
                    let data_url = reader_for_result
                        .result()
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_default();
                    let data_base64 = data_url
                        .split_once(",")
                        .map(|(_, b64)| b64.to_string())
                        .unwrap_or_default();

                    let filename = filename.clone();
                    let content_type = content_type.clone();
                    spawn_local(async move {
                        let result = upload_featured_image(UploadFeaturedImageInput {
                            filename,
                            content_type,
                            data_base64,
                        })
                        .await;
                        busy_for_load.set(false);
                        match result {
                            Ok(url) => image_url_for_load.set(url),
                            Err(e) => error_for_load.set(Some(e.to_string())),
                        }
                    });
                })
                    as Box<dyn FnMut(web_sys::Event)>);

            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
            onload.forget();
            let _ = reader.read_as_data_url(&file);
        }
        #[cfg(not(feature = "hydrate"))]
        {
            let _ = ev;
        }
    };

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if busy.get_untracked() {
            return;
        }
        if tool_id.get_untracked().is_nil() {
            error.set(Some("Select a tool".into()));
            return;
        }
        if image_url.get_untracked().trim().is_empty() {
            error.set(Some("Upload an image".into()));
            return;
        }
        busy.set(true);
        error.set(None);

        let payload = (
            tool_id.get_untracked(),
            image_url.get_untracked(),
            headline.get_untracked(),
            subtitle.get_untracked(),
            sort_order.get_untracked(),
            is_active.get_untracked(),
        );

        spawn_local(async move {
            let result = if mode == "edit" {
                let id = card_id.expect("edit requires card id");
                update_featured_card(UpdateFeaturedCardInput {
                    id,
                    tool_id: payload.0,
                    image_url: payload.1,
                    headline: Some(payload.2),
                    subtitle: Some(payload.3),
                    sort_order: payload.4,
                    is_active: payload.5,
                })
                .await
                .map(|_| ())
            } else {
                create_featured_card(FeaturedCardInput {
                    tool_id: payload.0,
                    image_url: payload.1,
                    headline: Some(payload.2),
                    subtitle: Some(payload.3),
                    sort_order: payload.4,
                    is_active: payload.5,
                })
                .await
                .map(|_| ())
            };
            busy.set(false);
            match result {
                Ok(()) => on_done(),
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <form class="rounded-lg border border-[#E5E5E5] p-4 mb-6 space-y-4" on:submit=on_submit>
            <h2 class="text-[16px] font-semibold">
                {if mode == "edit" { "Edit featured card" } else { "New featured card" }}
            </h2>

            <label class="block">
                <span class="text-[14px] font-medium">"Tool"</span>
                <input
                    class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px]"
                    placeholder="Search by name or slug"
                    prop:value=move || tool_query.get()
                    on:input=move |ev| tool_query.set(event_target_value(&ev))
                />
                {move || tool_label.get().is_empty().then(|| view! {
                    <p class="text-[13px] text-[#6B6B6B] mt-1">"Search and pick a tool below."</p>
                })}
                {move || (!tool_label.get().is_empty()).then(|| view! {
                    <p class="text-[13px] text-[#1A7F4B] mt-1">"Selected: " {tool_label.get()}</p>
                })}
                <Suspense fallback=|| ()>
                    {move || picker_results.get().and_then(|res| res.ok()).map(|items| {
                        if items.is_empty() {
                            ().into_any()
                        } else {
                            view! {
                                <ul class="mt-2 rounded-lg border border-[#E5E5E5] divide-y divide-[#E5E5E5]">
                                    {items.into_iter().map(|item| {
                                        let pick_item = item.clone();
                                        view! {
                                            <li>
                                                <button
                                                    type="button"
                                                    class="w-full text-left px-3 py-2 text-[14px] hover:bg-[#FAFAFA]"
                                                    on:click=move |_| on_pick(pick_item.clone())
                                                >
                                                    {item.name.clone()}
                                                    <span class="text-[#6B6B6B]">" ("{item.slug}")"</span>
                                                </button>
                                            </li>
                                        }
                                    }).collect_view()}
                                </ul>
                            }.into_any()
                        }
                    })}
                </Suspense>
            </label>

            <label class="block">
                <span class="text-[14px] font-medium">"Image"</span>
                <input
                    class="mt-1 block text-[14px]"
                    type="file"
                    accept="image/png,image/jpeg,image/webp,image/svg+xml"
                    on:change=on_upload
                />
                {move || image_url.get().is_empty().then(|| view! {
                    <p class="text-[13px] text-[#6B6B6B] mt-1">"Upload a landscape image for the carousel."</p>
                })}
                {move || (!image_url.get().is_empty()).then(|| {
                    let url = image_url.get();
                    view! {
                        <img src=url class="mt-2 h-24 w-full max-w-[360px] object-cover rounded-lg border border-[#E5E5E5]" alt="Uploaded preview"/>
                    }
                })}
            </label>

            <label class="block">
                <span class="text-[14px] font-medium">"Headline (optional)"</span>
                <input
                    class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px]"
                    prop:value=move || headline.get()
                    on:input=move |ev| headline.set(event_target_value(&ev))
                />
            </label>

            <label class="block">
                <span class="text-[14px] font-medium">"Subtitle (optional)"</span>
                <input
                    class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px]"
                    prop:value=move || subtitle.get()
                    on:input=move |ev| subtitle.set(event_target_value(&ev))
                />
            </label>

            <div class="flex flex-wrap gap-4">
                <label class="block">
                    <span class="text-[14px] font-medium">"Sort order"</span>
                    <input
                        class="mt-1 w-28 rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px]"
                        type="number"
                        prop:value=move || sort_order.get().to_string()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                sort_order.set(v);
                            }
                        }
                    />
                </label>
                <label class="flex items-center gap-2 mt-6 text-[14px]">
                    <input
                        type="checkbox"
                        prop:checked=move || is_active.get()
                        on:change=move |ev| {
                            #[cfg(feature = "hydrate")]
                            {
                                let input: web_sys::HtmlInputElement = event_target(&ev);
                                is_active.set(input.checked());
                            }
                            #[cfg(not(feature = "hydrate"))]
                            {
                                let _ = ev;
                            }
                        }
                    />
                    "Active"
                </label>
            </div>

            <button
                type="submit"
                class="text-[14px] px-4 py-2 rounded-md bg-[#E76F00] text-white hover:bg-[#D96400] disabled:opacity-50"
                disabled=move || busy.get()
            >
                {move || if busy.get() { "Saving..." } else if mode == "edit" { "Save changes" } else { "Create card" }}
            </button>
        </form>
    }
}
