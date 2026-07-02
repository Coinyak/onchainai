"use client";

import Link from "next/link";
import { Star, MessageCircle } from "lucide-react";
import type { Tool } from "@/lib/api";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";
import { HighlightedCommand } from "@/components/tools/HighlightedCommand";
import { ChainLogo } from "@/components/tools/ChainLogo";
import { chainTagsForTool } from "@/lib/chains";
import { timeAgo, statusBadgeLabel, displayInstallCommand } from "@/lib/format";

const CHAINS_VISIBLE_DESKTOP = 5;
const CHAINS_VISIBLE_MOBILE = 3;

interface ToolCardProps {
  tool: Tool;
  previewHref: string;
  isSelected?: boolean;
  commentCount?: number;
}

function statusVariant(status: string): "verified" | "official" | "community" {
  if (status === "verified") return "verified";
  if (status === "official") return "official";
  return "community";
}

export function ToolCard({ tool, previewHref, isSelected = false, commentCount = 0 }: ToolCardProps) {
  const href = `/tools/${tool.slug}`;
  const chains = chainTagsForTool(tool.chains);
  const installCmd = displayInstallCommand(tool);
  const meta = [
    tool.official_team || tool.source,
    timeAgo(tool.last_commit_at || tool.updated_at),
    tool.license,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <article className={isSelected ? "tool-card is-selected" : "tool-card"}>
      <Link href={previewHref} className="tool-card-link no-underline text-inherit">
        <div className="tool-card-inner">
          <ToolLogo
            name={tool.name}
            logoUrl={tool.logo_url}
            logoMonogram={tool.logo_monogram}
            size={48}
          />
          <div className="tool-card-body">
            <div className="tool-card-header">
              <h3 className="tool-card-title">
                <Link href={href} className="tool-card-name-link" onClick={(e) => e.stopPropagation()}>
                  {tool.name}
                </Link>
              </h3>
              <div className="tool-card-badges">
                <Badge variant={statusVariant(tool.status)}>{statusBadgeLabel(tool.status)}</Badge>
                <Badge variant={tool.type === "x402" ? "x402" : "neutral"}>
                  {tool.type.toUpperCase()}
                </Badge>
              </div>
            </div>
            {tool.description && <p className="tool-card-desc">{tool.description}</p>}
            <p className="tool-card-meta">{meta}</p>
            <div className="tool-card-chains">
              <span className="chains-desktop">
                {chains.slice(0, CHAINS_VISIBLE_DESKTOP).map((c) => (
                  <span key={c.id} className="chain-tag" title={c.label}>
                    <ChainLogo id={c.id} label={c.label} size={18} />
                  </span>
                ))}
                {chains.length > CHAINS_VISIBLE_DESKTOP && (
                  <span className="chain-tag-more">+{chains.length - CHAINS_VISIBLE_DESKTOP}</span>
                )}
              </span>
              <span className="chains-mobile">
                {chains.slice(0, CHAINS_VISIBLE_MOBILE).map((c) => (
                  <span key={c.id} className="chain-tag" title={c.label}>
                    <ChainLogo id={c.id} label={c.label} size={18} />
                  </span>
                ))}
                {chains.length > CHAINS_VISIBLE_MOBILE && (
                  <span className="chain-tag-more">+{chains.length - CHAINS_VISIBLE_MOBILE}</span>
                )}
              </span>
            </div>
            <div className="tool-card-stats">
              <span className="tool-stat">
                <Star size={14} aria-hidden /> {tool.stars}
              </span>
              <span className="tool-stat">
                <MessageCircle size={14} aria-hidden /> {commentCount}
              </span>
            </div>
            {installCmd && (
              <div className="tool-card-install desktop-only">
                <HighlightedCommand command={installCmd} />
              </div>
            )}
          </div>
        </div>
      </Link>
    </article>
  );
}