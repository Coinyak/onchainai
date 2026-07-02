//! Global OnchainAI MCP connect card — distinct from per-tool Add MCP.

use crate::components::copy_button::CopyButton;
use crate::components::highlighted_command::HighlightedCommand;
use crate::public_install_guide::{
    build_onchainai_connect_guide, copy_label_aria, InstallPlatform,
};
use leptos::prelude::*;

fn platform_label(platform: InstallPlatform) -> &'static str {
    match platform {
        InstallPlatform::Claude => "Claude",
        InstallPlatform::Cursor => "Cursor",
        InstallPlatform::GenericMcp => "Generic MCP",
        InstallPlatform::CliSdk => "CLI/SDK",
    }
}

fn display_text(guide: &crate::public_install_guide::PublicInstallGuide) -> String {
    guide
        .copy_text
        .clone()
        .or(guide.config_json.clone())
        .or(guide.command.clone())
        .unwrap_or_default()
}

#[component]
pub fn ConnectOnchainAiMcpCard(mcp_endpoint: String) -> impl IntoView {
    let platform = RwSignal::new(InstallPlatform::default_connect_platform());
    let endpoint = StoredValue::new(mcp_endpoint);

    let guide = move || build_onchainai_connect_guide(platform.get(), &endpoint.get_value());
    let copy_text = move || display_text(&guide());
    let copy_aria = move || copy_label_aria(&guide().copy_label);

    view! {
        <div class="promo-card border border-[#E5E5E5] rounded-lg p-6 bg-white min-w-0" data-testid="connect-onchainai-mcp-card">
            <h3 class="text-[16px] font-semibold mb-2">"Connect OnchainAI MCP"</h3>
            <p class="text-[14px] text-[#6B6B6B] mb-3 leading-relaxed">
                "Let your agent search OnchainAI for crypto tools."
            </p>
            <div class="install-platform-group connect-mcp-platforms" role="group" aria-label="Connect OnchainAI MCP client">
                {InstallPlatform::connect_card_platforms().into_iter().map(|value| {
                    let label = platform_label(value);
                    view! {
                        <button
                            type="button"
                            class=move || {
                                if platform.get() == value {
                                    "install-platform-btn active"
                                } else {
                                    "install-platform-btn"
                                }
                            }
                            aria-pressed=move || if platform.get() == value { "true" } else { "false" }
                            on:click=move |_| platform.set(value)
                        >
                            {label}
                        </button>
                    }
                }).collect_view()}
            </div>
            <div class="flex items-center gap-2 min-w-0 mt-3">
                <HighlightedCommand text=copy_text() show_prefix=false/>
                <CopyButton text=copy_text() label=copy_aria()/>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public_install_guide::InstallPlatform;

    #[test]
    fn connect_card_defaults_to_claude() {
        assert_eq!(
            InstallPlatform::default_connect_platform(),
            InstallPlatform::Claude
        );
    }

    #[test]
    fn connect_card_lists_spec_platforms_only() {
        assert_eq!(InstallPlatform::connect_card_platforms().len(), 3);
        assert!(InstallPlatform::connect_card_platforms().contains(&InstallPlatform::Claude));
        assert!(!InstallPlatform::connect_card_platforms().contains(&InstallPlatform::CliSdk));
    }

    #[test]
    fn connect_guide_copy_label_follows_platform() {
        let cmd = "npx mcp-remote www.onchain-ai.xyz/mcp";
        let claude = build_onchainai_connect_guide(InstallPlatform::Claude, cmd);
        let generic = build_onchainai_connect_guide(InstallPlatform::GenericMcp, cmd);
        assert_eq!(claude.copy_label, "Copy config");
        assert_eq!(generic.copy_label, "Copy command");
        assert_eq!(copy_label_aria(&claude.copy_label), "Copy config");
    }
}
