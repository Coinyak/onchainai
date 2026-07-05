"use client";

import { useLayoutEffect, useRef, useState } from "react";
import Link from "next/link";
import { Star } from "lucide-react";
import type { PublicTool } from "@/lib/api";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";
import { ChainLogo } from "@/components/tools/ChainLogo";
import { InstallGuidePanel } from "@/components/tools/InstallGuidePanel";
import { TrustFacts } from "@/components/tools/TrustFacts";
import { PreviewCommentsPreview } from "@/components/tools/PreviewCommentsPreview";
import { chainTagsForTool } from "@/lib/chains";
import { timeAgo, statusBadgeLabel, formatGithubStars } from "@/lib/format";
import { toolHasInstallPath } from "@/lib/install-guide";

const PREVIEW_CHAINS_MAX = 8;

interface PreviewPanelContentProps {
  tool: PublicTool;
  closeHref: string;
  fullPageHref: string;
  commentCount: number;
}

function statusVariant(status: string): "verified" | "official" | "community" {
  if (status === "verified") return "verified";
  if (status === "official") return "official";
  return "community";
}

function formatRisk(level: string): string {
  if (!level) return "—";
  return level.charAt(0).toUpperCase() + level.slice(1);
}

function QuickFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="preview-quick-fact">
      <span className="preview-quick-fact-label">{label}</span>
      <span className="preview-quick-fact-value">{value}</span>
    </div>
  );
}

export function PreviewPanelContent({
  tool,
  closeHref,
  fullPageHref,
  commentCount,
}: PreviewPanelContentProps) {
  const [descExpanded, setDescExpanded] = useState(false);
  const [descOverflows, setDescOverflows] = useState(false);
  const descRef = useRef<HTMLParagraphElement>(null);
  const chains = chainTagsForTool(tool.chains);
  const visibleChains = chains.slice(0, PREVIEW_CHAINS_MAX);
  const extraChains = Math.max(0, chains.length - PREVIEW_CHAINS_MAX);
  const updated = timeAgo(tool.last_commit_at || tool.updated_at);
  const source = tool.official_team || tool.source || "—";
  const license = tool.license?.trim() || "—";

  useLayoutEffect(() => {
    const el = descRef.current;
    if (!el || descExpanded || !tool.description) {
      setDescOverflows(false);
      return;
    }
    setDescOverflows(el.scrollHeight > el.clientHeight + 1);
  }, [tool.description, descExpanded]);

  return (
    <>
      <header className="preview-panel-header">
        <ToolLogo
          name={tool.name}
          logoUrl={tool.logo_url}
          logoMonogram={tool.logo_monogram}
          status={tool.status}
          size={48}
        />
        <div className="preview-header-main">
          <h2 className="preview-title">{tool.name}</h2>
          <div className="preview-badges tool-badges">
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
          <p className="preview-stars">
            <Star size={14} aria-hidden />
            {formatGithubStars(tool.stars)}
          </p>
        </div>
        <Link href={closeHref} scroll={false} className="preview-close" aria-label="Close preview">
          ×
        </Link>
      </header>

      <div className="preview-quick-facts">
        <QuickFact label="License" value={license} />
        <QuickFact label="Updated" value={updated} />
        <QuickFact label="Install risk" value={formatRisk(tool.install_risk_level)} />
        <QuickFact label="Source" value={source} />
      </div>

      {toolHasInstallPath(tool) && (
        <div className="preview-install-wrap">
          <InstallGuidePanel tool={tool} compact />
        </div>
      )}

      <TrustFacts tool={tool} variant="preview" />

      {chains.length > 0 && (
        <div className="preview-chains" aria-label="Supported chains">
          {visibleChains.map((chain) => (
            <ChainLogo
              key={chain.id}
              id={chain.id}
              label={chain.label}
              size={20}
              className="chain-logo chain-logo-tag"
            />
          ))}
          {extraChains > 0 && (
            <span className="chain-pill chain-more" title={`${extraChains} more chains`}>
              +{extraChains}
            </span>
          )}
        </div>
      )}

      {tool.description && (
        <section className="preview-description">
          <h3 className="preview-section-heading">Description</h3>
          <p
            ref={descRef}
            className={descExpanded ? "preview-desc" : "preview-desc preview-desc-clamped"}
          >
            {tool.description}
          </p>
          {descOverflows && !descExpanded && (
            <button
              type="button"
              className="preview-desc-more"
              onClick={() => setDescExpanded(true)}
            >
              more
            </button>
          )}
        </section>
      )}

      <p className="preview-category-link">
        <Link href={`/categories/${encodeURIComponent(tool.function)}`}>
          More in {tool.function} →
        </Link>
      </p>

      <PreviewCommentsPreview
        slug={tool.slug}
        commentCount={commentCount}
        fullPageHref={fullPageHref}
      />
    </>
  );
}