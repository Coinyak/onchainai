import Link from "next/link";
import { ExternalLink, Star, MessageCircle } from "lucide-react";
import type { Tool, TrustFact } from "@/lib/api";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";
import { InstallSection } from "@/components/tools/InstallSection";
import { InstallGuidePanel } from "@/components/tools/InstallGuidePanel";
import { AddMcpAction } from "@/components/tools/AddMcpAction";
import { TrustFacts } from "@/components/tools/TrustFacts";
import { ChainLogo } from "@/components/tools/ChainLogo";
import { chainTagsForTool } from "@/lib/chains";
import { compareHref } from "@/lib/browser-query";
import { toolHasInstallPath } from "@/lib/install-guide";
import { timeAgo, statusBadgeLabel } from "@/lib/format";

interface ToolDetailProps {
  tool: Tool;
  trustFacts?: TrustFact[];
  compact?: boolean;
  commentCount?: number;
  addMode?: boolean;
  addMcpQueryBase?: string;
  compareReturnHref?: string;
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
  addMode = false,
  addMcpQueryBase = "",
  compareReturnHref = "",
}: ToolDetailProps) {
  const chains = chainTagsForTool(tool.chains);
  const links = [
    tool.repo_url && { label: "GitHub", url: tool.repo_url, extra: tool.stars ? `${tool.stars}★` : undefined },
    tool.homepage && { label: "Homepage", url: tool.homepage },
    tool.npm_package && { label: "npm", url: tool.npm_package.startsWith("http") ? tool.npm_package : `https://www.npmjs.com/package/${tool.npm_package}` },
    tool.mcp_endpoint && { label: "MCP", url: tool.mcp_endpoint },
  ].filter(Boolean) as { label: string; url: string; extra?: string }[];

  const contentClass = addMode
    ? compact
      ? "detail-content compact add-mode"
      : "detail-content add-mode"
    : compact
      ? "detail-content compact"
      : "detail-content";

  return (
    <article className={`tool-detail ${contentClass}`}>
      <header className="tool-detail-header detail-header">
        <ToolLogo
          name={tool.name}
          logoUrl={tool.logo_url}
          logoMonogram={tool.logo_monogram}
          size={compact ? 48 : 64}
        />
        <div className="tool-detail-heading detail-header-text">
          <div className="detail-header-row">
            <h1 className={compact ? "text-h2 detail-title" : "text-h1 detail-title"}>
              {tool.name}
            </h1>
            {!addMode && addMcpQueryBase && (
              <AddMcpAction
                tool={tool}
                hrefSource={{ kind: "query_base", base: addMcpQueryBase }}
                variant="detail_primary"
              />
            )}
          </div>
          <div className="tool-detail-badges tool-badges">
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
          {!addMode && (
            <div className="tool-detail-stats">
              <span><Star size={14} /> {tool.stars}</span>
              <span><MessageCircle size={14} /> {commentCount} comments</span>
              <span>updated {timeAgo(tool.last_commit_at || tool.updated_at)}</span>
            </div>
          )}
        </div>
      </header>

      {addMode ? (
        <>
          {compareReturnHref && (
            <Link href={compareReturnHref} className="detail-compare-return-link">
              ← Back to compare
            </Link>
          )}
          <TrustFacts tool={tool} facts={trustFacts} />
          <InstallGuidePanel tool={tool} compact={compact} showProgress />
          {tool.description && (
            <section className="detail-section">
              <h2 className="text-h2 mb-3">Description</h2>
              <p className="text-body-md md:text-mobile-body leading-relaxed detail-desc">
                {tool.description}
              </p>
            </section>
          )}
          <div className="detail-compare-row">
            <Link href={compareHref([tool.slug])} className="detail-compare-link">
              Compare this tool
            </Link>
          </div>
        </>
      ) : (
        <>
          {tool.description && (
            <section className="detail-section">
              <h2 className="text-h2 mb-3">Description</h2>
              <p className="text-body-md md:text-mobile-body leading-relaxed">{tool.description}</p>
            </section>
          )}
          {toolHasInstallPath(tool) && <InstallSection tool={tool} compact={compact} />}
          <div className="detail-compare-row">
            <Link href={compareHref([tool.slug])} className="detail-compare-link">
              Compare this tool
            </Link>
          </div>
        </>
      )}

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
        <section className="detail-section links-section">
          <h2 className="text-h2 mb-3 install-heading">Links</h2>
          <div className="detail-links">
            {links.map((link) => (
              <a
                key={link.url}
                href={link.url}
                target="_blank"
                rel="noopener noreferrer"
                className="detail-link external-link no-underline"
              >
                {link.label}
                {link.extra && ` ${link.extra}`}
                <ExternalLink size={14} aria-hidden />
              </a>
            ))}
          </div>
        </section>
      )}

      {!addMode && <TrustFacts tool={tool} facts={trustFacts} />}

      {!compact && !addMode && (
        <p className="mt-4">
          <Link href={`/tools/${tool.slug}`} className="text-tertiary">
            View full page
          </Link>
        </p>
      )}
    </article>
  );
}