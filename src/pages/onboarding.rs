//! First-login profile onboarding — nickname + optional bio.

use crate::components::site_shell::SiteShell;
use crate::server::functions::get_current_user;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

#[component]
pub fn OnboardingProfilePage() -> impl IntoView {
    let query = use_query_map();
    let next = Memo::new(move |_| {
        query
            .with(|q| q.get("next").map(|s| s.to_string()))
            .filter(|s| s.starts_with('/') && !s.starts_with("//"))
            .unwrap_or_else(|| "/".into())
    });

    let user = Resource::new_blocking(|| (), |_| async move { get_current_user().await });

    view! {
        <SiteShell>
        <div class="max-w-[480px] px-4 py-12">
            <h1 class="text-[28px] font-bold mb-2">"Set up your profile"</h1>
            <p class="text-[#6B6B6B] text-[14px] mb-8 leading-relaxed">
                "Choose a nickname for comments and submissions. You can change it later."
            </p>
            {move || user.get().map(|res| match res {
                Ok(Some(session)) => {
                    let suggested = session.nickname.clone().unwrap_or_default();
                    let next_dest = next.get();
                    view! {
                        <form action="/onboarding/complete" method="post" class="space-y-4">
                            <input type="hidden" name="next" prop:value=move || next_dest.clone()/>
                            <label class="block text-[14px] font-medium mb-1">"Nickname"</label>
                            <input
                                type="text"
                                name="nickname"
                                class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px]"
                                placeholder="alice"
                                minlength="2"
                                maxlength="20"
                                pattern="[a-zA-Z0-9_-]+"
                                prop:value=move || suggested.clone()
                                required=true
                            />
                            <p class="text-[12px] text-[#6B6B6B]">"2–20 characters: letters, numbers, - or _"</p>
                            <label class="block text-[14px] font-medium mb-1 mt-4">"Bio (optional)"</label>
                            <textarea
                                name="bio"
                                class="w-full min-h-[80px] px-3 py-2 border border-[#E5E5E5] rounded-lg text-[14px]"
                                maxlength="200"
                                placeholder="What are you building?"
                            />
                            <div class="flex gap-3 pt-2">
                                <button
                                    type="submit"
                                    formaction="/onboarding/skip"
                                    formmethod="post"
                                    class="h-10 px-4 rounded-lg border border-[#E5E5E5] text-[14px] bg-white hover:bg-[#FAFAFA]"
                                >
                                    "Skip for now"
                                </button>
                                <button
                                    type="submit"
                                    class="h-10 px-4 rounded-lg bg-[#E76F00] text-white text-[14px] font-medium hover:bg-[#D96400]"
                                >
                                    "Save & Continue"
                                </button>
                            </div>
                        </form>
                    }
                    .into_any()
                }
                Ok(None) => view! {
                    <p class="text-[#6B6B6B]">"Sign in first."</p>
                    <a href="/login" class="text-[#E76F00] no-underline hover:underline">"Sign in"</a>
                }
                .into_any(),
                Err(_) => view! {
                    <p class="text-[#6B6B6B]">"Could not load session."</p>
                }
                .into_any(),
            })}
        </div>
        </SiteShell>
    }
}
