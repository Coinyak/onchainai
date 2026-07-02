"use client";

import { useState } from "react";
import { monogramFromName } from "@/lib/format";

interface ToolLogoProps {
  name: string;
  logoUrl?: string | null;
  logoMonogram?: string | null;
  size?: number;
}

export function ToolLogo({ name, logoUrl, logoMonogram, size = 48 }: ToolLogoProps) {
  const [showImg, setShowImg] = useState(!!logoUrl);
  const monogram = logoMonogram?.trim() || monogramFromName(name);

  return (
    <div className="tool-logo" style={{ width: size, height: size }}>
      <span className="tool-logo-monogram" aria-hidden="true">
        {monogram}
      </span>
      {showImg && logoUrl && (
        <img
          className="tool-logo-img"
          src={logoUrl}
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