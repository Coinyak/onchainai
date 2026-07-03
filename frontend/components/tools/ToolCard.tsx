"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useMutation } from "@tanstack/react-query";
import { ArrowLeftRight, Star } from "lucide-react";
import type { Tool } from "@/lib/api";
import { isBookmarked, setBookmark } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";
import { HighlightedCommand } from "@/components/tools/HighlightedCommand";
import { ChainLogo } from "@/components/tools/ChainLogo";
import { AddMcpAction } from "@/components/tools/AddMcpAction";
import { LoginModal } from "@/components/auth/LoginModal";
import { chainTagsForTool } from "@/lib/chains";
import { compareHref, stripPreviewParams } from "@/lib/browser-query";
import { timeAgo, statusBadgeLabel, displayInstallCommand } from "@/lib/format";

const CHAINS_VISIBLE_DESKTOP = 5;
const CHAINS_VISIBLE_MOBILE = 3;

interface ToolCardProps {
  tool: Tool;
  previewHref: string;
  queryBase?: string;
  isSelected?: boolean;
  commentCount?: number;
  initiallyStarred?: boolean;
}

function statusVariant(status: string): "verified" | "official" | "community" {
  if (status === "verified") return "verified";
  if (status === "official") return "official";
  return "community";
}

function bookmarkActionLabel(starred: boolean): string {
  return starred ? "Remove from Toolkit" : "Save to Toolkit";
}

function renderChainTags(chains: ReturnType<typeof chainTagsForTool>, extra: number, className: string) {
  return (
    <span className={className}>
      {chains.map((c) => (
        <ChainLogo key={c.id} id={c.id} label={c.label} size={20} className="chain-logo chain-logo-tag" />
      ))}
      {extra > 0 && (
        <span className="chain-pill chain-more" title={`${extra} more chains`}>
          +{extra}
        </span>
      )}
    </span>
  );
}

export function ToolCard({
  tool,
  previewHref,
  queryBase,
  isSelected = false,
  commentCount = 0,
  initiallyStarred = false,
}: ToolCardProps) {
  const { isAuthenticated } = useAuth();
  const [showLogin, setShowLogin] = useState(false);
  const [starred, setStarred] = useState(initiallyStarred);

  const href = `/tools/${tool.slug}`;
  const chains = chainTagsForTool(tool.chains);
  const chainDesktop = chains.slice(0, CHAINS_VISIBLE_DESKTOP);
  const chainMobile = chains.slice(0, CHAINS_VISIBLE_MOBILE);
  const extraDesktop = Math.max(0, chains.length - CHAINS_VISIBLE_DESKTOP);
  const extraMobile = Math.max(0, chains.length - CHAINS_VISIBLE_MOBILE);
  const installCmd = displayInstallCommand(tool);
  const team = tool.official_team || tool.source;
  const time = timeAgo(tool.last_commit_at || tool.updated_at);
  const license = tool.license?.trim() ?? "";
  const compareUrl = compareHref([tool.slug]);
  const cardQueryBase = queryBase
    ? stripPreviewParams(queryBase.split("?")[0] || "/tools", queryBase)
    : undefined;

  useEffect(() => {
    if (!isAuthenticated) return;
    let cancelled = false;
    isBookmarked(tool.slug)
      .then((bookmarked) => {
        if (!cancelled) setStarred(bookmarked);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [isAuthenticated, tool.slug]);

  const bookmarkMut = useMutation({
    mutationFn: (wantStarred: boolean) => setBookmark(tool.slug, wantStarred),
    onSuccess: (_, wantStarred) => setStarred(wantStarred),
    onError: () => setShowLogin(true),
  });

  return (
    <article className={isSelected ? "tool-card is-selected" : "tool-card"}>
      <LoginModal open={showLogin} onClose={() => setShowLogin(false)} />
      <Link
        href={previewHref}
        scroll={false}
        className="tool-card-link no-underline text-inherit"
        data-testid="tool-card-link"
      >
        <div className="tool-card-inner">
          <ToolLogo
            name={tool.name}
            logoUrl={tool.logo_url}
            logoMonogram={tool.logo_monogram}
            size={48}
          />
          <div className="tool-card-body">
            <div className="tool-card-header">
              <h3 className="tool-name">{tool.name}</h3>
              <div className="tool-badges">
                <Badge variant={statusVariant(tool.status)}>{statusBadgeLabel(tool.status)}</Badge>
                <Badge variant={tool.type === "x402" ? "x402" : "neutral"}>
                  {tool.type.toUpperCase()}
                </Badge>
              </div>
            </div>
            {tool.description && <p className="tool-desc">{tool.description}</p>}
            <div className="tool-source-line">
              {team && <span className="tool-team">{team}</span>}
              {team && time && <span className="tool-meta-sep">·</span>}
              {time && <span className="tool-time">{time}</span>}
              {license && (
                <>
                  <span className="tool-meta-sep">·</span>
                  <span className="tool-license">{license}</span>
                </>
              )}
            </div>
            <div className="tool-meta">
              {renderChainTags(chainDesktop, extraDesktop, "tool-chains tool-chains-desktop")}
              {renderChainTags(chainMobile, extraMobile, "tool-chains tool-chains-mobile")}
              <span className="tool-meta-sep">·</span>
              <span className="tool-stars" title="GitHub stars">
                <Star size={14} strokeWidth={1.75} aria-hidden />
                {tool.stars} GitHub stars
              </span>
              <span className="tool-meta-sep">·</span>
              <span className="tool-comments">comments {commentCount}</span>
            </div>
            {installCmd && (
              <div className="tool-install hidden md:flex">
                <HighlightedCommand command={installCmd} />
              </div>
            )}
          </div>
        </div>
      </Link>
      <div className="tool-card-actions">
        {cardQueryBase && (
          <AddMcpAction
            tool={tool}
            hrefSource={{ kind: "query_base", base: cardQueryBase }}
            variant="card_icon"
          />
        )}
        <button
          type="button"
          className="card-action-btn"
          aria-label={bookmarkActionLabel(starred)}
          aria-pressed={starred}
          title={bookmarkActionLabel(starred)}
          onClick={(e) => {
            e.stopPropagation();
            if (!isAuthenticated) {
              setShowLogin(true);
              return;
            }
            bookmarkMut.mutate(!starred);
          }}
        >
          <Star
            className={starred ? "bookmark-icon is-filled" : "bookmark-icon"}
            size={16}
            strokeWidth={1.75}
            fill={starred ? "currentColor" : "none"}
            aria-hidden
          />
        </button>
        <Link
          href={compareUrl}
          className="card-action-btn"
          aria-label="Compare"
          title="Compare"
          onClick={(e) => e.stopPropagation()}
        >
          <ArrowLeftRight className="card-action-icon" size={16} strokeWidth={1.75} aria-hidden />
        </Link>
      </div>
    </article>
  );
}