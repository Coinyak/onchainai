"use client";

import { useState } from "react";
import { Clipboard } from "lucide-react";

interface CopyButtonProps {
  text: string;
  label?: string;
}

export function CopyButton({ text, label = "Copy" }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  async function handleCopy(e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      /* ignore */
    }
  }

  return (
    <button
      type="button"
      className="copy-btn"
      onClick={handleCopy}
      aria-label={copied ? "Copied" : label}
    >
      {copied ? (
        "Copied"
      ) : (
        <>
          <Clipboard size={14} strokeWidth={1.75} aria-hidden />
          <span>{label}</span>
        </>
      )}
    </button>
  );
}