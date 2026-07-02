//! Risk gating for install copy — shared contract for UI surfaces.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstallRiskState {
    pub risk_level: &'static str,
    pub copy_blocked: bool,
    pub high_risk_reveal_required: bool,
}

impl InstallRiskState {
    pub fn from_label(risk_level: &str) -> Self {
        Self {
            risk_level: match risk_level {
                "low" => "low",
                "medium" => "medium",
                "high" => "high",
                "critical" => "critical",
                _ => "medium",
            },
            copy_blocked: risk_level == "critical",
            high_risk_reveal_required: risk_level == "high",
        }
    }

    pub fn copy_allowed(&self, copy_revealed: bool) -> bool {
        !self.copy_blocked && (!self.high_risk_reveal_required || copy_revealed)
    }
}
