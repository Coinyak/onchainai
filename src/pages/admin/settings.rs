//! Admin site settings — slogan, MCP endpoint, crawler keywords, registration flags.

use crate::components::top_nav::TopNav;
use crate::models::SiteSettings;
use crate::pages::admin::admin_page_shell;
use crate::server::functions::{
    get_site_settings, update_site_settings, UpdateSiteSettingsPayload,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn AdminSettingsPage() -> impl IntoView {
    let settings = Resource::new(|| (), |_| async move { get_site_settings().await });

    admin_page_shell(move || view! {
        <TopNav/>
        <div class="px-4 md:px-6 py-8 max-w-[720px] mx-auto">
            <div class="flex items-baseline justify-between gap-4 mb-6">
                <div>
                    <h1 class="text-[20px] font-semibold tracking-tight">"Site Settings"</h1>
                    <p class="text-[#6B6B6B] text-[14px] mt-1">
                        "Update public copy, MCP endpoint, crawler keywords, and registration rules."
                    </p>
                </div>
                <a href="/admin" class="text-[14px] text-[#E76F00] hover:underline">"Admin home"</a>
            </div>

            <Suspense fallback=|| view! {
                <p class="text-[#6B6B6B] text-[14px]">"Loading settings..."</p>
            }>
                {move || match settings.get() {
                    Some(Ok(initial)) => view! {
                        <AdminSettingsForm initial=initial/>
                    }.into_any(),
                    Some(Err(e)) => view! {
                        <p class="text-[14px] text-[#C0392B]">
                            "Failed to load settings: " {e.to_string()}
                        </p>
                    }.into_any(),
                    None => ().into_any(),
                }}
            </Suspense>
        </div>
    })
}

#[component]
fn AdminSettingsForm(initial: SiteSettings) -> impl IntoView {
    let site_name = RwSignal::new(initial.site_name);
    let slogan = RwSignal::new(initial.slogan);
    let description = RwSignal::new(initial.description);
    let mcp_endpoint = RwSignal::new(initial.mcp_endpoint);
    let keywords_text = RwSignal::new(initial.search_keywords.join(", "));
    let allow_free_registration = RwSignal::new(initial.allow_free_registration);
    let require_tool_approval = RwSignal::new(initial.require_tool_approval);
    let allow_x402_registration = RwSignal::new(initial.allow_x402_registration);

    let save_error = RwSignal::new(None::<String>);
    let save_ok = RwSignal::new(false);
    let saving = RwSignal::new(false);

    let on_save = move |_| {
        if saving.get_untracked() {
            return;
        }
        saving.set(true);
        save_error.set(None);
        save_ok.set(false);

        let payload = (
            site_name.get_untracked(),
            slogan.get_untracked(),
            description.get_untracked(),
            mcp_endpoint.get_untracked(),
            keywords_text.get_untracked(),
            allow_free_registration.get_untracked(),
            require_tool_approval.get_untracked(),
            allow_x402_registration.get_untracked(),
        );

        spawn_local(async move {
            let result = update_site_settings(UpdateSiteSettingsPayload {
                site_name: payload.0,
                slogan: payload.1,
                description: payload.2,
                mcp_endpoint: payload.3,
                search_keywords_raw: payload.4,
                allow_free_registration: payload.5,
                require_tool_approval: payload.6,
                allow_x402_registration: payload.7,
            })
            .await;
            saving.set(false);
            match result {
                Ok(updated) => {
                    save_ok.set(true);
                    site_name.set(updated.site_name);
                    slogan.set(updated.slogan);
                    description.set(updated.description);
                    mcp_endpoint.set(updated.mcp_endpoint);
                    keywords_text.set(updated.search_keywords.join(", "));
                    allow_free_registration.set(updated.allow_free_registration);
                    require_tool_approval.set(updated.require_tool_approval);
                    allow_x402_registration.set(updated.allow_x402_registration);
                }
                Err(e) => save_error.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <form class="space-y-5" on:submit=move |ev| {
            ev.prevent_default();
            on_save(());
        }>
            <label class="block">
                <span class="text-[14px] font-medium">"Site name"</span>
                <input
                    class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px]"
                    prop:value=move || site_name.get()
                    on:input=move |ev| site_name.set(event_target_value(&ev))
                />
            </label>

            <label class="block">
                <span class="text-[14px] font-medium">"Slogan"</span>
                <input
                    class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px]"
                    prop:value=move || slogan.get()
                    on:input=move |ev| slogan.set(event_target_value(&ev))
                />
            </label>

            <label class="block">
                <span class="text-[14px] font-medium">"Description"</span>
                <textarea
                    class="mt-1 w-full min-h-[96px] rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px]"
                    prop:value=move || description.get()
                    on:input=move |ev| description.set(event_target_value(&ev))
                />
            </label>

            <label class="block">
                <span class="text-[14px] font-medium">"MCP endpoint"</span>
                <input
                    class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px] font-mono"
                    prop:value=move || mcp_endpoint.get()
                    on:input=move |ev| mcp_endpoint.set(event_target_value(&ev))
                />
            </label>

            <label class="block">
                <span class="text-[14px] font-medium">"Search keywords (crawler)"</span>
                <p class="text-[12px] text-[#6B6B6B] mt-0.5 mb-1">
                    "Comma-separated GitHub topic keywords. Used when crawling new tools."
                </p>
                <textarea
                    class="w-full min-h-[72px] rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px] font-mono"
                    placeholder="mcp-server, crypto-mcp, web3-mcp"
                    prop:value=move || keywords_text.get()
                    on:input=move |ev| keywords_text.set(event_target_value(&ev))
                />
            </label>

            <fieldset class="space-y-3 rounded-lg border border-[#E5E5E5] px-4 py-3">
                <legend class="text-[14px] font-medium px-1">"Registration"</legend>
                <label class="flex items-center gap-2 text-[14px]">
                    <input
                        type="checkbox"
                        prop:checked=move || allow_free_registration.get()
                        on:change=move |ev| {
                            allow_free_registration.set(event_target_checked(&ev));
                        }
                    />
                    "Allow free registration"
                </label>
                <label class="flex items-center gap-2 text-[14px]">
                    <input
                        type="checkbox"
                        prop:checked=move || require_tool_approval.get()
                        on:change=move |ev| {
                            require_tool_approval.set(event_target_checked(&ev));
                        }
                    />
                    "Require approval for new tools"
                </label>
                <label class="flex items-center gap-2 text-[14px]">
                    <input
                        type="checkbox"
                        prop:checked=move || allow_x402_registration.get()
                        on:change=move |ev| {
                            allow_x402_registration.set(event_target_checked(&ev));
                        }
                    />
                    "Allow x402 paid registration"
                </label>
            </fieldset>

            {move || save_error.get().map(|msg| view! {
                <p class="text-[14px] text-[#C0392B]">{msg}</p>
            })}

            {move || save_ok.get().then(|| view! {
                <p class="text-[14px] text-[#2D7D46]">"Settings saved."</p>
            })}

            <button
                type="submit"
                class="px-4 py-2 rounded-lg bg-[#1A1A1A] text-white text-[14px] font-medium hover:opacity-90 disabled:opacity-50"
                disabled=move || saving.get()
            >
                {move || if saving.get() { "Saving..." } else { "Save settings" }}
            </button>
        </form>
    }
}