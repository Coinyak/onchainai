"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";
import { ChevronLeft, ChevronRight } from "lucide-react";
import type { FeaturedCard } from "@/lib/api";

interface FeaturedCarouselProps {
  cards: FeaturedCard[];
}

type CarouselDirection = "previous" | "next";

function carouselTargetIndex(
  current: number,
  len: number,
  direction: CarouselDirection,
): number {
  if (len === 0) return 0;
  if (direction === "previous") {
    return current === 0 ? len - 1 : current - 1;
  }
  return (current + 1) % len;
}

function prefersReducedMotion(): boolean {
  if (typeof window === "undefined") return false;
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function FeaturedCarousel({ cards }: FeaturedCarouselProps) {
  const [active, setActive] = useState(0);
  const [paused, setPaused] = useState(false);
  const reducedMotion = useRef(false);

  const goTo = useCallback(
    (direction: CarouselDirection) => {
      setActive((i) => carouselTargetIndex(i, cards.length, direction));
    },
    [cards.length],
  );

  useEffect(() => {
    reducedMotion.current = prefersReducedMotion();
  }, []);

  useEffect(() => {
    if (cards.length <= 1 || paused || reducedMotion.current) return;
    const id = setInterval(() => {
      setActive((i) => carouselTargetIndex(i, cards.length, "next"));
    }, 6000);
    return () => clearInterval(id);
  }, [cards.length, paused]);

  if (!cards.length) return null;

  const shellClass =
    cards.length > 1
      ? "featured-carousel-shell featured-carousel-shell--controls"
      : "featured-carousel-shell";

  const handleArrowClick = (
    e: React.MouseEvent<HTMLButtonElement>,
    direction: CarouselDirection,
  ) => {
    e.stopPropagation();
    e.preventDefault();
    goTo(direction);
  };

  return (
    <div
      className={shellClass}
      onMouseEnter={() => setPaused(true)}
      onMouseLeave={() => setPaused(false)}
      onFocusCapture={() => setPaused(true)}
      onBlurCapture={() => setPaused(false)}
    >
      {cards.length > 1 && (
        <button
          type="button"
          className="carousel-arrow carousel-arrow-prev"
          aria-label="Previous featured card"
          onClick={(e) => handleArrowClick(e, "previous")}
        >
          <ChevronLeft className="carousel-arrow-icon" aria-hidden />
        </button>
      )}
      <section
        className="featured-carousel"
        aria-label="Featured tools carousel"
        aria-roledescription="carousel"
      >
        <div
          className="featured-carousel-track"
          style={{ transform: `translateX(-${active * 100}%)` }}
        >
          {cards.map((card, index) => {
            const isActive = index === active;
            const title = card.headline || card.tool_name;
            const href = `/tools/${card.tool_slug}`;
            return (
              <Link
                key={card.id}
                href={href}
                className={
                  isActive
                    ? "featured-carousel-card active no-underline"
                    : "featured-carousel-card pointer-events-none no-underline"
                }
                aria-hidden={!isActive}
                tabIndex={isActive ? 0 : -1}
              >
                <img
                  className="featured-carousel-image"
                  src={card.image_url}
                  alt={title}
                />
                <div className="featured-carousel-overlay">
                  <h2 className="featured-carousel-headline">{title}</h2>
                  {card.subtitle && (
                    <p className="featured-carousel-subtitle">{card.subtitle}</p>
                  )}
                </div>
              </Link>
            );
          })}
        </div>
        {cards.length > 1 && (
          <div
            className="featured-carousel-dots"
            role="tablist"
            aria-label="Featured slides"
          >
            {cards.map((card, index) => {
              const label = card.headline?.trim() || card.tool_name;
              return (
                <button
                  key={card.id}
                  type="button"
                  role="tab"
                  aria-selected={index === active}
                  aria-label={`Show ${label}`}
                  className={index === active ? "carousel-dot active" : "carousel-dot"}
                  onClick={(e) => {
                    e.stopPropagation();
                    e.preventDefault();
                    setActive(index);
                  }}
                />
              );
            })}
          </div>
        )}
      </section>
      {cards.length > 1 && (
        <button
          type="button"
          className="carousel-arrow carousel-arrow-next"
          aria-label="Next featured card"
          onClick={(e) => handleArrowClick(e, "next")}
        >
          <ChevronRight className="carousel-arrow-icon" aria-hidden />
        </button>
      )}
    </div>
  );
}