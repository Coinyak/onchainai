//! Secret redaction for operator harness and admin API responses.
//!
//! Ensures env secrets, tokens, and full `.env` assignments never leave the server.

use crate::models::{Comment, Tool};

/// Known secret env var names that must never appear in API output.
pub const SECRET_ENV_NAMES: &[&str] = &[
    "SUPABASE_SERVICE_KEY",
    "JWT_SECRET",
    "GITHUB_CLIENT_SECRET",
    "GITHUB_API_TOKEN",
    "RAILWAY_TOKEN",
    "GODADDY_API_KEY",
    "GODADDY_API_SECRET",
    "DATABASE_URL",
    "SUPABASE_ANON_KEY",
];

/// Token-like prefixes redacted from arbitrary strings.
pub const SECRET_PREFIXES: &[&str] = &[
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "sk-",
    "xoxb-",
    "xoxp-",
    "sb_secret_",
    "sb_publishable_",
];

/// Case-insensitive substring search; returns the byte offset of the first match.
/// ASCII lowercasing preserves byte length and UTF-8 boundaries, so the offset
/// found in the lowercased copy applies unchanged to the original string.
fn find_ignore_ascii_case(haystack: &str, pattern: &str) -> Option<usize> {
    if pattern.is_empty() {
        return None;
    }
    haystack
        .to_ascii_lowercase()
        .find(&pattern.to_ascii_lowercase())
}

/// Redact secrets from text before it reaches clients or Hermes.
pub fn redact_secrets(input: &str) -> String {
    let mut out = input.to_string();

    for name in SECRET_ENV_NAMES {
        for pattern in [format!("{name}="), format!("{name} =")] {
            let mut search_from = 0;
            while let Some(rel) = find_ignore_ascii_case(&out[search_from..], &pattern) {
                let start = search_from + rel;
                let mut value_start = start + pattern.len();
                while out[value_start..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_whitespace())
                {
                    value_start += 1;
                }
                let rest = &out[value_start..];
                let end = rest
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '\n')
                    .unwrap_or(rest.len());
                out.replace_range(value_start..value_start + end, "[REDACTED]");
                search_from = value_start + "[REDACTED]".len();
            }
        }
        while let Some(rel) = find_ignore_ascii_case(&out, name) {
            out.replace_range(rel..rel + name.len(), "[REDACTED]");
        }
    }

    for prefix in SECRET_PREFIXES {
        while let Some(idx) = out.find(prefix) {
            let rest = &out[idx..];
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '\n')
                .unwrap_or(rest.len());
            out.replace_range(idx..idx + end, "[REDACTED_TOKEN]");
        }
    }

    out
}

/// Returns true when `text` still contains known secret material after redaction.
pub fn contains_secret_material(text: &str) -> bool {
    let redacted = redact_secrets(text);
    for name in SECRET_ENV_NAMES {
        if text.contains(name) && redacted.contains(name) {
            return true;
        }
    }
    for prefix in SECRET_PREFIXES {
        if text.contains(prefix) && redacted.contains(prefix) {
            return true;
        }
    }
    false
}

/// Assert JSON payloads do not leak secret values (for tests and response guards).
pub fn assert_json_has_no_secrets(json: &str) {
    for name in SECRET_ENV_NAMES {
        assert!(
            !json.contains(name),
            "JSON must not contain secret env name `{name}`"
        );
    }
    let patterns = [
        "postgresql://user:pass",
        "super-secret-key",
        "also-leaked",
        "ghp_abcdefghijklmnopqrstuvwxyz",
    ];
    for pattern in patterns {
        assert!(
            !json.contains(pattern),
            "JSON must not contain secret pattern `{pattern}`"
        );
    }
}

/// Redact user-controlled string fields on a tool before admin API serialization.
pub fn redact_tool_for_admin(mut tool: Tool) -> Tool {
    tool.name = redact_secrets(&tool.name);
    tool.description = tool.description.map(|d| redact_secrets(&d));
    tool.repo_url = tool.repo_url.map(|u| redact_secrets(&u));
    tool.homepage = tool.homepage.map(|u| redact_secrets(&u));
    tool.install_command = tool.install_command.map(|c| redact_secrets(&c));
    tool.mcp_endpoint = tool.mcp_endpoint.map(|u| redact_secrets(&u));
    tool.source_url = tool.source_url.map(|u| redact_secrets(&u));
    tool.rejection_reason = tool.rejection_reason.map(|r| redact_secrets(&r));
    tool.safe_copy_command = tool.safe_copy_command.map(|c| redact_secrets(&c));
    tool.crypto_relevance_reasons = tool
        .crypto_relevance_reasons
        .into_iter()
        .map(|r| redact_secrets(&r))
        .collect();
    tool.install_risk_reasons = tool
        .install_risk_reasons
        .into_iter()
        .map(|r| redact_secrets(&r))
        .collect();
    tool.referral_payout_address = tool.referral_payout_address.map(|a| redact_secrets(&a));
    tool.x402_pay_to_address = tool.x402_pay_to_address.map(|a| redact_secrets(&a));
    tool.x402_builder_code = tool.x402_builder_code.map(|c| redact_secrets(&c));
    tool
}

/// Redact comment body before admin API serialization.
pub fn redact_comment_for_admin(mut comment: Comment) -> Comment {
    comment.content = redact_secrets(&comment.content);
    comment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_secrets_masks_env_assignments_and_tokens() {
        let input = "JWT_SECRET=super-secret-key GITHUB_CLIENT_SECRET=abc SUPABASE_SERVICE_KEY=xyz token ghp_abcdefghijklmnopqrstuvwxyz";
        let out = redact_secrets(input);
        assert!(!out.contains("super-secret-key"));
        assert!(!out.contains("ghp_abcdefghijklmnopqrstuvwxyz"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn redact_secrets_masks_deploy_and_supabase_prefix_tokens() {
        let input =
            "GITHUB_API_TOKEN=ghp_leak RAILWAY_TOKEN=rw_leak sb_secret_abc123 sb_publishable_xyz";
        let out = redact_secrets(input);
        assert!(!out.contains("ghp_leak"));
        assert!(!out.contains("rw_leak"));
        assert!(!out.contains("sb_secret_abc123"));
        assert!(!out.contains("sb_publishable_xyz"));
    }

    #[test]
    fn redact_secrets_masks_lowercase_and_mixed_case_env_names() {
        let input = "jwt_secret=super-secret-value Jwt_Secret=other-secret database_url=postgresql://user:pass@host/db";
        let out = redact_secrets(input);
        assert!(!out.contains("super-secret-value"));
        assert!(!out.contains("other-secret"));
        assert!(!out.contains("postgresql://user:pass"));
    }

    #[test]
    fn redact_secrets_masks_prefix_tokens_longer_than_forty_chars() {
        let input = "sb_secret_abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJmore-secret-tail-data";
        let out = redact_secrets(input);
        assert!(!out.contains("more-secret-tail-data"));
        assert!(out.contains("[REDACTED_TOKEN]"));
    }

    #[test]
    fn redact_secrets_masks_multiple_ghp_tokens() {
        let input = "ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ghp_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let out = redact_secrets(input);
        assert!(!out.contains("ghp_aaaaaaaa"));
        assert!(!out.contains("ghp_bbbbbbbb"));
    }

    #[test]
    fn redact_secrets_masks_dotenv_style_lines() {
        let input = "DATABASE_URL=postgresql://user:pass@db.xxx.supabase.co:5432/postgres\nJWT_SECRET = my-jwt-value";
        let out = redact_secrets(input);
        assert!(!out.contains("postgresql://user:pass"));
        assert!(!out.contains("my-jwt-value"));
    }

    #[test]
    fn assert_json_has_no_secrets_catches_leaks() {
        let safe = r#"{"note":"[REDACTED] configured"}"#;
        assert_json_has_no_secrets(safe);
    }

    #[test]
    fn redact_tool_for_admin_sanitizes_description() {
        let review_fields = crate::models::tool::default_review_fields();
        let tool = Tool {
            id: uuid::Uuid::new_v4(),
            name: "Test".into(),
            slug: "test".into(),
            description: Some("leak JWT_SECRET=hidden-value".into()),
            function: "bridge".into(),
            asset_class: "multi".into(),
            actor: "agent".into(),
            tool_type: "mcp".into(),
            repo_url: None,
            homepage: None,
            npm_package: None,
            install_command: Some("GITHUB_CLIENT_SECRET=oops".into()),
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
        let redacted = redact_tool_for_admin(tool);
        let json = serde_json::to_string(&redacted).expect("serialize");
        assert_json_has_no_secrets(&json);
        assert!(!json.contains("hidden-value"));
        assert!(!json.contains("oops"));
    }
}
