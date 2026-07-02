//! Lightweight install flow orientation — not a blocking wizard.

use crate::components::install_risk_gate::InstallRiskState;
use crate::public_install_guide::InstallPlatform;
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstallStep {
    Review,
    Client,
    Copy,
    Save,
}

/// Derive the current orientation step from shared install-flow signals.
pub fn current_install_step(
    risk_state: InstallRiskState,
    has_warning: bool,
    platform_interacted: bool,
    copy_revealed: bool,
    bookmarked: bool,
) -> InstallStep {
    if bookmarked {
        return InstallStep::Save;
    }
    if risk_state.copy_blocked {
        return InstallStep::Review;
    }
    if risk_state.high_risk_reveal_required && !copy_revealed {
        return InstallStep::Review;
    }
    if !platform_interacted && has_warning {
        return InstallStep::Review;
    }
    if risk_state.copy_allowed(copy_revealed) && platform_interacted {
        return InstallStep::Copy;
    }
    InstallStep::Client
}

#[component]
pub fn InstallProgressIndicator(
    #[allow(unused_variables)] platform: RwSignal<InstallPlatform>,
    risk_state: InstallRiskState,
    has_warning: Signal<bool>,
    platform_interacted: RwSignal<bool>,
    copy_revealed: RwSignal<bool>,
    #[prop(optional, default = Signal::derive(|| false))] bookmarked: Signal<bool>,
) -> impl IntoView {
    let current = move || {
        current_install_step(
            risk_state,
            has_warning.get(),
            platform_interacted.get(),
            copy_revealed.get(),
            bookmarked.get(),
        )
    };

    let steps = [
        (InstallStep::Review, "Review"),
        (InstallStep::Client, "Client"),
        (InstallStep::Copy, "Copy"),
        (InstallStep::Save, "Save"),
    ];

    view! {
        <ol class="install-progress" aria-label="Install steps">
            {steps.into_iter().map(|(step, label)| {
                let is_active = move || current() == step;
                let is_complete = move || current() > step;
                let is_blocked = move || {
                    step == InstallStep::Review
                        && is_active()
                        && risk_state.copy_blocked
                };
                view! {
                    <li
                        class=move || {
                            if is_blocked() {
                                "install-progress-step is-active is-blocked"
                            } else if is_active() {
                                "install-progress-step is-active"
                            } else if is_complete() {
                                "install-progress-step is-complete"
                            } else {
                                "install-progress-step"
                            }
                        }
                        aria-current=move || if is_active() { Some("step") } else { None }
                    >
                        {label}
                    </li>
                }
            }).collect_view()}
        </ol>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_risk_starts_on_client_until_platform_selected() {
        let risk = InstallRiskState::from_label("low");
        assert_eq!(
            current_install_step(risk, false, true, true, false),
            InstallStep::Copy
        );
        assert_eq!(
            current_install_step(risk, false, false, true, false),
            InstallStep::Client
        );
    }

    #[test]
    fn high_risk_stays_on_review_until_reveal() {
        let risk = InstallRiskState::from_label("high");
        assert_eq!(
            current_install_step(risk, true, false, false, false),
            InstallStep::Review
        );
        assert_eq!(
            current_install_step(risk, true, true, false, false),
            InstallStep::Review
        );
        assert_eq!(
            current_install_step(risk, true, true, true, false),
            InstallStep::Copy
        );
    }

    #[test]
    fn bookmarked_marks_save_step() {
        let risk = InstallRiskState::from_label("low");
        assert_eq!(
            current_install_step(risk, false, true, true, true),
            InstallStep::Save
        );
    }

    #[test]
    fn critical_risk_review_step_is_blocked_state() {
        let risk = InstallRiskState::from_label("critical");
        assert_eq!(
            current_install_step(risk, true, true, true, false),
            InstallStep::Review
        );
        assert!(risk.copy_blocked);
    }

    #[test]
    fn current_install_step_matrix() {
        let cases: [(&str, bool, bool, bool, bool, InstallStep); 12] = [
            // bookmarked wins
            ("low", false, false, false, true, InstallStep::Save),
            // high risk: review until reveal (even after platform pick)
            ("high", true, false, false, false, InstallStep::Review),
            ("high", true, true, false, false, InstallStep::Review),
            ("high", true, true, true, false, InstallStep::Copy),
            // medium warning before platform
            ("medium", true, false, true, false, InstallStep::Review),
            ("medium", true, true, true, false, InstallStep::Copy),
            // low: client until platform, then copy
            ("low", false, false, true, false, InstallStep::Client),
            ("low", false, true, true, false, InstallStep::Copy),
            // critical: copy blocked — stay on review
            ("critical", true, true, true, false, InstallStep::Review),
            ("critical", false, false, true, false, InstallStep::Review),
            // high without warning still review until reveal
            ("high", false, true, false, false, InstallStep::Review),
            // low: copy allowed without reveal gate once platform picked
            ("low", false, true, false, false, InstallStep::Copy),
        ];

        for (risk_label, has_warning, interacted, revealed, bookmarked, expected) in cases {
            let risk = InstallRiskState::from_label(risk_label);
            let actual = current_install_step(risk, has_warning, interacted, revealed, bookmarked);
            assert_eq!(
                actual, expected,
                "risk={risk_label} warning={has_warning} interacted={interacted} revealed={revealed} bookmarked={bookmarked}"
            );
        }
    }
}
