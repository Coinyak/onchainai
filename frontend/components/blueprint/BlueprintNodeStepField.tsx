"use client";

import { useState } from "react";
import { parseBlueprintStepsInput } from "@/lib/blueprint-utils";

interface BlueprintNodeStepFieldProps {
  values?: number[];
  className?: string;
  placeholder?: string;
  "aria-label"?: string;
  "data-testid"?: string;
  onChange: (steps: number[]) => void;
  onClick?: (event: React.MouseEvent<HTMLInputElement>) => void;
  onPointerDown?: (event: React.PointerEvent<HTMLInputElement>) => void;
}

/** Commits steps on blur so multi-digit entry (e.g. 10, 1 7) does not fight controlled updates. */
export function BlueprintNodeStepField({
  values,
  className,
  placeholder = "#",
  onChange,
  onClick,
  onPointerDown,
  ...rest
}: BlueprintNodeStepFieldProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const displayValue =
    draft !== null
      ? draft
      : values && values.length > 0
        ? values.map((s) => `#${s}`).join(" ")
        : "";

  return (
    <input
      type="text"
      className={className}
      value={displayValue}
      placeholder={placeholder}
      onClick={onClick}
      onPointerDown={onPointerDown}
      onFocus={() =>
        setDraft(values && values.length > 0 ? values.map((s) => `#${s}`).join(" ") : "")
      }
      onChange={(e) => setDraft(e.target.value)}
      onBlur={() => {
        const next = parseBlueprintStepsInput(draft ?? "");
        onChange(next);
        setDraft(null);
      }}
      {...rest}
    />
  );
}