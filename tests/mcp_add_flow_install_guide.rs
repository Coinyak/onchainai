//! Integration-test binary — exercises the shipped `get_public_install_guide` server fn
//! with Leptos `Owner` + `provide_context`, same path as `InstallGuideRemoteLoader`.

#[cfg(feature = "ssr")]
mod ssr {
    use leptos::prelude::{provide_context, Owner};
    use onchainai::server::functions::get_public_install_guide;
    use onchainai::server::functions::server_fn_context_tests::{
        run_get_public_install_guide_server_fn_loads_approved_tool,
        run_get_public_install_guide_server_fn_returns_not_found_for_missing_slug,
        run_install_guide_panel_chain_matches_server_fn_for_approved_tool, skip_or_panic,
        test_pool,
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn get_public_install_guide_server_fn_loads_approved_tool() {
        run_get_public_install_guide_server_fn_loads_approved_tool().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_public_install_guide_server_fn_returns_not_found_for_missing_slug() {
        run_get_public_install_guide_server_fn_returns_not_found_for_missing_slug().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn install_guide_panel_chain_matches_server_fn_for_approved_tool() {
        run_install_guide_panel_chain_matches_server_fn_for_approved_tool().await;
    }

    /// Integration crate calls `get_public_install_guide` directly (not only via lib delegate).
    #[tokio::test(flavor = "multi_thread")]
    async fn integration_binary_invokes_get_public_install_guide_with_owner_context() {
        let pool = match test_pool().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("integration Owner context DB setup failed", err);
                return;
            }
        };

        let missing = format!("missing-integration-{}", uuid::Uuid::new_v4());
        let owner = Owner::new();
        let result = owner.with(|| {
            provide_context(pool);
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(get_public_install_guide(missing.clone(), "claude".into()))
            })
        });

        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("tool not found"),
            "integration binary must hit shipped server fn for {missing}"
        );
    }
}
