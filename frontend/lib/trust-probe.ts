import type { StaleTrustBadge } from "@/lib/api";
import { timeAgo } from "@/lib/format";

export type TrustProbeTone = "live" | "stale" | "dead" | "unknown";

export function trustProbeTone(badge: StaleTrustBadge): TrustProbeTone {
  if (badge.live && !badge.stale) return "live";
  if (badge.stale) return "stale";
  if (!badge.live) return "dead";
  return "unknown";
}

export function trustProbeStatusLabel(badge: StaleTrustBadge): string {
  const tone = trustProbeTone(badge);
  switch (tone) {
    case "live":
      return "LIVE";
    case "stale":
      return "Stale";
    case "dead":
      return "Not live";
    default:
      return "Unknown";
  }
}

export function trustProbeLastCheckedLabel(badge: StaleTrustBadge): string {
  if (!badge.last_probe_at) return "No scheduled probe on record";
  return `Last probe ${timeAgo(badge.last_probe_at)}`;
}

export function toolShowsTrustProbe(
  tool: { pricing: string },
  trustProbe?: StaleTrustBadge | null,
): trustProbe is StaleTrustBadge {
  return Boolean(trustProbe) || tool.pricing === "x402";
}