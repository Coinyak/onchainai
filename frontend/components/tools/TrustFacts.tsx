import type { TrustFact, Tool } from "@/lib/api";
import { timeAgo } from "@/lib/format";
import { Check } from "lucide-react";

interface TrustFactsProps {
  tool: Tool;
  facts?: TrustFact[];
}

function buildDefaultFacts(tool: Tool): TrustFact[] {
  const facts: TrustFact[] = [];
  if (tool.status === "verified" || tool.status === "official") {
    facts.push({ label: "Verified", detail: `${tool.status} listing`, severity: "positive" });
  }
  if (tool.official_team) {
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

export function TrustFacts({ tool, facts }: TrustFactsProps) {
  const items = facts?.length ? facts : buildDefaultFacts(tool);
  if (!items.length) return null;

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