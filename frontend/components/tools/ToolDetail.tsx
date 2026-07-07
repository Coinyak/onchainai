import Link from "next/link";
import { ExternalLink, Star, MessageCircle } from "lucide-react";
import type { PublicTool, StaleTrustBadge, TrustFact } from "@/lib/api";
import { TrustProbeBadge } from "@/components/tools/TrustProbeBadge";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";
import { InstallSection } from "@/components/tools/InstallSection";
import { InstallGuidePanel } from "@/components/tools/InstallGuidePanel";
import { AddMcpAction } from "@/components/tools/AddMcpAction";
import { ToolDetailBookmark } from "@/components/tools/ToolDetailBookmark";
import { TrustFacts } from "@/components/tools/TrustFacts";
import { DetailFilterChip } from "@/components/tools/DetailFilterChip";
import { DetailAboutSidebar } from "@/components/tools/DetailAboutSidebar";
import { DetailChainsSection } from "@/components/tools/DetailChainsSection";
import { compareHref, singleFilterHref } from "@/lib/browser-query";
import { toolHasInstallPath } from "@/lib/install-guide";
import { timeAgo, statusBadgeLabel, formatGithubStars } from "@/lib/format";

interface ToolDetailProps {
  tool: PublicTool;
  trustProbe?: StaleTrustBadge | null;
  trustFacts?: TrustFact[];
  compact?: boolean;
  commentCount?: number;
  addMode?: boolean;
  addMcpQueryBase?: string;
  compareReturnHref?: string;
  /** Only set true in preview/sheet contexts — never on full tool pages. */
  showFullPageLink?: boolean;
}

function statusVariant(status: string): "verified" | "official" | "community" {
  if (status === "verified") return "verified";
  if (status === "official") return "official";
  return "community";
}

function DetailMainContent({
  tool,
  trustProbe,
  trustFacts,
  compact,
  addMode,
  compareReturnHref,
}: {
  tool: PublicTool;
  trustProbe?: StaleTrustBadge | null;
  trustFacts?: TrustFact[];
  compact: boolean;
  addMode: boolean;
  compareReturnHref: string;
}) {
  const links = [
    tool.repo_url && {
      label: tool.stars ? formatGithubStars(tool.stars) : "GitHub",
      url: tool.repo_url,
    },
    tool.homepage && { label: "Homepage", url: tool.homepage },
    tool.npm_package && {
      label: "npm",
      url: tool.npm_package.startsWith("http")
        ? tool.npm_package
        : `https://www.npmjs.com/package/${tool.npm_package}`,
    },
    tool.mcp_endpoint && { label: "MCP", url: tool.mcp_endpoint },
  ].filter(Boolean) as { label: string; url: string; extra?: string }[];

  if (addMode) {
    return (
      <>
        {compareReturnHref && (
          <Link href={compareReturnHref} scroll={false} className="detail-compare-return-link">
            ← Back to compare
          </Link>
        )}
        <InstallGuidePanel tool={tool} compact={compact} showProgress={compact} />
        {trustProbe && <TrustProbeBadge trustProbe={trustProbe} />}
        <TrustFacts
          tool={tool}
          facts={trustFacts}
          variant={compact ? "preview" : "default"}
        />
        {tool.description && (
          <section className={compact ? "preview-description" : "detail-section"}>
            <h3 className={compact ? "preview-section-heading" : "text-h2 mb-3"}>Description</h3>
            <p
              className={
                compact
                  ? "preview-desc preview-desc-clamped"
                  : "text-body-md md:text-mobile-body leading-relaxed detail-desc"
              }
            >
              {tool.description}
            </p>
          </section>
        )}
        {!compact && (
          <div className="detail-compare-row">
            <Link href={compareHref([tool.slug])} className="detail-compare-link">
              Compare this tool
            </Link>
          </div>
        )}
      </>
    );
  }

  return (
    <>
      {tool.description && (
        <section className="detail-section">
          <h2 className="text-h2 mb-3">Description</h2>
          <p className="text-body-md md:text-mobile-body leading-relaxed detail-desc">
            {tool.description}
          </p>
        </section>
      )}
      {toolHasInstallPath(tool) && <InstallSection tool={tool} compact={compact} />}
      {trustProbe && <TrustProbeBadge trustProbe={trustProbe} />}
      <div className="detail-compare-row">
        <Link href={compareHref([tool.slug])} className="detail-compare-link">
          Compare this tool
        </Link>
      </div>
      <DetailChainsSection chains={tool.chains} />
      {links.length > 0 && (
        <section className="detail-section links-section">
          <h2 className="text-h2 mb-3 install-heading">Links</h2>
          <div className="detail-links">
            {links.map((link) => (
              <a
                key={`${link.label}-${link.url}`}
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
      <TrustFacts tool={tool} facts={trustFacts} />
    </>
  );
}

export function ToolDetail({
  tool,
  trustProbe,
  trustFacts,
  compact = false,
  commentCount = 0,
  addMode = false,
  addMcpQueryBase = "",
  compareReturnHref = "",
  showFullPageLink = false,
}: ToolDetailProps) {
  const showSidebar = !compact && !addMode;

  const contentClass = addMode
    ? compact
      ? "detail-content compact add-mode"
      : "detail-content add-mode"
    : compact
      ? "detail-content compact"
      : "detail-content";

  const hideHeader = compact && addMode;

  return (
    <article className={`tool-detail ${contentClass}`}>
      {!hideHeader && (
      <header className="tool-detail-header detail-header">
        <ToolLogo
          name={tool.name}
          logoUrl={tool.logo_url}
          logoMonogram={tool.logo_monogram}
          status={tool.status}
          size={compact ? 48 : 64}
        />
        <div className="tool-detail-heading detail-header-text">
          <div className="detail-header-row">
            <h1 className={compact ? "text-h2 detail-title" : "text-h1 detail-title"}>
              {tool.name}
            </h1>
            <div className="detail-header-actions">
              {!addMode && !compact && <ToolDetailBookmark tool={tool} />}
              {!addMode && addMcpQueryBase && (
                <AddMcpAction
                  tool={tool}
                  hrefSource={{ kind: "query_base", base: addMcpQueryBase }}
                  variant="detail_primary"
                />
              )}
            </div>
          </div>
          <div className="tool-detail-badges tool-badges">
            {!(tool.official_team && tool.status === "official") && (
              <Badge variant={statusVariant(tool.status)}>{statusBadgeLabel(tool.status)}</Badge>
            )}
            {tool.official_team && (
              <Badge variant="official">Official: {tool.official_team}</Badge>
            )}
            <Badge variant={tool.type === "x402" ? "x402" : "neutral"}>
              {tool.type.toUpperCase()}
            </Badge>
          </div>
          <div className="detail-filter-chips detail-tags">
            <DetailFilterChip
              href={singleFilterHref("function", tool.function)}
              label={tool.function}
            />
            <DetailFilterChip
              href={singleFilterHref("asset_class", tool.asset_class)}
              label={tool.asset_class}
            />
            <DetailFilterChip href={singleFilterHref("actor", tool.actor)} label={tool.actor} />
          </div>
          {!addMode && (
            <div className="tool-detail-stats">
              {compact && (
                <>
                  <span className="tool-detail-stat" title="GitHub stars">
                    <Star size={14} aria-hidden />
                    {formatGithubStars(tool.stars)}
                  </span>
                  <span className="tool-meta-sep" aria-hidden>
                    ·
                  </span>
                </>
              )}
              <span className="tool-detail-stat">
                <MessageCircle size={14} aria-hidden />
                {commentCount} comments
              </span>
              {compact && (
                <>
                  <span className="tool-meta-sep" aria-hidden>
                    ·
                  </span>
                  <span className="tool-detail-stat">
                    updated {timeAgo(tool.last_commit_at || tool.updated_at)}
                  </span>
                </>
              )}
            </div>
          )}
        </div>
      </header>
      )}

      {showSidebar ? (
        <div className="detail-layout">
          <div className="detail-main min-w-0">
            <DetailMainContent
              tool={tool}
              trustProbe={trustProbe}
              trustFacts={trustFacts}
              compact={compact}
              addMode={addMode}
              compareReturnHref={compareReturnHref}
            />
          </div>
          <DetailAboutSidebar tool={tool} />
        </div>
      ) : (
        <DetailMainContent
          tool={tool}
          trustProbe={trustProbe}
          trustFacts={trustFacts}
          compact={compact}
          addMode={addMode}
          compareReturnHref={compareReturnHref}
        />
      )}

      {showFullPageLink && !addMode && (
        <p className="mt-4">
          <Link href={`/tools/${tool.slug}`} className="text-tertiary">
            View full page
          </Link>
        </p>
      )}
    </article>
  );
}