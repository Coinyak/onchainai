"use client";

import Link from "next/link";
import type { ToolComparisonView } from "@/lib/api";
import { InstallGuidePanel } from "@/components/tools/InstallGuidePanel";
import { AddMcpAction } from "@/components/tools/AddMcpAction";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { toolHasInstallPath } from "@/lib/install-guide";

interface CompareInstallSectionsProps {
  entries: ToolComparisonView[];
  slugs: string[];
}

export function CompareInstallSections({ entries, slugs }: CompareInstallSectionsProps) {
  if (entries.length === 0) return null;

  return (
    <section className="compare-install-sections" aria-label="Install guides">
      <h2 className="compare-install-heading">Install guides</h2>
      {entries.map((entry) => {
        const tool = entry.tool;
        return (
          <details
            key={tool.slug}
            className="compare-install-section"
            data-testid={`compare-install-${tool.slug}`}
          >
            <summary className="compare-install-summary">
              <ToolLogo
                name={tool.name}
                logoUrl={tool.logo_url}
                logoMonogram={tool.logo_monogram}
                size={28}
              />
              <span className="compare-install-summary-text">
                <span className="compare-install-summary-name">{tool.name}</span>
                <span className="compare-install-summary-hint">Safe install steps</span>
              </span>
            </summary>
            <div className="compare-install-body">
              <InstallGuidePanel tool={tool} compact />
              <div className="compare-card-actions">
                <Link href={`/tools/${tool.slug}`}>Open details</Link>
                {toolHasInstallPath(tool) && (
                  <AddMcpAction
                    tool={tool}
                    hrefSource={{ kind: "compare_slugs", slugs }}
                    variant="inline_button"
                  />
                )}
                <Link href="/toolkit">Open toolkit</Link>
              </div>
            </div>
          </details>
        );
      })}
    </section>
  );
}