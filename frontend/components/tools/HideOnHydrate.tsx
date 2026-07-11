"use client";

import { useLayoutEffect, useState, type ReactNode } from "react";

/** Renders server HTML for crawlers, then removes it before interactive UI paints. */
export function HideOnHydrate({ children }: { children: ReactNode }) {
  const [hide, setHide] = useState(false);
  useLayoutEffect(() => {
    setHide(true);
  }, []);
  if (hide) return null;
  return <>{children}</>;
}
