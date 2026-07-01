//! Featured tool carousel — SSR first card + client auto-advance (harness-round-11).

use crate::components::admin_context::{use_current_user_resource, user_is_admin};
use crate::components::icons::LucideIcon;
use crate::server::functions::FeaturedCardView;
use leptos::prelude::*;

#[cfg(feature = "hydrate")]
use gloo_timers::callback::Interval;

#[derive(Clone, Copy)]
enum CarouselDirection {
    Previous,
    Next,
}

fn carousel_target_index(current: usize, len: usize, direction: CarouselDirection) -> usize {
    if len == 0 {
        return 0;
    }

    match direction {
        CarouselDirection::Previous if current == 0 => len - 1,
        CarouselDirection::Previous => current - 1,
        CarouselDirection::Next => (current + 1) % len,
    }
}

fn featured_edit_href(cards: &[FeaturedCardView], current: usize) -> String {
    cards
        .get(current)
        .map(|card| format!("/admin/featured?edit={}", card.id))
        .unwrap_or_else(|| "/admin/featured".into())
}

fn featured_add_href(cards: &[FeaturedCardView], current: usize) -> String {
    cards
        .get(current)
        .map(|card| format!("/admin/featured?new=1&tool={}", card.tool_slug))
        .unwrap_or_else(|| "/admin/featured?new=1".into())
}

#[component]
fn FeaturedCarouselAdminActions(
    cards: Vec<FeaturedCardView>,
    current: RwSignal<usize>,
) -> impl IntoView {
    let Some(user) = use_current_user_resource() else {
        return ().into_any();
    };

    view! {
        <Suspense fallback=|| ()>
            {move || {
                let is_admin = user.get().map(|res| user_is_admin(&res)).unwrap_or(false);
                if !is_admin {
                    return ().into_any();
                }
                let current_idx = current.get();
                view! {
                    <div class="featured-admin-actions">
                        <a class="featured-admin-link" href=featured_edit_href(&cards, current_idx)>
                            "Edit"
                        </a>
                        <a class="featured-admin-link" href=featured_add_href(&cards, current_idx)>
                            "Add"
                        </a>
                    </div>
                }.into_any()
            }}
        </Suspense>
    }
    .into_any()
}

#[component]
pub fn FeaturedCarousel(cards: Vec<FeaturedCardView>) -> impl IntoView {
    if cards.is_empty() {
        return ().into_any();
    }

    let len = cards.len();
    let current = RwSignal::new(0usize);
    let paused = RwSignal::new(false);
    let admin_cards = cards.clone();
    let dot_cards = cards.clone();

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
            <FeaturedCarouselAdminActions cards=admin_cards current=current/>
            {if len > 1 {
                view! {
                    <button
                        type="button"
                        class="carousel-arrow carousel-arrow-prev"
                        aria-label="Previous featured card"
                        on:click=move |ev| {
                            ev.stop_propagation();
                            ev.prevent_default();
                            current.set(carousel_target_index(
                                current.get_untracked(),
                                len,
                                CarouselDirection::Previous,
                            ));
                        }
                    >
                        <LucideIcon name="chevron-left".to_string() class="carousel-arrow-icon"/>
                    </button>
                    <button
                        type="button"
                        class="carousel-arrow carousel-arrow-next"
                        aria-label="Next featured card"
                        on:click=move |ev| {
                            ev.stop_propagation();
                            ev.prevent_default();
                            current.set(carousel_target_index(
                                current.get_untracked(),
                                len,
                                CarouselDirection::Next,
                            ));
                        }
                    >
                        <LucideIcon name="chevron-right".to_string() class="carousel-arrow-icon"/>
                    </button>
                    <div class="featured-carousel-dots" role="tablist" aria-label="Featured slides">
                        {dot_cards.into_iter().enumerate().map(|(idx, card)| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carousel_target_index_wraps_cleanly() {
        assert_eq!(carousel_target_index(0, 0, CarouselDirection::Next), 0);
        assert_eq!(carousel_target_index(0, 3, CarouselDirection::Previous), 2);
        assert_eq!(carousel_target_index(2, 3, CarouselDirection::Next), 0);
        assert_eq!(carousel_target_index(1, 3, CarouselDirection::Previous), 0);
    }
}
