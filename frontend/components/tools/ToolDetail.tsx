import Link from "next/link";
import { ExternalLink, Star, MessageCircle } from "lucide-react";
import type { Tool, TrustFact } from "@/lib/api";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";
import { InstallSection } from "@/components/tools/InstallSection";
import { TrustFacts } from "@/components/tools/TrustFacts";
import { ChainLogo } from "@/components/tools/ChainLogo";
import { chainTagsForTool } from "@/lib/chains";
import { timeAgo, statusBadgeLabel } from "@/lib/format";

interface ToolDetailProps {
  tool: Tool;
  trustFacts?: TrustFact[];
  compact?: boolean;
  commentCount?: number;
}

function statusVariant(status: string): "verified" | "official" | "community" {
  if (status === "verified") return "verified";
  if (status === "official") return "official";
  return "community";
}

export function ToolDetail({
  tool,
  trustFacts,
  compact = false,
  commentCount = 0,
}: ToolDetailProps) {
  const chains = chainTagsForTool(tool.chains);
  const links = [
    tool.repo_url && { label: "GitHub", url: tool.repo_url, extra: tool.stars ? `${tool.stars}★` : undefined },
    tool.homepage && { label: "Homepage", url: tool.homepage },
    tool.npm_package && { label: "npm", url: tool.npm_package.startsWith("http") ? tool.npm_package : `https://www.npmjs.com/package/${tool.npm_package}` },
    tool.mcp_endpoint && { label: "MCP", url: tool.mcp_endpoint },
  ].filter(Boolean) as { label: string; url: string; extra?: string }[];

  return (
    <article className="tool-detail">
      <header className="tool-detail-header">
        <ToolLogo
          name={tool.name}
          logoUrl={tool.logo_url}
          logoMonogram={tool.logo_monogram}
          size={compact ? 48 : 64}
        />
        <div className="tool-detail-heading">
          <h1 className="text-h1">{tool.name}</h1>
          <div className="tool-detail-badges">
            <Badge variant={statusVariant(tool.status)}>{statusBadgeLabel(tool.status)}</Badge>
            {tool.official_team && (
              <Badge variant="official">Official: {tool.official_team}</Badge>
            )}
            <Badge variant={tool.type === "x402" ? "x402" : "neutral"}>
              {tool.type.toUpperCase()}
            </Badge>
            <Badge variant="neutral">{tool.function}</Badge>
            <Badge variant="neutral">{tool.asset_class}</Badge>
            <Badge variant="neutral">{tool.actor}</Badge>
          </div>
          <div className="tool-detail-stats">
            <span><Star size={14} /> {tool.stars}</span>
            <span><MessageCircle size={14} /> {commentCount} comments</span>
            <span>updated {timeAgo(tool.last_commit_at || tool.updated_at)}</span>
          </div>
        </div>
      </header>

      {tool.description && (
        <section className="detail-section">
          <h2 className="text-h2 mb-3">Description</h2>
          <p className="text-body-md md:text-mobile-body leading-relaxed">{tool.description}</p>
        </section>
      )}

      <InstallSection tool={tool} />

      {chains.length > 0 && (
        <section className="detail-section">
          <h2 className="text-h2 mb-3">Chains</h2>
          <div className="detail-chains">
            {chains.map((c) => (
              <span key={c.id} className="detail-chain-tag">
                <ChainLogo id={c.id} label={c.label} size={24} />
                {c.label}
              </span>
            ))}
          </div>
        </section>
      )}

      {links.length > 0 && (
        <section className="detail-section">
          <h2 className="text-h2 mb-3">Links</h2>
          <div className="detail-links">
            {links.map((link) => (
              <a
                key={link.url}
                href={link.url}
                target="_blank"
                rel="noopener noreferrer"
                className="detail-link no-underline"
              >
                {link.label}
                {link.extra && ` ${link.extra}`}
                <ExternalLink size={14} aria-hidden />
              </a>
            ))}
          </div>
        </section>
      )}

      <TrustFacts tool={tool} facts={trustFacts} />

      {!compact && (
        <p className="mt-4">
          <Link href={`/tools/${tool.slug}`} className="text-tertiary">
            View full page
          </Link>
        </p>
      )}
    </article>
  );
}