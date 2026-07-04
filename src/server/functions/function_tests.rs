use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_list_request_serializes_filters_field() {
        let req = ToolListRequest {
            sort: "hot".into(),
            offset: 0,
            limit: 50,
            filters: ToolFilters {
                function: vec!["bridge".into()],
                ..Default::default()
            },
            query: Some("mcp".into()),
        };
        let json = serde_json::to_value(&req).expect("serialize request");
        assert!(json.get("filters").is_some());
        assert_eq!(json["sort"], "hot");

        let round_trip: ToolListRequest =
            serde_json::from_value(json).expect("deserialize request");
        assert_eq!(round_trip.sort, "hot");
        assert_eq!(round_trip.limit, 50);
        assert_eq!(round_trip.filters.function, vec!["bridge"]);
        assert_eq!(round_trip.query.as_deref(), Some("mcp"));
    }

    #[test]
    fn tool_filters_deserialize_partial_wasm_payload() {
        let json = serde_json::json!({
            "function": ["bridge"]
        });
        let filters: ToolFilters = serde_json::from_value(json).expect("partial filters");
        assert_eq!(filters.function, vec!["bridge"]);
        assert!(filters.asset_class.is_empty());
        assert!(filters.pricing.is_empty());
        assert!(filters.chain.is_empty());
    }

    #[test]
    fn load_browser_data_request_deserialize_partial_filters() {
        let json = serde_json::json!({
            "sort": "hot",
            "filters": { "function": ["bridge"] },
            "page": 1
        });
        let req: LoadBrowserDataRequest = serde_json::from_value(json).expect("partial request");
        assert_eq!(req.sort, "hot");
        assert_eq!(req.filters.function, vec!["bridge"]);
        assert_eq!(req.page, 1);
    }

    #[test]
    fn list_tools_limit_uses_max_cap_not_legacy_100() {
        assert_eq!(clamp_list_tools_limit(100), 100);
        assert_eq!(clamp_list_tools_limit(150), 150);
        assert_eq!(clamp_list_tools_limit(500), MAX_LIST_TOOLS_LIMIT);
        assert_eq!(clamp_list_tools_limit(501), MAX_LIST_TOOLS_LIMIT);
        assert_eq!(clamp_list_tools_limit(0), 1);
    }

    #[test]
    fn browser_visible_limit_page_two_is_cumulative_100() {
        assert_eq!(browser_visible_limit_for_page(2), 100);
        assert_eq!(browser_visible_limit_for_page(1), 50);
        assert_eq!(browser_visible_limit_for_page(0), 50);
    }

    #[test]
    fn clamp_browser_page_param_bounds_window() {
        assert_eq!(clamp_browser_page_param(0), 1);
        assert_eq!(clamp_browser_page_param(2), 2);
        assert_eq!(clamp_browser_page_param(99), 10);
    }

    #[test]
    fn tool_list_request_limit_500_accepted() {
        let req = ToolListRequest {
            sort: "hot".into(),
            offset: 0,
            limit: MAX_LIST_TOOLS_LIMIT,
            filters: ToolFilters::default(),
            query: None,
        };
        assert!(validate_tool_list_request(&req).is_ok());
    }

    #[test]
    fn tool_list_request_limit_501_rejected() {
        let req = ToolListRequest {
            sort: "hot".into(),
            offset: 0,
            limit: MAX_LIST_TOOLS_LIMIT + 1,
            filters: ToolFilters::default(),
            query: None,
        };
        let err = validate_tool_list_request(&req).expect_err("limit 501 should fail");
        assert!(err.to_string().contains("limit must be between 1 and 500"));
    }

    #[test]
    fn tool_list_request_rejects_invalid_sort() {
        let req = ToolListRequest {
            sort: "random".into(),
            offset: 0,
            limit: 50,
            filters: ToolFilters::default(),
            query: None,
        };
        let err = validate_tool_list_request(&req).expect_err("invalid sort should fail");
        assert!(err.to_string().contains("sort must be one of"));
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn append_tool_filters_uses_query_builder_binds() {
        let mut query = sqlx::QueryBuilder::new("SELECT * FROM tools WHERE true");
        let filters = ToolFilters {
            function: vec!["bridge".into(), "swap".into()],
            pricing: vec!["x402".into()],
            ..Default::default()
        };
        append_tool_filters(&mut query, &filters);
        assert!(query.sql().contains("function = ANY($1)"));
        assert!(query.sql().contains("pricing = ANY($2)"));
    }

    #[test]
    fn list_tools_comments_sort_uses_comment_count() {
        let order = list_tools_order_clause("comments");
        assert!(order.contains("comments cm"));
        assert!(order.contains("COUNT(*)"));
    }

    #[test]
    fn dashboard_snapshot_limit_is_bounded_for_public_surfaces() {
        assert_eq!(clamp_dashboard_list_limit(0), 1);
        assert_eq!(clamp_dashboard_list_limit(6), 6);
        assert_eq!(clamp_dashboard_list_limit(99), MAX_DASHBOARD_LIST_LIMIT);
    }

    #[test]
    fn dashboard_bucket_links_target_existing_public_filters() {
        assert_eq!(
            dashboard_filter_href("function", "payments"),
            "/tools?function=payments"
        );
        assert_eq!(dashboard_filter_href("type", "mcp"), "/tools?type=mcp");
        assert_eq!(dashboard_filter_href("chain", "base"), "/tools?chain=base");
        assert_eq!(
            dashboard_filter_href("status", "official"),
            "/tools?status=official"
        );
        assert_eq!(
            dashboard_filter_href("pricing", "x402"),
            "/tools?pricing=x402"
        );
    }

    #[test]
    fn toolkit_export_payload_redacts_sensitive_payment_addresses() {
        let mut tool = sample_review_tool();
        tool.approval_status = "approved".into();
        tool.relevance_status = "accepted".into();
        tool.status = "official".into();
        tool.pricing = "x402".into();
        tool.x402_price = Some("$0.01".into());
        tool.install_command = Some("npx bridge-mcp".into());
        tool.referral_payout_address = Some("0xoperatorpayout".into());
        tool.x402_pay_to_address = Some("0xproviderpayto".into());
        tool.submitted_by = Some(Uuid::new_v4());

        let payload =
            build_toolkit_payload(vec![ToolkitToolView::from_tool(tool)]).expect("toolkit payload");

        assert_eq!(payload.total, 1);
        assert_eq!(payload.tools[0].referral_payout_address, None);
        assert_eq!(payload.tools[0].x402_pay_to_address, None);
        assert!(payload.markdown_export.body.contains("Bridge MCP"));
        assert!(payload.markdown_export.body.contains("npx bridge-mcp"));
        assert!(!payload.markdown_export.body.contains("0xoperatorpayout"));
        assert!(!payload.json_export.body.contains("0xproviderpayto"));
        assert!(
            !payload.json_export.body.contains("submitted_by"),
            "JSON export must omit internal fields"
        );
        assert!(
            !payload.json_export.body.contains("approval_status"),
            "JSON export must omit operator fields"
        );
    }

    #[test]
    fn toolkit_export_includes_user_notes_and_tags() {
        let mut tool = sample_review_tool();
        tool.approval_status = "approved".into();
        tool.relevance_status = "accepted".into();
        tool.install_command = Some("npx bridge-mcp".into());
        let item = ToolkitToolView {
            tool,
            note: Some("Use for Base bridge research".into()),
            tags: vec!["base".into(), "research".into()],
            source: "web".into(),
            source_client: None,
            saved_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let payload = build_toolkit_payload(vec![item]).expect("toolkit payload");

        assert_eq!(
            payload.items[0].note.as_deref(),
            Some("Use for Base bridge research")
        );
        assert_eq!(payload.items[0].tags, vec!["base", "research"]);
        assert!(payload
            .markdown_export
            .body
            .contains("- Note: Use for Base bridge research"));
        assert!(payload
            .markdown_export
            .body
            .contains("- Tags: base, research"));
        assert!(payload.json_export.body.contains("\"note\""));
        assert!(payload.json_export.body.contains("\"tags\""));
    }

    fn sample_review_tool() -> Tool {
        let review = crate::models::tool::default_review_fields();
        Tool {
            id: Uuid::nil(),
            name: "Bridge MCP".into(),
            slug: "bridge-mcp".into(),
            description: Some("Ethereum bridge tool".into()),
            function: "bridge".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "mcp".into(),
            repo_url: Some("https://github.com/example/bridge".into()),
            homepage: None,
            npm_package: None,
            install_command: None,
            mcp_endpoint: None,
            chains: vec![],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "pending".into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: review.crypto_relevance_score,
            crypto_relevance_reasons: review.crypto_relevance_reasons,
            relevance_status: review.relevance_status,
            install_risk_level: review.install_risk_level,
            install_risk_reasons: review.install_risk_reasons,
            requires_secret: review.requires_secret,
            safe_copy_command: review.safe_copy_command,
            quarantined_at: review.quarantined_at,
            last_reviewed_at: review.last_reviewed_at,
            review_policy_version: review.review_policy_version,
            claim_state: "unclaimed".into(),
            license: None,
            pricing: "free".into(),
            x402_price: None,
            referral_enabled: false,
            referral_bps: None,
            referral_payout_address: None,
            referral_model: None,
            x402_pay_to_address: None,
            x402_builder_code: None,
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
            x402_endpoint: None,
            x402_last_checked_at: None,
            x402_check_failures: 0,
            stars: 0,
            last_commit_at: None,
            source: "github".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn review_approval_gate_requires_trustworthy_url() {
        let mut tool = sample_review_tool();
        tool.repo_url = None;
        assert_eq!(
            validate_review_approval_gate(&tool, None),
            Err("approval requires a repo, homepage, npm package, or MCP endpoint")
        );
    }

    #[test]
    fn review_approval_gate_allows_needs_review_with_url() {
        let tool = sample_review_tool();
        assert!(validate_review_approval_gate(&tool, None).is_ok());
    }

    #[test]
    fn review_approval_gate_requires_override_for_rejected_relevance() {
        let mut tool = sample_review_tool();
        tool.relevance_status = "rejected".into();
        assert_eq!(
            validate_review_approval_gate(&tool, None),
            Err("override reason required when overriding rejected relevance or critical install risk")
        );
        assert!(validate_review_approval_gate(&tool, Some("operator override")).is_ok());
    }

    #[test]
    fn review_approval_gate_requires_override_for_critical_install() {
        let mut tool = sample_review_tool();
        tool.install_risk_level = "critical".into();
        assert_eq!(
            validate_review_approval_gate(&tool, None),
            Err("override reason required when overriding rejected relevance or critical install risk")
        );
    }

    #[test]
    fn review_override_required_detects_rejected_and_critical() {
        let mut tool = sample_review_tool();
        assert!(!review_override_required(&tool));
        tool.relevance_status = "rejected".into();
        assert!(review_override_required(&tool));
        tool.relevance_status = "accepted".into();
        tool.install_risk_level = "critical".into();
        assert!(review_override_required(&tool));
    }

    #[test]
    fn tool_has_trustworthy_url_accepts_repo_or_npm() {
        let mut tool = sample_review_tool();
        assert!(tool_has_trustworthy_url(&tool));
        tool.repo_url = None;
        tool.npm_package = Some("@example/pkg".into());
        assert!(tool_has_trustworthy_url(&tool));
    }

    #[test]
    fn set_tool_approval_validation_accepts_approved_and_pending() {
        assert!(validate_set_tool_approval_input("approved", None).is_ok());
        assert!(validate_set_tool_approval_input("pending", None).is_ok());
    }

    #[test]
    fn set_tool_approval_validation_rejects_without_reason() {
        assert_eq!(
            validate_set_tool_approval_input("rejected", None),
            Err("rejection requires a non-empty reason")
        );
        assert_eq!(
            validate_set_tool_approval_input("rejected", Some("   ")),
            Err("rejection requires a non-empty reason")
        );
    }

    #[test]
    fn set_tool_approval_validation_rejects_invalid_status() {
        assert!(validate_set_tool_approval_input("published", None).is_err());
    }

    #[test]
    fn list_pending_tools_sql_filters_pending_only() {
        assert!(LIST_PENDING_TOOLS_SQL.contains("approval_status = 'pending'"));
        assert!(!LIST_PENDING_TOOLS_SQL.contains("approved"));
    }

    #[test]
    fn admin_review_limit_is_clamped() {
        assert_eq!(clamp_admin_review_list_limit(0), 1);
        assert_eq!(clamp_admin_review_list_limit(25), 25);
        assert_eq!(
            clamp_admin_review_list_limit(10_000),
            MAX_ADMIN_REVIEW_LIST_LIMIT
        );
    }

    #[test]
    fn review_queue_where_covers_all_queues() {
        for queue in REVIEW_QUEUES {
            assert!(
                review_queue_where(queue).is_ok(),
                "missing where for {queue}"
            );
        }
        assert_eq!(review_queue_where("unknown"), Err("unknown review queue"));
    }

    #[test]
    fn review_queue_sql_covers_all_queues_without_runtime_formatting() {
        for queue in REVIEW_QUEUES {
            let sql = review_queue_sql(queue).expect("queue sql");
            assert!(sql.starts_with("SELECT * FROM tools"));
            assert!(sql.contains("LIMIT $1"));
            assert!(!sql.contains("{}"));
        }
        assert_eq!(review_queue_sql("unknown"), Err("unknown review queue"));
    }

    #[test]
    fn derive_lifecycle_state_maps_pending_and_quarantine() {
        let mut tool = sample_review_tool();
        assert_eq!(derive_lifecycle_state(&tool), "candidate");
        tool.last_reviewed_at = Some(chrono::Utc::now());
        assert_eq!(derive_lifecycle_state(&tool), "pending");
        tool.quarantined_at = Some(chrono::Utc::now());
        assert_eq!(derive_lifecycle_state(&tool), "flagged");
    }

    #[test]
    fn derive_claim_state_reads_tool_column() {
        let mut tool = sample_review_tool();
        assert_eq!(derive_claim_state(&tool), "unclaimed");
        tool.claim_state = "claim_pending".into();
        assert_eq!(derive_claim_state(&tool), "claim_pending");
    }

    fn sample_submit_input() -> SubmitToolInput {
        SubmitToolInput {
            name: "Bridge MCP".into(),
            description: "Ethereum bridge MCP server for crypto agents.".into(),
            tool_type: "mcp".into(),
            function: "bridge".into(),
            repo_url: Some("https://github.com/example/bridge".into()),
            homepage: None,
            npm_package: None,
            mcp_endpoint: None,
            install_command: Some("npm i @example/bridge-mcp".into()),
            chains_raw: "ethereum, arbitrum".into(),
            category_suggestion: None,
            official_team_claim: false,
            verification_note: None,
        }
    }

    #[test]
    fn validate_submit_tool_accepts_minimally_plausible_crypto_tool() {
        assert!(validate_submit_tool_input(&sample_submit_input()).is_ok());
    }

    #[test]
    fn validate_submit_tool_rejects_without_link() {
        let mut input = sample_submit_input();
        input.repo_url = None;
        assert!(validate_submit_tool_input(&input).is_err());
    }

    #[test]
    fn validate_submit_tool_rejects_short_description() {
        let mut input = sample_submit_input();
        input.description = "too short".into();
        assert!(validate_submit_tool_input(&input).is_err());
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn scan_submission_runs_relevance_and_install_scanners() {
        let scan = scan_submission(&sample_submit_input());
        assert!(scan.crypto_relevance_score > 0);
        assert!(!scan.relevance_status.is_empty());
        assert_eq!(scan.install_risk_level, "low");
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn scan_submission_accepts_low_relevance_intake() {
        let mut input = sample_submit_input();
        input.name = "Generic Helper".into();
        input.description = "A generic helper tool without crypto terms.".into();
        input.repo_url = Some("https://example.com".into());
        let scan = scan_submission(&input);
        assert!(scan.relevance_status == "needs_review" || scan.relevance_status == "rejected");
        assert!(validate_submit_tool_input(&input).is_ok());
    }

    #[test]
    fn validate_report_reason_accepts_allowlist() {
        assert!(validate_report_reason("scam_phishing").is_ok());
        assert!(validate_report_reason("broken_link").is_ok());
        assert!(validate_report_reason("invalid").is_err());
    }

    #[test]
    fn validate_claim_tool_input_bounds() {
        assert!(validate_claim_tool_input(&ClaimToolInput {
            slug: "bridge-mcp".into(),
            verification_note: "I maintain this repo and can verify via DNS TXT.".into(),
            contact_email: Some("team@example.com".into()),
            team_name: None,
            github_url: None,
            website_url: None,
            x_url: None,
            proof_links: vec![],
        })
        .is_ok());
        assert!(validate_claim_tool_input(&ClaimToolInput {
            slug: "bridge-mcp".into(),
            verification_note: "short".into(),
            contact_email: None,
            team_name: None,
            github_url: None,
            website_url: None,
            x_url: None,
            proof_links: vec![],
        })
        .is_err());
    }

    #[test]
    fn validate_claim_tool_input_requires_verification_note() {
        let input = ClaimToolInput {
            slug: "bob-gateway-cli".into(),
            contact_email: Some("team@gobob.xyz".into()),
            verification_note: "".into(),
            team_name: None,
            github_url: None,
            website_url: None,
            x_url: None,
            proof_links: vec![],
        };
        let err =
            validate_claim_tool_input(&input).expect_err("empty verification note should fail");
        assert!(err.contains("verification note"));
    }

    #[test]
    fn validate_claim_proof_urls_reject_non_http_links() {
        let urls = vec!["javascript:alert(1)".to_string()];
        let err = validate_claim_proof_urls(&urls).expect_err("unsafe links should fail");
        assert!(err.contains("https"));
    }

    #[test]
    fn validate_claim_proof_urls_rejects_localhost_prefix_spoofing() {
        let urls = vec!["http://localhost.evil.example/proof".to_string()];
        let err = validate_claim_proof_urls(&urls).expect_err("spoofed localhost should fail");
        assert!(err.contains("https"));
    }

    #[test]
    fn validate_claim_tool_input_rejects_too_many_proof_links() {
        let links: Vec<String> = (0..11)
            .map(|i| format!("https://example.com/{i}"))
            .collect();
        let err = validate_claim_tool_input(&ClaimToolInput {
            slug: "bridge-mcp".into(),
            verification_note: "I maintain this repo and can verify via DNS TXT.".into(),
            contact_email: None,
            team_name: None,
            github_url: None,
            website_url: None,
            x_url: None,
            proof_links: links,
        })
        .expect_err("too many proof links should fail");
        assert!(err.contains("10 proof links"));
    }

    #[test]
    fn build_claim_proof_note_enforces_total_size_cap() {
        let links: Vec<String> = (0..10)
            .map(|i| format!("https://example.com/proof-path-segment-{i:03}/extra"))
            .collect();
        let err = build_claim_proof_note(&ClaimToolInput {
            slug: "bridge-mcp".into(),
            verification_note: "x".repeat(3500),
            contact_email: None,
            team_name: Some("A very long team name that pushes the note over the limit".into()),
            github_url: None,
            website_url: None,
            x_url: None,
            proof_links: links,
        })
        .expect_err("oversized formatted note should fail");
        assert!(err.contains("4000"));
    }

    #[test]
    fn review_queue_where_reported_uses_open_reports() {
        let where_clause = review_queue_where("reported").expect("reported queue");
        assert!(where_clause.contains("tool_reports"));
        assert!(where_clause.contains("status = 'open'"));
    }

    #[test]
    fn validate_review_action_accepts_operator_actions() {
        assert!(validate_review_action("needs_info", "more context").is_ok());
        assert!(validate_review_action("quarantine", "unsafe install").is_ok());
        assert!(validate_review_action("mark_verified", "checked repo").is_ok());
        assert!(validate_review_action("mark_official", "official domain").is_ok());
        assert!(validate_review_action("demote_verified", "trust revoked").is_ok());
        assert!(validate_review_action("demote_official", "badge revoked").is_ok());
        assert!(validate_review_action("needs_info", "   ").is_err());
        assert!(validate_review_action("demote_verified", "   ").is_err());
    }

    #[test]
    fn review_audit_statuses_tracks_trust_and_quarantine() {
        let tool = sample_review_tool();
        assert_eq!(
            review_audit_statuses(&tool, "mark_verified"),
            ("community".into(), "verified".into())
        );
        assert_eq!(
            review_audit_statuses(&tool, "mark_official"),
            ("community".into(), "official".into())
        );
        assert_eq!(
            review_audit_statuses(&tool, "demote_verified"),
            ("community".into(), "community".into())
        );
        assert_eq!(
            review_audit_statuses(&tool, "needs_info"),
            ("pending".into(), "needs_info".into())
        );

        let mut verified = sample_review_tool();
        verified.status = "verified".into();
        assert_eq!(
            review_audit_statuses(&verified, "demote_verified"),
            ("verified".into(), "community".into())
        );

        let mut official = sample_review_tool();
        official.status = "official".into();
        assert_eq!(
            review_audit_statuses(&official, "demote_official"),
            ("official".into(), "community".into())
        );
    }

    #[test]
    fn parse_search_keywords_splits_commas_and_newlines() {
        assert_eq!(
            parse_search_keywords("mcp-server, crypto-mcp\nweb3-mcp"),
            vec![
                "mcp-server".to_string(),
                "crypto-mcp".to_string(),
                "web3-mcp".to_string()
            ]
        );
    }

    #[test]
    fn validate_site_settings_accepts_defaults() {
        let keywords = vec!["mcp-server".into()];
        assert!(
            validate_update_site_settings_input(SiteSettingsValidationInput {
                site_name: "OnchainAI",
                slogan: "Crypto tools, unified.",
                description: "Discover tools.",
                mcp_endpoint: "npx mcp-remote www.onchain-ai.xyz/mcp",
                search_keywords: &keywords,
                default_referral_bps: Some(250),
                default_referral_payout_address: Some("0x0000000000000000000000000000000000000000"),
                x402_builder_code: Some("onchainai"),
                hero_title: Some("Hero"),
                hero_subtitle: None,
                about_content: None,
                footer_links: &[],
            })
            .is_ok()
        );
    }

    #[test]
    fn validate_site_settings_rejects_empty_keywords() {
        assert!(
            validate_update_site_settings_input(SiteSettingsValidationInput {
                site_name: "OnchainAI",
                slogan: "Slogan",
                description: "Description here.",
                mcp_endpoint: "npx mcp-remote",
                search_keywords: &[],
                default_referral_bps: None,
                default_referral_payout_address: None,
                x402_builder_code: None,
                hero_title: None,
                hero_subtitle: None,
                about_content: None,
                footer_links: &[],
            })
            .is_err()
        );
    }

    #[test]
    fn validate_site_settings_rejects_invalid_keyword_chars() {
        let keywords = vec!["bad keyword".into()];
        assert!(
            validate_update_site_settings_input(SiteSettingsValidationInput {
                site_name: "OnchainAI",
                slogan: "Slogan",
                description: "Description here.",
                mcp_endpoint: "npx mcp-remote",
                search_keywords: &keywords,
                default_referral_bps: None,
                default_referral_payout_address: None,
                x402_builder_code: None,
                hero_title: None,
                hero_subtitle: None,
                about_content: None,
                footer_links: &[],
            })
            .is_err()
        );
    }

    #[test]
    fn validate_site_settings_rejects_invalid_footer_link_url() {
        let keywords = vec!["mcp-server".into()];
        let footer_links = vec![crate::models::FooterLink {
            label: "Docs".into(),
            url: "ftp://bad.example".into(),
        }];
        assert!(
            validate_update_site_settings_input(SiteSettingsValidationInput {
                site_name: "OnchainAI",
                slogan: "Slogan",
                description: "Description here.",
                mcp_endpoint: "npx mcp-remote",
                search_keywords: &keywords,
                default_referral_bps: None,
                default_referral_payout_address: None,
                x402_builder_code: None,
                hero_title: None,
                hero_subtitle: None,
                about_content: None,
                footer_links: &footer_links,
            })
            .is_err()
        );
    }

    #[test]
    fn validate_crawler_schedule_accepts_hourly_default() {
        assert!(validate_update_crawler_source(60).is_ok());
    }

    #[test]
    fn validate_crawler_schedule_rejects_too_short() {
        assert!(validate_update_crawler_source(1).is_err());
    }

    #[test]
    fn format_schedule_minutes_renders_hours() {
        assert_eq!(format_schedule_minutes(360), "Every 6h");
    }

    #[test]
    fn validate_tool_referral_payload_allows_unverified_x402_referral() {
        assert!(validate_tool_referral_payload(&UpdateToolReferralPayload {
            slug: "paid-tool".into(),
            referral_enabled: true,
            referral_bps: Some(250),
            referral_payout_address: Some("0x0000000000000000000000000000000000000000".into()),
            referral_model: Some("attribution".into()),
            x402_pay_to_address: Some("0x1111111111111111111111111111111111111111".into()),
            x402_builder_code: Some("onchainai".into()),
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
            x402_endpoint: Some("https://pay.example.com/probe".into()),
        })
        .is_ok());
    }

    #[test]
    fn validate_tool_referral_payload_rejects_bad_x402_endpoint() {
        let payload = UpdateToolReferralPayload {
            slug: "paid-tool".into(),
            referral_enabled: false,
            referral_bps: None,
            referral_payout_address: None,
            referral_model: None,
            x402_pay_to_address: None,
            x402_builder_code: None,
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
            x402_endpoint: Some("http://insecure.example/pay".into()),
        };
        assert!(validate_tool_referral_payload(&payload).is_err());
    }

    #[test]
    fn validate_tool_referral_payload_rejects_bad_bps_and_model() {
        let mut payload = UpdateToolReferralPayload {
            slug: "paid-tool".into(),
            referral_enabled: true,
            referral_bps: Some(10_001),
            referral_payout_address: None,
            referral_model: Some("mystery".into()),
            x402_pay_to_address: None,
            x402_builder_code: None,
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
            x402_endpoint: None,
        };
        assert!(validate_tool_referral_payload(&payload).is_err());
        payload.referral_bps = Some(100);
        assert!(validate_tool_referral_payload(&payload).is_err());
        payload.referral_model = Some("split".into());
        assert!(validate_tool_referral_payload(&payload).is_ok());
    }

    #[test]
    fn validate_trigger_crawler_source_accepts_known_sources() {
        assert!(validate_trigger_crawler_source("npm").is_ok());
        assert!(validate_trigger_crawler_source("sync_stars").is_ok());
    }

    #[test]
    fn admin_review_queue_redacts_secrets_in_tool_json() {
        use crate::server::secret_redaction::assert_json_has_no_secrets;

        let review_fields = crate::models::tool::default_review_fields();
        let tool = Tool {
            id: Uuid::new_v4(),
            name: "Leak test".into(),
            slug: "leak-test".into(),
            description: Some("SUPABASE_SERVICE_KEY=leaked-service-key".into()),
            function: "bridge".into(),
            asset_class: "multi".into(),
            actor: "agent".into(),
            tool_type: "mcp".into(),
            repo_url: None,
            homepage: None,
            npm_package: None,
            install_command: Some("GITHUB_CLIENT_SECRET=leaked-client-secret".into()),
            mcp_endpoint: None,
            chains: vec![],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "pending".into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: 0,
            crypto_relevance_reasons: vec![],
            relevance_status: "needs_review".into(),
            install_risk_level: "low".into(),
            install_risk_reasons: vec![],
            requires_secret: false,
            safe_copy_command: None,
            quarantined_at: None,
            last_reviewed_at: None,
            review_policy_version: review_fields.review_policy_version,
            claim_state: "unclaimed".into(),
            license: None,
            pricing: "free".into(),
            x402_price: None,
            referral_enabled: false,
            referral_bps: None,
            referral_payout_address: None,
            referral_model: None,
            x402_pay_to_address: None,
            x402_builder_code: None,
            payment_verified: false,
            x402_endpoint_verified: false,
            price_verified: false,
            x402_endpoint: None,
            x402_last_checked_at: None,
            x402_check_failures: 0,
            stars: 0,
            last_commit_at: None,
            source: "manual".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let item = ReviewQueueItem {
            tool: redact_tool_for_admin(tool),
            duplicate_candidates: vec![],
            lifecycle_state: "candidate".into(),
            claim_state: "unclaimed".into(),
        };
        let json = serde_json::to_string(&item).expect("serialize");
        assert_json_has_no_secrets(&json);
        assert!(!json.contains("leaked-service-key"));
        assert!(!json.contains("leaked-client-secret"));
    }

    #[test]
    fn validate_trigger_crawler_source_rejects_unknown() {
        assert!(validate_trigger_crawler_source("unknown").is_err());
    }

    #[test]
    fn validate_comment_content_bounds() {
        assert!(validate_comment_content("hello").is_ok());
        assert!(validate_comment_content("").is_err());
        assert!(validate_comment_content(&"x".repeat(2001)).is_err());
    }

    #[test]
    fn toggle_sql_uses_advisory_lock_for_same_target_and_user() {
        assert!(TOGGLE_UPVOTE_SQL.contains("pg_advisory_xact_lock"));
        assert!(TOGGLE_UPVOTE_SQL.contains("upvote:"));
        assert!(TOGGLE_BOOKMARK_SQL.contains("pg_advisory_xact_lock"));
        assert!(TOGGLE_BOOKMARK_SQL.contains("bookmark:"));
    }

    #[test]
    fn validate_category_input_accepts_slug_id() {
        assert!(validate_category_input(&CategoryInput {
            id: "my-cat".into(),
            label: "My Category".into(),
            icon: "git-branch".into(),
            description: "A test category.".into(),
            sort_order: 10,
        })
        .is_ok());
    }

    #[test]
    fn validate_category_input_rejects_uppercase_id() {
        assert!(validate_category_input(&CategoryInput {
            id: "Bad-ID".into(),
            label: "Label".into(),
            icon: "icon".into(),
            description: "Description.".into(),
            sort_order: 1,
        })
        .is_err());
    }

    #[test]
    fn validate_featured_image_upload_accepts_png_within_limit() {
        assert!(validate_featured_image_upload("image/png", 1024).is_ok());
    }

    #[test]
    fn validate_featured_image_upload_rejects_oversized_and_bad_type() {
        assert!(validate_featured_image_upload("image/png", MAX_FEATURED_IMAGE_BYTES + 1).is_err());
        assert!(validate_featured_image_upload("application/pdf", 100).is_err());
        assert!(validate_featured_image_upload("image/png", 0).is_err());
    }

    #[test]
    fn validate_featured_card_input_bounds() {
        assert!(validate_featured_card_input(
            "https://cdn.example/card.png",
            Some("Headline"),
            None
        )
        .is_ok());
        assert!(validate_featured_card_input("   ", None, None).is_err());
        assert!(validate_featured_card_input(
            "https://cdn.example/card.png",
            Some(&"x".repeat(121)),
            None
        )
        .is_err());
    }

    #[test]
    fn featured_queries_require_full_public_visibility_gate() {
        for sql in [
            GET_FEATURED_CARDS_SQL,
            SEARCH_TOOLS_FOR_PICKER_SQL,
            FEATURED_TOOL_EXISTS_SQL,
        ] {
            assert!(sql.contains("approval_status = 'approved'"));
            assert!(sql.contains("relevance_status = 'accepted'"));
            assert!(sql.contains("install_risk_level <> 'critical'"));
            assert!(sql.contains("quarantined_at IS NULL"));
        }
    }

    #[test]
    fn select_active_featured_cards_orders_by_sort_order() {
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let mut cards = vec![
            FeaturedCardView {
                id: id_b,
                tool_id: Uuid::new_v4(),
                tool_slug: "b".into(),
                tool_name: "B".into(),
                image_url: "https://cdn.example/b.png".into(),
                headline: None,
                subtitle: None,
                sort_order: 2,
            },
            FeaturedCardView {
                id: id_a,
                tool_id: Uuid::new_v4(),
                tool_slug: "a".into(),
                tool_name: "A".into(),
                image_url: "https://cdn.example/a.png".into(),
                headline: None,
                subtitle: None,
                sort_order: 1,
            },
        ];
        let ordered = select_active_featured_cards(&mut cards);
        assert_eq!(ordered[0].tool_slug, "a");
        assert_eq!(ordered[1].tool_slug, "b");
    }
}
