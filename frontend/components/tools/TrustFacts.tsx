import type { PublicTool, TrustFact } from "@/lib/api";
import { timeAgo } from "@/lib/format";
import { Check } from "lucide-react";

interface TrustFactsProps {
  tool: PublicTool;
  facts?: TrustFact[];
  variant?: "default" | "preview";
}

function buildDefaultFacts(tool: PublicTool): TrustFact[] {
  const facts: TrustFact[] = [];
  if (tool.status === "verified" || tool.status === "official") {
    facts.push({ label: "Verified", detail: `${tool.status} listing`, severity: "positive" });
  }
  if (tool.official_team && tool.status === "official") {
    facts.push({
      label: "Official team",
      detail: tool.official_team,
      severity: "positive",
    });
  }
  if (tool.last_commit_at) {
    facts.push({
      label: "Active",
      detail: `${timeAgo(tool.last_commit_at)} commit`,
      severity: "neutral",
    });
  }
  if (tool.stars > 0) {
    facts.push({
      label: "GitHub stars",
      detail: String(tool.stars),
      severity: "neutral",
    });
  }
  if (tool.license) {
    facts.push({ label: "License", detail: tool.license, severity: "neutral" });
  }
  return facts;
}

export function TrustFacts({ tool, facts, variant = "default" }: TrustFactsProps) {
  const allItems = facts?.length ? facts : buildDefaultFacts(tool);
  const items = variant === "preview" ? allItems.slice(0, 3) : allItems;
  if (!items.length) return null;

  if (variant === "preview") {
    return (
      <ul className="preview-trust-facts" aria-label="Trust summary">
        {items.map((fact) => (
          <li key={`${fact.label}-${fact.detail}`} className="preview-trust-fact">
            <Check size={14} className="preview-trust-fact-icon" aria-hidden />
            <span>
              <strong>{fact.label}</strong>
              {fact.detail && <> · {fact.detail}</>}
            </span>
          </li>
        ))}
      </ul>
    );
  }

  return (
    <section className="detail-section">
      <h2 className="text-h2 mb-3">Trust</h2>
      <ul className="trust-facts-list">
        {items.map((fact) => (
          <li key={`${fact.label}-${fact.detail}`} className="trust-fact-item">
            <Check size={14} className="trust-fact-icon" aria-hidden />
            <span>
              <strong>{fact.label}</strong>
              {fact.detail && <> · {fact.detail}</>}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}