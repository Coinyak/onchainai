//! Integration-test binary — exercises `fetch_public_install_guide` directly.

#[cfg(feature = "ssr")]
mod ssr {
    use onchainai::server::functions::fetch_public_install_guide;
    use onchainai::server::functions::server_fn_context_tests::{
        run_get_public_install_guide_server_fn_loads_approved_tool,
        run_get_public_install_guide_server_fn_returns_not_found_for_missing_slug,
        run_install_guide_panel_chain_matches_server_fn_for_approved_tool, skip_or_panic,
        test_pool,
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn fetch_public_install_guide_loads_approved_tool() {
        run_get_public_install_guide_server_fn_loads_approved_tool().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fetch_public_install_guide_returns_not_found_for_missing_slug() {
        run_get_public_install_guide_server_fn_returns_not_found_for_missing_slug().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn install_guide_panel_chain_matches_fetch_for_approved_tool() {
        run_install_guide_panel_chain_matches_server_fn_for_approved_tool().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn integration_binary_invokes_fetch_public_install_guide_directly() {
        let pool = match test_pool().await {
            Ok(value) => value,
            Err(err) => {
                skip_or_panic("integration fetch DB setup failed", err);
                return;
            }
        };

        let missing = format!("missing-integration-{}", uuid::Uuid::new_v4());
        let result = fetch_public_install_guide(&pool, &missing, "claude").await;

        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("tool not found"),
            "integration binary must hit fetch_public_install_guide for {missing}"
        );
    }
}
