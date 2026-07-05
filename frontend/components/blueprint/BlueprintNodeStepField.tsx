"use client";

import { useState } from "react";
import { parseBlueprintStepInput } from "@/lib/blueprint-utils";

interface BlueprintNodeStepFieldProps {
  value?: number;
  className?: string;
  placeholder?: string;
  "aria-label"?: string;
  "data-testid"?: string;
  onChange: (step: number | undefined) => void;
  onClick?: (event: React.MouseEvent<HTMLInputElement>) => void;
  onPointerDown?: (event: React.PointerEvent<HTMLInputElement>) => void;
}

/** Commits step on blur so multi-digit entry (e.g. 10) does not fight controlled updates. */
export function BlueprintNodeStepField({
  value,
  className,
  placeholder = "#",
  onChange,
  onClick,
  onPointerDown,
  ...rest
}: BlueprintNodeStepFieldProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const displayValue = draft !== null ? draft : value != null ? String(value) : "";

  return (
    <input
      type="text"
      inputMode="numeric"
      className={className}
      maxLength={2}
      value={displayValue}
      placeholder={placeholder}
      onClick={onClick}
      onPointerDown={onPointerDown}
      onFocus={() => setDraft(value != null ? String(value) : "")}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={() => {
        const next = parseBlueprintStepInput(draft ?? "");
        onChange(next);
        setDraft(null);
      }}
      {...rest}
    />
  );
}