"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";

export function SearchBar() {
  const router = useRouter();
  const [input, setInput] = useState("");
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    if (!input.trim()) return;
    timerRef.current = setTimeout(() => {
      const q = encodeURIComponent(input.trim());
      router.push(`/tools?q=${q}`);
    }, 200);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [input, router]);

  return (
    <div className="w-full">
      <input
        type="search"
        placeholder="Search: asset tracking, trading, DeFi, chain name..."
        className="search-input w-full h-12 px-4 text-body-md md:text-mobile-body rounded-md border border-border bg-neutral-bg text-primary outline-none focus:border-tertiary"
        autoComplete="off"
        value={input}
        onChange={(e) => setInput(e.target.value)}
      />
    </div>
  );
}