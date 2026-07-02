"use client";

import { useQuery } from "@tanstack/react-query";
import { getSiteSettings } from "@/lib/api";

export default function AboutPage() {
  const settingsQuery = useQuery({
    queryKey: ["site-settings"],
    queryFn: getSiteSettings,
    retry: false,
  });

  const content = settingsQuery.data?.about_content?.trim();

  return (
    <div className="px-gutter md:px-6 py-8 md:py-10 max-w-[720px]">
      <h1 className="text-h1 font-bold mb-4">About</h1>
      {content ? (
        <div className="text-body-md leading-relaxed whitespace-pre-wrap text-primary">
          {content}
        </div>
      ) : (
        <p className="text-secondary text-body-md">
          About content has not been published yet.
        </p>
      )}
    </div>
  );
}