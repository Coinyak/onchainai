"use client";

import { useState } from "react";
import {
  codingClientLogoPath,
  hasCodingClientLogo,
  type CodingClientLogoId,
} from "@/lib/coding-clients";

interface CodingClientLogoProps {
  id: CodingClientLogoId | string;
  label: string;
  size?: number;
  className?: string;
  /** Decorative when visible text label sits beside the logo. */
  decorative?: boolean;
}

export function CodingClientLogo({
  id,
  label,
  size = 20,
  className = "coding-client-logo",
  decorative = false,
}: CodingClientLogoProps) {
  const [broken, setBroken] = useState(false);
  const logoId = hasCodingClientLogo(id) ? id : null;
  const fallback = label.trim().slice(0, 2).toUpperCase() || "?";

  if (!logoId || broken) {
    return (
      <span
        className={`${className} coding-client-logo-fallback`}
        aria-hidden={decorative ? true : undefined}
        aria-label={decorative ? undefined : label}
        style={{ width: size, height: size, fontSize: Math.max(8, Math.round(size * 0.38)) }}
      >
        {fallback}
      </span>
    );
  }

  return (
    <img
      className={className}
      src={codingClientLogoPath(logoId)}
      alt={decorative ? "" : label}
      title={decorative ? undefined : label}
      aria-hidden={decorative ? true : undefined}
      width={size}
      height={size}
      style={{ width: size, height: size }}
      onError={() => setBroken(true)}
    />
  );
}