"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { getRelatedTools } from "@/lib/api";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";
import { typeBadgeLabel } from "@/lib/format";

interface RelatedToolsSectionProps {
  slug: string;
}

export function RelatedToolsSection({ slug }: RelatedToolsSectionProps) {
  const { data: related = [] } = useQuery({
    queryKey: ["related-tools", slug],
    queryFn: () => getRelatedTools(slug, 4),
    staleTime: 5 * 60 * 1000,
  });

  if (related.length === 0) return null;

  return (
    <section className="detail-section related-tools-section" data-testid="related-tools-section">
      <h2 className="text-h2 mb-3">Related tools</h2>
      <div className="related-tools-grid">
        {related.map((tool) => (
          <Link
            key={tool.slug}
            href={`/tools/${tool.slug}`}
            className="related-tool-card no-underline text-inherit"
            data-testid="related-tool-card"
          >
            <ToolLogo
              name={tool.name}
              logoUrl={tool.logo_url}
              logoMonogram={tool.logo_monogram}
              size={40}
            />
            <div className="related-tool-card-body">
              <h3 className="related-tool-card-name">{tool.name}</h3>
              <Badge variant={tool.type === "x402" ? "x402" : "neutral"}>
                {typeBadgeLabel(tool.type)}
              </Badge>
            </div>
          </Link>
        ))}
      </div>
    </section>
  );
}