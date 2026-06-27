//! Admin site settings — slogan, MCP endpoint, crawler keywords, registration flags.

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

    admin_page_shell(move || {
        view! {
            <div class="px-4 md:px-6 py-8 max-w-[720px] mx-auto">
                <div class="mb-6">
                    <h1 class="text-[20px] font-semibold tracking-tight">"Site Settings"</h1>
                    <p class="text-[#6B6B6B] text-[14px] mt-1">
                        "Update public copy, MCP endpoint, crawler keywords, and registration rules."
                    </p>
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
        }
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
    let default_referral_bps = RwSignal::new(
        initial
            .default_referral_bps
            .map(|v| v.to_string())
            .unwrap_or_default(),
    );
    let default_referral_payout_address =
        RwSignal::new(initial.default_referral_payout_address.unwrap_or_default());
    let x402_builder_code = RwSignal::new(initial.x402_builder_code.unwrap_or_default());

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

        let bps_text = default_referral_bps.get_untracked();
        let default_bps = if bps_text.trim().is_empty() {
            None
        } else {
            match bps_text.trim().parse::<i32>() {
                Ok(value) => Some(value),
                Err(_) => {
                    saving.set(false);
                    save_error.set(Some("Default referral bps must be numeric.".into()));
                    return;
                }
            }
        };

        let payload = (
            site_name.get_untracked(),
            slogan.get_untracked(),
            description.get_untracked(),
            mcp_endpoint.get_untracked(),
            keywords_text.get_untracked(),
            allow_free_registration.get_untracked(),
            require_tool_approval.get_untracked(),
            allow_x402_registration.get_untracked(),
            default_bps,
            default_referral_payout_address.get_untracked(),
            x402_builder_code.get_untracked(),
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
                default_referral_bps: payload.8,
                default_referral_payout_address: Some(payload.9).filter(|s| !s.trim().is_empty()),
                x402_builder_code: Some(payload.10).filter(|s| !s.trim().is_empty()),
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
                    default_referral_bps.set(
                        updated
                            .default_referral_bps
                            .map(|value| value.to_string())
                            .unwrap_or_default(),
                    );
                    default_referral_payout_address
                        .set(updated.default_referral_payout_address.unwrap_or_default());
                    x402_builder_code.set(updated.x402_builder_code.unwrap_or_default());
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

            <fieldset class="space-y-3 rounded-lg border border-[#E5E5E5] px-4 py-3">
                <legend class="text-[14px] font-medium px-1">"x402 Referral"</legend>
                <label class="block">
                    <span class="text-[14px] font-medium">"Default referral bps"</span>
                    <input
                        class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px] font-mono"
                        inputmode="numeric"
                        placeholder="250"
                        prop:value=move || default_referral_bps.get()
                        on:input=move |ev| default_referral_bps.set(event_target_value(&ev))
                    />
                </label>
                <label class="block">
                    <span class="text-[14px] font-medium">"Default payout address"</span>
                    <input
                        class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px] font-mono"
                        placeholder="0x..."
                        prop:value=move || default_referral_payout_address.get()
                        on:input=move |ev| default_referral_payout_address.set(event_target_value(&ev))
                    />
                </label>
                <label class="block">
                    <span class="text-[14px] font-medium">"Builder code"</span>
                    <input
                        class="mt-1 w-full rounded-lg border border-[#E5E5E5] px-3 py-2 text-[14px] font-mono"
                        placeholder="onchainai"
                        prop:value=move || x402_builder_code.get()
                        on:input=move |ev| x402_builder_code.set(event_target_value(&ev))
                    />
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
