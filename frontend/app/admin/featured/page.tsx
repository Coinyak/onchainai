"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { listFeaturedCardsAdmin } from "@/lib/api";

export default function AdminFeaturedPage() {
  const featuredQuery = useQuery({
    queryKey: ["admin-featured"],
    queryFn: listFeaturedCardsAdmin,
  });

  return (
    <div className="px-gutter md:px-6 py-8 max-w-[900px] mx-auto">
      <h1 className="text-h2 mb-6">Featured carousel</h1>
      <p className="text-secondary text-body-md mb-6">
        Manage hero carousel cards shown on the home page when active.
      </p>
      <div className="space-y-4">
        {featuredQuery.data?.map((card) => (
          <article key={card.id} className="border border-border rounded-md p-lg flex gap-4">
            <img src={card.image_url} alt="" className="w-24 h-16 object-cover rounded-sm" />
            <div>
              <h3 className="text-h3">{card.headline || card.tool_name}</h3>
              <p className="text-body-sm text-secondary">{card.subtitle}</p>
              <Link href={`/tools/${card.tool_slug}`} className="text-tertiary text-body-sm">
                {card.tool_slug}
              </Link>
            </div>
          </article>
        ))}
        {featuredQuery.data?.length === 0 && (
          <p className="text-secondary">No featured cards yet.</p>
        )}
      </div>
    </div>
  );
}