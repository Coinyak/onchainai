"use client";

import { useState } from "react";
import { monogramFromName } from "@/lib/format";
import { shouldPreferMonogramOverLogo } from "@/lib/tool-logo";

interface ToolLogoProps {
  name: string;
  logoUrl?: string | null;
  logoMonogram?: string | null;
  size?: number;
  status?: string | null;
}

export function ToolLogo({
  name,
  logoUrl,
  logoMonogram,
  size = 48,
  status,
}: ToolLogoProps) {
  const skipGenericGithub = shouldPreferMonogramOverLogo(logoUrl, status);
  const effectiveUrl = skipGenericGithub ? null : logoUrl;
  const [showImg, setShowImg] = useState(!!effectiveUrl);
  const monogram = logoMonogram?.trim() || monogramFromName(name);
  const isBrandLogo = !!effectiveUrl?.startsWith("/brand/");

  return (
    <div className="tool-logo" style={{ width: size, height: size }}>
      <span className="tool-logo-monogram" aria-hidden="true">
        {monogram}
      </span>
      {showImg && effectiveUrl && (
        <img
          className={isBrandLogo ? "tool-logo-img tool-logo-img-brand" : "tool-logo-img"}
          src={effectiveUrl}
          alt=""
          width={size}
          height={size}
          referrerPolicy="no-referrer"
          onError={() => setShowImg(false)}
        />
      )}
    </div>
  );
}