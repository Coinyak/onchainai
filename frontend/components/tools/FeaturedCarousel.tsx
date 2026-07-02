"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import type { FeaturedCard } from "@/lib/api";

interface FeaturedCarouselProps {
  cards: FeaturedCard[];
}

export function FeaturedCarousel({ cards }: FeaturedCarouselProps) {
  const [active, setActive] = useState(0);

  useEffect(() => {
    if (cards.length <= 1) return;
    const id = setInterval(() => {
      setActive((i) => (i + 1) % cards.length);
    }, 3000);
    return () => clearInterval(id);
  }, [cards.length]);

  if (!cards.length) return null;

  const shellClass =
    cards.length > 1
      ? "featured-carousel-shell featured-carousel-shell--controls"
      : "featured-carousel-shell";

  return (
    <section className={shellClass} aria-label="Featured tools">
      <div className="featured-carousel">
        <div className="featured-carousel-track">
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
                <img className="featured-carousel-image" src={card.image_url} alt={title} />
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
          <div className="featured-carousel-dots" role="tablist" aria-label="Featured slides">
            {cards.map((card, index) => (
              <button
                key={card.id}
                type="button"
                role="tab"
                aria-selected={index === active}
                aria-label={`Slide ${index + 1}`}
                className={index === active ? "featured-dot active" : "featured-dot"}
                onClick={() => setActive(index)}
              />
            ))}
          </div>
        )}
      </div>
    </section>
  );
}