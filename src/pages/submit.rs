//! Public tool submission and claim page.

use crate::components::claim_status_timeline::{default_claim_steps, ClaimStatusTimeline};
use crate::components::login_modal::LoginModal;
use crate::components::site_shell::SiteShell;
use crate::server::functions::{
    get_current_user, list_my_submissions, request_tool_claim, submit_tool, ClaimToolInput,
    SubmitToolInput,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn SubmitPage() -> impl IntoView {
    let show_login = RwSignal::new(false);
    let status_msg = RwSignal::new(None::<String>);
    let error_msg = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let refresh = RwSignal::new(0u32);
    let claim_mode = RwSignal::new(false);

    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let tool_type = RwSignal::new("mcp".to_string());
    let function = RwSignal::new("dev-tool".to_string());
    let repo_url = RwSignal::new(String::new());
    let homepage = RwSignal::new(String::new());
    let npm_package = RwSignal::new(String::new());
    let mcp_endpoint = RwSignal::new(String::new());
    let install_command = RwSignal::new(String::new());
    let chains_raw = RwSignal::new(String::new());
    let category_suggestion = RwSignal::new(String::new());
    let official_team_claim = RwSignal::new(false);
    let verification_note = RwSignal::new(String::new());

    let claim_slug = RwSignal::new(String::new());
    let claim_email = RwSignal::new(String::new());
    let claim_team = RwSignal::new(String::new());
    let claim_github = RwSignal::new(String::new());
    let claim_website = RwSignal::new(String::new());
    let claim_x = RwSignal::new(String::new());
    let claim_proof_links = RwSignal::new(String::new());
    let claim_note = RwSignal::new(String::new());

    let user = Resource::new(|| (), |_| async move { get_current_user().await });
    let submissions = Resource::new(
        move || refresh.get(),
        |_| async move { list_my_submissions().await },
    );

    let on_submit_suggest = move |_| {
        error_msg.set(None);
        status_msg.set(None);
        busy.set(true);

        let input = SubmitToolInput {
            name: name.get_untracked(),
            description: description.get_untracked(),
            tool_type: tool_type.get_untracked(),
            function: function.get_untracked(),
            repo_url: opt_nonempty(repo_url.get_untracked()),
            homepage: opt_nonempty(homepage.get_untracked()),
            npm_package: opt_nonempty(npm_package.get_untracked()),
            mcp_endpoint: opt_nonempty(mcp_endpoint.get_untracked()),
            install_command: opt_nonempty(install_command.get_untracked()),
            chains_raw: chains_raw.get_untracked(),
            category_suggestion: opt_nonempty(category_suggestion.get_untracked()),
            official_team_claim: official_team_claim.get_untracked(),
            verification_note: opt_nonempty(verification_note.get_untracked()),
        };

        spawn_local(async move {
            match get_current_user().await {
                Ok(Some(_)) => match submit_tool(input).await {
                    Ok(row) => {
                        status_msg.set(Some(format!(
                            "Submission received (status: {}). Operators will review before any public listing.",
                            row.status
                        )));
                        refresh.update(|n| *n = n.wrapping_add(1));
                    }
                    Err(e) => error_msg.set(Some(e.to_string())),
                },
                Ok(None) => show_login.set(true),
                Err(e) => error_msg.set(Some(e.to_string())),
            }
            busy.set(false);
        });
    };

    let on_submit_claim = move |_| {
        error_msg.set(None);
        status_msg.set(None);
        busy.set(true);

        let proof_links: Vec<String> = claim_proof_links
            .get_untracked()
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();

        let input = ClaimToolInput {
            slug: claim_slug.get_untracked(),
            verification_note: claim_note.get_untracked(),
            contact_email: opt_nonempty(claim_email.get_untracked()),
            team_name: opt_nonempty(claim_team.get_untracked()),
            github_url: opt_nonempty(claim_github.get_untracked()),
            website_url: opt_nonempty(claim_website.get_untracked()),
            x_url: opt_nonempty(claim_x.get_untracked()),
            proof_links,
        };

        spawn_local(async move {
            match get_current_user().await {
                Ok(Some(_)) => match request_tool_claim(input).await {
                    Ok(_) => {
                        status_msg.set(Some(
                            "Claim submitted. Operators will verify your official links independently."
                                .into(),
                        ));
                    }
                    Err(e) => error_msg.set(Some(e.to_string())),
                },
                Ok(None) => show_login.set(true),
                Err(e) => error_msg.set(Some(e.to_string())),
            }
            busy.set(false);
        });
    };

    view! {
        <SiteShell>
        <LoginModal show=show_login/>
        <div class="max-w-[720px] px-4 py-8">
            <h1 class="text-[28px] font-bold mb-2">"Submit or Claim"</h1>
            <p class="text-[#6B6B6B] text-[14px] leading-relaxed mb-6">
                "Suggest a new tool for review, or claim an existing listing with proof of official ownership."
            </p>

            <div class="flex gap-2 mb-8">
                <button
                    type="button"
                    class=move || if !claim_mode.get() {
                        "px-4 py-2 rounded-lg text-[14px] font-medium bg-[#1A1A1A] text-white"
                    } else {
                        "px-4 py-2 rounded-lg text-[14px] font-medium border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                    }
                    on:click=move |_| claim_mode.set(false)
                >
                    "Suggest a tool"
                </button>
                <button
                    type="button"
                    class=move || if claim_mode.get() {
                        "px-4 py-2 rounded-lg text-[14px] font-medium bg-[#1A1A1A] text-white"
                    } else {
                        "px-4 py-2 rounded-lg text-[14px] font-medium border border-[#E5E5E5] hover:bg-[#FAFAFA]"
                    }
                    on:click=move |_| claim_mode.set(true)
                >
                    "Claim this tool"
                </button>
            </div>

            <Suspense fallback=|| ()>
                {move || user.get().map(|res| match res {
                    Ok(None) => view! {
                        <div class="rounded-lg border border-[#E5E5E5] bg-[#FAFAFA] p-4 mb-6">
                            <p class="text-[14px] text-[#6B6B6B] mb-3">"Sign in to submit or claim."</p>
                            <button
                                type="button"
                                class="h-10 px-4 rounded-lg bg-[#E76F00] text-white text-[14px] font-medium hover:bg-[#D96400]"
                                on:click=move |_| show_login.set(true)
                            >
                                "Sign in"
                            </button>
                        </div>
                    }.into_any(),
                    Ok(Some(_)) => {
                        if claim_mode.get() {
                            view! {
                                <ClaimStatusTimeline steps=default_claim_steps() active_index=0/>
                                <div class="space-y-4">
                                    <label class="block text-[14px] font-medium">"Tool slug (existing listing)"</label>
                                    <input type="text" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px] font-mono"
                                        placeholder="bridge-mcp"
                                        prop:value=move || claim_slug.get()
                                        on:input=move |ev| claim_slug.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"Team or company name"</label>
                                    <input type="text" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        prop:value=move || claim_team.get()
                                        on:input=move |ev| claim_team.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"Official email"</label>
                                    <input type="email" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        prop:value=move || claim_email.get()
                                        on:input=move |ev| claim_email.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"Official GitHub URL"</label>
                                    <input type="url" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        placeholder="https://github.com/org/repo"
                                        prop:value=move || claim_github.get()
                                        on:input=move |ev| claim_github.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"Official website URL"</label>
                                    <input type="url" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        prop:value=move || claim_website.get()
                                        on:input=move |ev| claim_website.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"Official X profile URL"</label>
                                    <input type="url" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        placeholder="https://x.com/handle"
                                        prop:value=move || claim_x.get()
                                        on:input=move |ev| claim_x.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"Proof note (required)"</label>
                                    <textarea class="w-full min-h-[100px] px-3 py-2 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        placeholder="Describe how operators can verify ownership (site backlink, docs reference, repo file proof, etc.)"
                                        prop:value=move || claim_note.get()
                                        on:input=move |ev| claim_note.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"Proof links (one per line, optional)"</label>
                                    <textarea class="w-full min-h-[72px] px-3 py-2 border border-[#E5E5E5] rounded-lg text-[14px] font-mono"
                                        placeholder="https://example.com/docs/onchainai\nhttps://github.com/org/repo/blob/main/OWNERS"
                                        prop:value=move || claim_proof_links.get()
                                        on:input=move |ev| claim_proof_links.set(event_target_value(&ev))
                                    />

                                    {move || error_msg.get().map(|m| view! {
                                        <p class="text-[14px] text-[#C0392B]" role="alert">{m}</p>
                                    })}
                                    {move || status_msg.get().map(|m| view! {
                                        <p class="text-[14px] text-[#1A7F4B]" role="status">{m}</p>
                                    })}

                                    <button
                                        type="button"
                                        class="h-10 px-4 rounded-lg bg-[#E76F00] text-white text-[14px] font-medium hover:bg-[#D96400] disabled:opacity-50"
                                        disabled=move || busy.get()
                                        on:click=on_submit_claim
                                    >
                                        "Submit claim with proof"
                                    </button>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="space-y-4">
                                    <label class="block text-[14px] font-medium">"Tool name"</label>
                                    <input type="text" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        placeholder="Bridge MCP"
                                        prop:value=move || name.get()
                                        on:input=move |ev| name.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"Description"</label>
                                    <textarea class="w-full min-h-[100px] px-3 py-2 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        prop:value=move || description.get()
                                        on:input=move |ev| description.set(event_target_value(&ev))
                                    />

                                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                        <div>
                                            <label class="block text-[14px] font-medium mb-1">"Type"</label>
                                            <select class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px] bg-white"
                                                prop:value=move || tool_type.get()
                                                on:change=move |ev| tool_type.set(event_target_value(&ev))
                                            >
                                                <option value="mcp">"MCP"</option>
                                                <option value="cli">"CLI"</option>
                                                <option value="sdk">"SDK"</option>
                                                <option value="api">"API"</option>
                                                <option value="skill">"Skill"</option>
                                                <option value="x402">"x402"</option>
                                            </select>
                                        </div>
                                        <div>
                                            <label class="block text-[14px] font-medium mb-1">"Function"</label>
                                            <select class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px] bg-white"
                                                prop:value=move || function.get()
                                                on:change=move |ev| function.set(event_target_value(&ev))
                                            >
                                                <option value="dev-tool">"Dev tool"</option>
                                                <option value="bridge">"Bridge"</option>
                                                <option value="swap">"Swap"</option>
                                                <option value="wallet">"Wallet"</option>
                                                <option value="payments">"Payments"</option>
                                                <option value="data">"Data"</option>
                                                <option value="ai-agent">"AI agent"</option>
                                            </select>
                                        </div>
                                    </div>

                                    <label class="block text-[14px] font-medium">"Repo URL"</label>
                                    <input type="url" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        prop:value=move || repo_url.get()
                                        on:input=move |ev| repo_url.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"Homepage"</label>
                                    <input type="url" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        prop:value=move || homepage.get()
                                        on:input=move |ev| homepage.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"npm package"</label>
                                    <input type="text" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        prop:value=move || npm_package.get()
                                        on:input=move |ev| npm_package.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"Install command"</label>
                                    <input type="text" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px] font-mono"
                                        prop:value=move || install_command.get()
                                        on:input=move |ev| install_command.set(event_target_value(&ev))
                                    />

                                    <label class="block text-[14px] font-medium">"Supported chains"</label>
                                    <input type="text" class="w-full h-10 px-3 border border-[#E5E5E5] rounded-lg text-[14px]"
                                        prop:value=move || chains_raw.get()
                                        on:input=move |ev| chains_raw.set(event_target_value(&ev))
                                    />

                                    <label class="flex items-center gap-2 text-[14px]">
                                        <input type="checkbox" class="rounded border-[#E5E5E5]"
                                            prop:checked=move || official_team_claim.get()
                                            on:change=move |ev| official_team_claim.set(event_target_checked(&ev))
                                        />
                                        "I represent the official project team"
                                    </label>

                                    {official_team_claim.get().then(|| view! {
                                        <label class="block text-[14px] font-medium">"Verification note"</label>
                                        <textarea class="w-full min-h-[72px] px-3 py-2 border border-[#E5E5E5] rounded-lg text-[14px]"
                                            prop:value=move || verification_note.get()
                                            on:input=move |ev| verification_note.set(event_target_value(&ev))
                                        />
                                    })}

                                    {move || error_msg.get().map(|m| view! {
                                        <p class="text-[14px] text-[#C0392B]" role="alert">{m}</p>
                                    })}
                                    {move || status_msg.get().map(|m| view! {
                                        <p class="text-[14px] text-[#1A7F4B]" role="status">{m}</p>
                                    })}

                                    <button
                                        type="button"
                                        class="h-10 px-4 rounded-lg bg-[#E76F00] text-white text-[14px] font-medium hover:bg-[#D96400] disabled:opacity-50"
                                        disabled=move || busy.get()
                                        on:click=on_submit_suggest
                                    >
                                        "Submit for review"
                                    </button>
                                </div>
                            }.into_any()
                        }
                    },
                    Err(e) => view! {
                        <p class="text-[14px] text-[#C0392B]">{e.to_string()}</p>
                    }.into_any(),
                })}
            </Suspense>

            <section class="mt-10 border-t border-[#E5E5E5] pt-8">
                <h2 class="text-[20px] font-semibold mb-4">"Your submissions"</h2>
                <Suspense fallback=|| view! {
                    <p class="text-[14px] text-[#6B6B6B]">"Loading..."</p>
                }>
                    {move || match submissions.get() {
                        Some(Ok(rows)) if rows.is_empty() => view! {
                            <p class="text-[14px] text-[#6B6B6B]">"No submissions yet."</p>
                        }.into_any(),
                        Some(Ok(rows)) => view! {
                            <ul class="space-y-3">
                                {rows.into_iter().map(|row| {
                                    let name = row.payload.get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Untitled")
                                        .to_string();
                                    let slug = row.payload.get("slug")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let status = row.status.clone();
                                    view! {
                                        <li class="border border-[#E5E5E5] rounded-lg p-4">
                                            <div class="flex items-center justify-between gap-3 flex-wrap">
                                                <span class="font-medium text-[14px]">{name}</span>
                                                <span class="text-[12px] px-2 py-0.5 rounded bg-[#F5F5F0] border border-[#E5E5E5]">{status}</span>
                                            </div>
                                            <p class="text-[12px] text-[#6B6B6B] mt-1">{"slug: "}{slug}</p>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any(),
                        Some(Err(_)) => view! {
                            <p class="text-[14px] text-[#6B6B6B]">"Sign in to view your submissions."</p>
                        }.into_any(),
                        None => ().into_any(),
                    }}
                </Suspense>
            </section>
        </div>
        </SiteShell>
    }
}

fn opt_nonempty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
