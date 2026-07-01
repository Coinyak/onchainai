//! Compact guided finder for public discovery.

use crate::discovery::{tool_finder_href, FinderSafety, ToolFinderAnswers};
use leptos::prelude::*;

#[derive(Clone, Copy)]
struct FinderOption {
    label: &'static str,
    value: &'static str,
}

const FUNCTIONS: &[FinderOption] = &[
    FinderOption {
        label: "Wallet / Portfolio",
        value: "wallet-portfolio",
    },
    FinderOption {
        label: "Trading / Swap",
        value: "trading-swap",
    },
    FinderOption {
        label: "Bridge",
        value: "bridge",
    },
    FinderOption {
        label: "Data / Indexing",
        value: "data-indexing",
    },
    FinderOption {
        label: "Payments / x402",
        value: "payments-x402",
    },
    FinderOption {
        label: "AI Agent / MCP",
        value: "ai-agent",
    },
];

const CHAINS: &[FinderOption] = &[
    FinderOption {
        label: "All chains",
        value: "",
    },
    FinderOption {
        label: "Bitcoin",
        value: "bitcoin",
    },
    FinderOption {
        label: "Ethereum",
        value: "ethereum",
    },
    FinderOption {
        label: "Base",
        value: "base",
    },
    FinderOption {
        label: "Solana",
        value: "solana",
    },
];

const TYPES: &[FinderOption] = &[
    FinderOption {
        label: "No preference",
        value: "",
    },
    FinderOption {
        label: "MCP server",
        value: "mcp",
    },
    FinderOption {
        label: "CLI",
        value: "cli",
    },
    FinderOption {
        label: "SDK",
        value: "sdk",
    },
    FinderOption {
        label: "API",
        value: "api",
    },
];

fn option_class(active: bool) -> &'static str {
    if active {
        "finder-option active"
    } else {
        "finder-option"
    }
}

#[component]
fn FinderOptionGroup(
    legend: &'static str,
    options: &'static [FinderOption],
    selected: Memo<Option<String>>,
    on_pick: Callback<Option<String>>,
) -> impl IntoView {
    view! {
        <fieldset class="finder-group">
            <legend>{legend}</legend>
            <div class="finder-options">
                {options.iter().copied().map(|option| {
                    let value = option.value.to_string();
                    let pick_value = if option.value.is_empty() {
                        None
                    } else {
                        Some(value.clone())
                    };
                    let active_value = pick_value.clone();
                    let click_value = pick_value.clone();
                    view! {
                        <button
                            type="button"
                            class=move || option_class(selected.get().as_deref() == active_value.as_deref())
                            on:click=move |_| on_pick.run(click_value.clone())
                        >
                            {option.label}
                        </button>
                    }
                }).collect_view()}
            </div>
        </fieldset>
    }
}

#[component]
pub fn ToolFinderPanel() -> impl IntoView {
    let open = RwSignal::new(false);
    let answers = RwSignal::new(ToolFinderAnswers::default());
    let selected_function = Memo::new(move |_| answers.get().function);
    let selected_chain = Memo::new(move |_| answers.get().chain);
    let selected_type = Memo::new(move |_| answers.get().tool_type);
    let selected_safety = Memo::new(move |_| answers.get().safety);
    let href = Memo::new(move |_| tool_finder_href(&answers.get()));

    let pick_function = Callback::new(move |value: Option<String>| {
        answers.update(|state| state.function = value);
    });
    let pick_chain = Callback::new(move |value: Option<String>| {
        answers.update(|state| state.chain = value);
    });
    let pick_type = Callback::new(move |value: Option<String>| {
        answers.update(|state| state.tool_type = value);
    });

    view! {
        <section class="tool-finder" aria-labelledby="tool-finder-title">
            <button
                type="button"
                class="tool-finder-toggle"
                prop:aria-expanded=move || open.get()
                aria-controls="tool-finder-panel"
                on:click=move |_| open.update(|value| *value = !*value)
            >
                <span id="tool-finder-title">"Tool Finder"</span>
                <span aria-hidden="true">{move || if open.get() { "Hide" } else { "Open" }}</span>
            </button>
            <Show when=move || open.get()>
                <div id="tool-finder-panel" class="tool-finder-panel">
                    <FinderOptionGroup
                        legend="What are you building?"
                        options=FUNCTIONS
                        selected=selected_function
                        on_pick=pick_function
                    />
                    <FinderOptionGroup
                        legend="Where should it work?"
                        options=CHAINS
                        selected=selected_chain
                        on_pick=pick_chain
                    />
                    <FinderOptionGroup
                        legend="How will you use it?"
                        options=TYPES
                        selected=selected_type
                        on_pick=pick_type
                    />
                    <fieldset class="finder-group">
                        <legend>"Install safety"</legend>
                        <div class="finder-options">
                            <button
                                type="button"
                                class=move || option_class(matches!(selected_safety.get(), FinderSafety::LowRiskOnly))
                                on:click=move |_| answers.update(|state| state.safety = FinderSafety::LowRiskOnly)
                            >
                                "Low risk only"
                            </button>
                            <button
                                type="button"
                                class=move || option_class(matches!(selected_safety.get(), FinderSafety::VerifiedPreferred))
                                on:click=move |_| answers.update(|state| state.safety = FinderSafety::VerifiedPreferred)
                            >
                                "Verified preferred"
                            </button>
                            <button
                                type="button"
                                class=move || option_class(matches!(selected_safety.get(), FinderSafety::ExcludeCritical))
                                on:click=move |_| answers.update(|state| state.safety = FinderSafety::ExcludeCritical)
                            >
                                "Show all safe public tools"
                            </button>
                        </div>
                    </fieldset>
                    <div class="finder-actions">
                        <a href=move || href.get() class="finder-submit">"Find matching tools"</a>
                        <button
                            type="button"
                            class="finder-reset"
                            on:click=move |_| answers.set(ToolFinderAnswers::default())
                        >
                            "Reset"
                        </button>
                    </div>
                </div>
            </Show>
        </section>
    }
}
