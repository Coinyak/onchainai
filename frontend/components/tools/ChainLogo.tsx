"use client";

import { useState } from "react";
import { chainFallbackLabel, chainLogoPath } from "@/lib/chains";

interface ChainLogoProps {
  id: string;
  label: string;
  size?: number;
  className?: string;
  /** When true, image is decorative and alt is empty (label is shown beside logo). */
  decorative?: boolean;
}

export function ChainLogo({
  id,
  label,
  size = 36,
  className = "chain-logo",
  decorative = false,
}: ChainLogoProps) {
  // Always attempt the committed SVG first; fall back to initials only on load error.
  const [broken, setBroken] = useState(false);
  const fallbackLabel = chainFallbackLabel(label || id).slice(0, 3).toUpperCase();

  if (broken) {
    return (
      <span
        className={`${className} chain-logo-fallback`}
        title={decorative ? undefined : label}
        aria-label={decorative ? undefined : label}
        aria-hidden={decorative ? true : undefined}
        style={{ width: size, height: size, fontSize: Math.max(8, Math.round(size * 0.35)) }}
      >
        {fallbackLabel}
      </span>
    );
  }

  return (
    <img
      className={className}
      src={chainLogoPath(id)}
      alt={decorative ? "" : label}
      title={decorative ? undefined : label}
      aria-hidden={decorative ? true : undefined}
      width={size}
      height={size}
      onError={() => setBroken(true)}
    />
  );
}