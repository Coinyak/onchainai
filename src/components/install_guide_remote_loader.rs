//! Remote install-guide I/O — owns `Resource` + `get_public_install_guide` and feeds the panel.

use crate::components::install_guide_panel::{
    resolve_install_guide, InstallGuidePanel, InstallGuideSource,
};
use crate::components::install_risk_gate::InstallRiskState;
use crate::models::Tool;
use crate::public_install_guide::{build_public_install_guide, InstallPlatform};
use crate::server::functions::get_public_install_guide;
use leptos::prelude::*;

fn default_platform_for_tool(tool: &Tool) -> InstallPlatform {
    if tool.tool_type == "cli" || tool.tool_type == "sdk" || tool.tool_type == "api" {
        InstallPlatform::CliSdk
    } else {
        InstallPlatform::GenericMcp
    }
}

/// Loads install guide data from the shipped server fn and renders `InstallGuidePanel`.
#[component]
pub fn InstallGuideRemoteLoader(
    tool: Tool,
    #[prop(optional)] initial_platform: Option<InstallPlatform>,
    #[prop(optional)] compact: bool,
    #[prop(optional)] source: InstallGuideSource,
    #[prop(optional)] show_progress: bool,
    #[prop(optional, default = Signal::derive(|| false))] bookmarked: Signal<bool>,
) -> impl IntoView {
    let slug = tool.slug.clone();
    let slug_local = slug.clone();
    let tool_local = tool.clone();
    let platform =
        RwSignal::new(initial_platform.unwrap_or_else(|| default_platform_for_tool(&tool)));
    let platform_interacted = RwSignal::new(false);
    let risk_state = InstallRiskState::from_label(&tool.install_risk_level);
    let copy_revealed = RwSignal::new(!risk_state.high_risk_reveal_required);

    let local_guide =
        Memo::new(move |_| build_public_install_guide(&tool_local, &slug_local, platform.get()));

    let remote_guide = Resource::new(
        move || (slug.clone(), platform.get()),
        |(slug, plat)| async move {
            get_public_install_guide(slug, plat.server_param().to_string()).await
        },
    );

    let guide = Memo::new(move |_| resolve_install_guide(remote_guide.get(), local_guide.get()));

    view! {
        <InstallGuidePanel
            tool=tool
            guide=guide
            platform=platform
            platform_interacted=platform_interacted
            risk_state=risk_state
            copy_revealed=copy_revealed
            compact=compact
            source=source
            show_progress=show_progress
            bookmarked=bookmarked
        />
    }
}

#[cfg(test)]
mod loader_tests {
    #[tokio::test(flavor = "multi_thread")]
    async fn install_guide_remote_loader_chain_matches_server_fn() {
        use crate::server::functions::server_fn_context_tests::run_install_guide_panel_chain_matches_server_fn_for_approved_tool;
        run_install_guide_panel_chain_matches_server_fn_for_approved_tool().await;
    }
}
