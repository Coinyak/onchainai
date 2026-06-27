//! Featured tool carousel — SSR first card + client auto-advance (harness-round-11).

use crate::server::functions::FeaturedCardView;
use leptos::prelude::*;

#[cfg(feature = "hydrate")]
use gloo_timers::callback::Interval;

#[component]
pub fn FeaturedCarousel(cards: Vec<FeaturedCardView>) -> impl IntoView {
    if cards.is_empty() {
        return ().into_any();
    }

    let len = cards.len();
    let current = RwSignal::new(0usize);
    let paused = RwSignal::new(false);

    #[cfg(feature = "hydrate")]
    {
        let interval = StoredValue::new_local(None::<Interval>);
        Effect::new(move |_| {
            interval.update_value(|slot| {
                if let Some(handle) = slot.take() {
                    drop(handle);
                }
                if !paused.get() && len > 1 {
                    *slot = Some(Interval::new(3_000, move || {
                        current.update(|idx| *idx = (*idx + 1) % len);
                    }));
                }
            });
        });
    }

    view! {
        <section
            class="featured-carousel"
            aria-label="Featured tools carousel"
            aria-roledescription="carousel"
            on:mouseenter=move |_| paused.set(true)
            on:mouseleave=move |_| paused.set(false)
            on:focusin=move |_| paused.set(true)
            on:focusout=move |_| paused.set(false)
        >
            <div class="featured-carousel-track">
                {cards.clone().into_iter().enumerate().map(|(idx, card)| {
                    let slug = card.tool_slug.clone();
                    let href = format!("/tools/{slug}");
                    let title = card
                        .headline
                        .clone()
                        .filter(|h| !h.trim().is_empty())
                        .unwrap_or_else(|| card.tool_name.clone());
                    let subtitle = card.subtitle.clone().unwrap_or_default();
                    let image_url = card.image_url.clone();
                    view! {
                        <a
                            href=href
                            class=move || if current.get() == idx {
                                "featured-carousel-card active"
                            } else {
                                "featured-carousel-card pointer-events-none"
                            }
                            prop:tabindex=move || if current.get() == idx { 0 } else { -1 }
                            aria-hidden=move || if current.get() == idx { "false" } else { "true" }
                        >
                            <img class="featured-carousel-image" src=image_url alt=title.clone()/>
                            <div class="featured-carousel-overlay">
                                <h2 class="featured-carousel-headline">{title}</h2>
                                {if !subtitle.is_empty() {
                                    view! {
                                        <p class="featured-carousel-subtitle">{subtitle}</p>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }}
                            </div>
                        </a>
                    }
                }).collect_view()}
            </div>
            {if len > 1 {
                view! {
                    <div class="featured-carousel-dots" role="tablist" aria-label="Featured slides">
                        {cards.into_iter().enumerate().map(|(idx, card)| {
                            let label = card
                                .headline
                                .filter(|h| !h.trim().is_empty())
                                .unwrap_or(card.tool_name);
                            view! {
                                <button
                                    type="button"
                                    class=move || if current.get() == idx {
                                        "carousel-dot active"
                                    } else {
                                        "carousel-dot"
                                    }
                                    role="tab"
                                    aria-label=format!("Show {label}")
                                    aria-selected=move || if current.get() == idx { "true" } else { "false" }
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        ev.prevent_default();
                                        current.set(idx);
                                    }
                                />
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}
        </section>
    }
    .into_any()
}
