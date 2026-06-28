//! Horizontal chain logo strip — toggles `?chain=` multi-select filters (harness-round-11).

use crate::chains::{chain_filter_active, strip_chains, ChainMeta, STRIP_PRIMARY_VISIBLE};
use crate::components::chain_logo::ChainLogo;
use crate::components::tools_browser::BrowserBase;
use crate::filter_query::{clear_axis, parse_multi, toggle_multi};
use leptos::prelude::*;

#[component]
pub fn ChainStrip(
    base: BrowserBase,
    query_base: String,
    active_chain: Option<String>,
    chain_counts: Vec<(String, i64)>,
) -> impl IntoView {
    let base_path = base.path();
    let chain_active = parse_multi(active_chain.as_deref());
    let all_href = clear_axis(&base_path, &query_base, "chain");
    let all_active = chain_active.is_empty();
    let expanded = RwSignal::new(false);

    let chains = strip_chains(&chain_counts);
    let primary: Vec<&'static ChainMeta> =
        chains.iter().take(STRIP_PRIMARY_VISIBLE).copied().collect();
    let overflow: Vec<&'static ChainMeta> =
        chains.iter().skip(STRIP_PRIMARY_VISIBLE).copied().collect();
    let overflow_count = overflow.len();

    view! {
        <div class="chain-strip" role="group" aria-label="Filter by chain">
            <div class="chain-strip-scroll">
                <a
                    href=all_href
                    class=if all_active { "chain-tile chain-tile-all active" } else { "chain-tile chain-tile-all" }
                    aria-label="All chains"
                    title="All chains"
                    aria-pressed=if all_active { "true" } else { "false" }
                >
                    "All"
                </a>

                {primary.into_iter().map(|entry| {
                    chain_tile(&base_path, &query_base, &chain_active, entry)
                }).collect_view()}

                <Show when=move || expanded.get()>
                    {overflow.clone().into_iter().map(|entry| {
                        chain_tile(&base_path, &query_base, &chain_active, entry)
                    }).collect_view()}
                </Show>

                {(overflow_count > 0).then(|| view! {
                    <button
                        type="button"
                        class=move || if expanded.get() { "chain-tile chain-tile-more active" } else { "chain-tile chain-tile-more" }
                        aria-label=move || if expanded.get() {
                            "Hide extra chains".to_string()
                        } else {
                            format!("Show {overflow_count} more chains")
                        }
                        title=move || if expanded.get() {
                            "Hide extra chains".to_string()
                        } else {
                            format!("Show {overflow_count} more chains")
                        }
                        aria-expanded=move || if expanded.get() { "true" } else { "false" }
                        on:click=move |ev| {
                            ev.stop_propagation();
                            ev.prevent_default();
                            expanded.update(|v| *v = !*v);
                        }
                    >
                        {move || if expanded.get() {
                            "Less".to_string()
                        } else {
                            format!("+{overflow_count}")
                        }}
                    </button>
                })}
            </div>
        </div>
    }
}

fn chain_tile(
    base_path: &str,
    query_base: &str,
    chain_active: &[String],
    entry: &'static ChainMeta,
) -> impl IntoView {
    let href = toggle_multi(base_path, query_base, "chain", entry.id, chain_active);
    let is_active = chain_filter_active(entry, chain_active);
    let label = entry.label.to_string();
    let class = if is_active {
        "chain-tile chain-tile-logo active"
    } else {
        "chain-tile chain-tile-logo"
    };

    view! {
        <a
            href=href
            class=class
            aria-label=label.clone()
            title=label
            aria-pressed=if is_active { "true" } else { "false" }
        >
            <ChainLogo id=entry.id.to_string() label=entry.label.to_string() size=36/>
        </a>
    }
}
