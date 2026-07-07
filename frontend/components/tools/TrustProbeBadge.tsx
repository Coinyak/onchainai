import Link from "next/link";
import { Activity, AlertTriangle, Clock } from "lucide-react";
import type { StaleTrustBadge } from "@/lib/api";
import {
  trustProbeLastCheckedLabel,
  trustProbeStatusLabel,
  trustProbeTone,
  type TrustProbeTone,
} from "@/lib/trust-probe";

interface TrustProbeBadgeProps {
  trustProbe: StaleTrustBadge;
  /** Compact layout for compare matrix cells. */
  variant?: "default" | "compact";
  className?: string;
}

function toneClass(tone: TrustProbeTone): string {
  switch (tone) {
    case "live":
      return "trust-probe-badge--live";
    case "stale":
      return "trust-probe-badge--stale";
    case "dead":
      return "trust-probe-badge--dead";
    default:
      return "trust-probe-badge--unknown";
  }
}

function StatusIcon({ tone }: { tone: TrustProbeTone }) {
  if (tone === "live") {
    return <Activity size={16} aria-hidden className="trust-probe-badge-icon" />;
  }
  if (tone === "stale" || tone === "dead") {
    return <AlertTriangle size={16} aria-hidden className="trust-probe-badge-icon" />;
  }
  return <Clock size={16} aria-hidden className="trust-probe-badge-icon" />;
}

export function TrustProbeBadge({
  trustProbe,
  variant = "default",
  className = "",
}: TrustProbeBadgeProps) {
  const tone = trustProbeTone(trustProbe);
  const statusLabel = trustProbeStatusLabel(trustProbe);
  const lastChecked = trustProbeLastCheckedLabel(trustProbe);
  const rootClass = [
    "trust-probe-badge",
    toneClass(tone),
    variant === "compact" ? "trust-probe-badge--compact" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <aside
      className={rootClass}
      aria-label="x402 endpoint probe status"
      data-testid="trust-probe-badge"
      data-probe-tone={tone}
      data-probe-stale={trustProbe.stale ? "true" : "false"}
    >
      <div className="trust-probe-badge-header">
        <StatusIcon tone={tone} />
        <div className="trust-probe-badge-title-wrap">
          <p className="trust-probe-badge-eyebrow">x402 endpoint probe</p>
          <p className="trust-probe-badge-status">
            <span className="trust-probe-badge-pill">{statusLabel}</span>
            {trustProbe.stale && (
              <span className="trust-probe-badge-stale-tag">
                Stale (&gt;{trustProbe.stale_threshold_hours}h)
              </span>
            )}
          </p>
        </div>
      </div>

      <p className="trust-probe-badge-meta">{lastChecked}</p>
      {trustProbe.latest_probe_status && (
        <p className="trust-probe-badge-meta">
          Latest status: <code>{trustProbe.latest_probe_status}</code>
        </p>
      )}

      {variant === "default" && (
        <>
          <p className="trust-probe-badge-cost">{trustProbe.skip_cost.message}</p>
          <p className="trust-probe-badge-hint">{trustProbe.k2_conversion_reason}</p>
          <p className="trust-probe-badge-footnote">
            Free catalog data may be up to {trustProbe.stale_threshold_hours} hours old. For a
            payment-time attestation, call{" "}
            <code>{trustProbe.fresh_probe_tool}</code> via{" "}
            <Link href="/connect#k2-probe-receipt" className="trust-probe-badge-link">
              K2 probe receipt guide
            </Link>{" "}
            (x402 micropayment; metadata only — we do not connect wallets or move funds).
          </p>
        </>
      )}

      {variant === "compact" && (
        <>
          <p className="trust-probe-badge-hint trust-probe-badge-hint--compact">
            {trustProbe.k2_conversion_reason}
          </p>
          <p className="trust-probe-badge-meta">
            <Link href="/connect#k2-probe-receipt" className="trust-probe-badge-link">
              K2 receipt guide
            </Link>
          </p>
        </>
      )}
    </aside>
  );
}