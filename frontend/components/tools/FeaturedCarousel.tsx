"use client";

import { useEffect, useRef, useState } from "react";
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

function hasRenderableImage(url: string | null | undefined): boolean {
  const trimmed = url?.trim();
  return Boolean(trimmed && (trimmed.startsWith("http://") || trimmed.startsWith("https://")));
}

export function FeaturedCarousel({ cards }: FeaturedCarouselProps) {
  const renderableCards = cards.filter((card) => hasRenderableImage(card.image_url));
  const [active, setActive] = useState(0);
  const [paused, setPaused] = useState(false);
  const [brokenIds, setBrokenIds] = useState<Set<string>>(() => new Set());
  const reducedMotion = useRef(false);

  const visibleCards = renderableCards.filter((card) => !brokenIds.has(card.id));
  const safeActive =
    visibleCards.length === 0 ? 0 : Math.min(active, visibleCards.length - 1);

  const cardCount = visibleCards.length;
  const goTo = (direction: CarouselDirection) => {
    setActive((i) => carouselTargetIndex(i, cardCount, direction));
  };

  useEffect(() => {
    reducedMotion.current = prefersReducedMotion();
  }, []);

  useEffect(() => {
    if (visibleCards.length <= 1 || paused || reducedMotion.current) return;
    const id = setInterval(() => {
      setActive((i) => carouselTargetIndex(i, visibleCards.length, "next"));
    }, 6000);
    return () => clearInterval(id);
  }, [cardCount, paused]);

  if (!visibleCards.length) return null;

  const shellClass =
    visibleCards.length > 1
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
      {visibleCards.length > 1 && (
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
          style={{ transform: `translateX(-${safeActive * 100}%)` }}
        >
          {visibleCards.map((card, index) => {
            const isActive = index === safeActive;
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
                  onError={() => {
                    setBrokenIds((prev) => {
                      const next = new Set(prev);
                      next.add(card.id);
                      return next;
                    });
                  }}
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
        {visibleCards.length > 1 && (
          <div
            className="featured-carousel-dots"
            role="tablist"
            aria-label="Featured slides"
          >
            {visibleCards.map((card, index) => {
              const label = card.headline?.trim() || card.tool_name;
              return (
                <button
                  key={card.id}
                  type="button"
                  role="tab"
                  aria-selected={index === safeActive}
                  aria-label={`Show ${label}`}
                  className={index === safeActive ? "carousel-dot active" : "carousel-dot"}
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
      {visibleCards.length > 1 && (
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