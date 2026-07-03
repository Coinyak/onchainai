import Link from "next/link";
import { Star } from "lucide-react";
import type { Tool } from "@/lib/api";
import { singleFilterHref } from "@/lib/browser-query";
import { timeAgo } from "@/lib/format";

interface DetailAboutSidebarProps {
  tool: Tool;
}

function riskBadgeClass(level: string): string {
  if (!level) return "badge badge-neutral";
  const normalized = level.toLowerCase();
  if (normalized === "low") return "badge badge-risk-low";
  if (normalized === "medium") return "badge badge-risk-medium";
  if (normalized === "high") return "badge badge-risk-high";
  if (normalized === "critical") return "badge badge-risk-critical";
  return "badge badge-neutral";
}

function riskLabel(level: string): string {
  if (!level) return "Unknown";
  return level.charAt(0).toUpperCase() + level.slice(1).toLowerCase();
}

export function DetailAboutSidebar({ tool }: DetailAboutSidebarProps) {
  const updated = timeAgo(tool.last_commit_at || tool.updated_at);
  const license = tool.license?.trim() || "—";
  const source = tool.source?.trim() || "—";
  const riskHref = tool.install_risk_level
    ? singleFilterHref("install_risk", tool.install_risk_level)
    : undefined;

  return (
    <aside className="detail-about" aria-label="About this tool">
      <h2 className="detail-about-heading text-h2">About</h2>
      <dl className="detail-about-grid">
        <div className="detail-about-item">
          <dt className="detail-about-label">GitHub stars</dt>
          <dd className="detail-about-value">
            {tool.repo_url ? (
              <a
                href={tool.repo_url}
                target="_blank"
                rel="noopener noreferrer"
                className="detail-about-link external-link no-underline"
              >
                <Star size={14} aria-hidden />
                {tool.stars}
              </a>
            ) : (
              <span className="detail-about-inline">
                <Star size={14} aria-hidden />
                {tool.stars}
              </span>
            )}
          </dd>
        </div>
        <div className="detail-about-item">
          <dt className="detail-about-label">License</dt>
          <dd className="detail-about-value">{license}</dd>
        </div>
        <div className="detail-about-item">
          <dt className="detail-about-label">Source</dt>
          <dd className="detail-about-value">
            {tool.source_url ? (
              <a
                href={tool.source_url}
                target="_blank"
                rel="noopener noreferrer"
                className="detail-about-link external-link no-underline"
              >
                {source}
              </a>
            ) : (
              source
            )}
          </dd>
        </div>
        <div className="detail-about-item">
          <dt className="detail-about-label">Updated</dt>
          <dd className="detail-about-value">{updated}</dd>
        </div>
        <div className="detail-about-item">
          <dt className="detail-about-label">Install risk</dt>
          <dd className="detail-about-value">
            {riskHref ? (
              <Link href={riskHref} scroll={false} className="detail-about-risk-link no-underline">
                <span className={riskBadgeClass(tool.install_risk_level)}>
                  {riskLabel(tool.install_risk_level)}
                </span>
              </Link>
            ) : (
              <span className={riskBadgeClass(tool.install_risk_level)}>
                {riskLabel(tool.install_risk_level)}
              </span>
            )}
          </dd>
        </div>
      </dl>
    </aside>
  );
}