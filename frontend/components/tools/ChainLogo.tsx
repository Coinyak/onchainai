"use client";

import { useState } from "react";
import { chainFallbackLabel, chainLogoPath, hasChainLogo } from "@/lib/chains";

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
  const logoAvailable = hasChainLogo(id);
  const [brokenForId, setBrokenForId] = useState<string | null>(null);
  const fallbackLabel = chainFallbackLabel(label || id).slice(0, 3).toUpperCase();

  if (brokenForId !== null && brokenForId !== id) {
    setBrokenForId(null);
  }

  const broken = brokenForId === id;

  if (!logoAvailable || broken) {
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
      loading="lazy"
      decoding="async"
      onError={() => setBrokenForId(id)}
    />
  );
}