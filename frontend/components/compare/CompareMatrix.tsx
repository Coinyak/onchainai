"use client";

import Link from "next/link";
import { Star, X } from "lucide-react";
import type { ToolComparisonView } from "@/lib/api";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { ChainLogo } from "@/components/tools/ChainLogo";
import { TrustProbeBadge } from "@/components/tools/TrustProbeBadge";
import { chainTagsForTool } from "@/lib/chains";
import {
  COMPARE_ROWS,
  MAX_COMPARE_TOOLS,
  type CompareRowKey,
  compareCellText,
  rowValuesDiffer,
  sharedChainIds,
} from "@/lib/compare";
import { CompareAddToolTypeahead } from "@/components/compare/CompareAddToolTypeahead";

interface CompareMatrixProps {
  entries: ToolComparisonView[];
  onRemove: (slug: string) => void;
  onAdd: (slug: string) => void;
}

function MatrixCell({
  entry,
  rowKey,
  diff,
  sharedChains,
}: {
  entry: ToolComparisonView;
  rowKey: CompareRowKey;
  diff: boolean;
  sharedChains: Set<string>;
}) {
  const tool = entry.tool;
  const className = `compare-matrix-cell${diff ? " compare-matrix-cell-diff" : ""}`;

  if (rowKey === "stars") {
    return (
      <td className={className}>
        <span className="compare-matrix-stars" title="GitHub stars">
          <Star size={14} aria-hidden />
          {tool.stars}
        </span>
      </td>
    );
  }

  if (rowKey === "chains") {
    const chains = chainTagsForTool(tool.chains);
    if (chains.length === 0) {
      return <td className={className}>—</td>;
    }
    return (
      <td className={className}>
        <div className="compare-matrix-chains">
          {chains.map((chain) => (
            <span
              key={chain.id}
              className={`compare-matrix-chain-tag${
                sharedChains.has(chain.id) ? " compare-matrix-chain-shared" : ""
              }`}
            >
              <ChainLogo id={chain.id} label={chain.label} size={16} decorative />
              {chain.label}
            </span>
          ))}
        </div>
      </td>
    );
  }

  return <td className={className}>{compareCellText(tool, rowKey)}</td>;
}

function ProbeMatrixCell({ entry }: { entry: ToolComparisonView }) {
  if (!entry.trust_probe) {
    return <td className="compare-matrix-cell">—</td>;
  }
  return (
    <td className="compare-matrix-cell compare-matrix-probe-cell">
      <TrustProbeBadge trustProbe={entry.trust_probe} variant="compact" />
    </td>
  );
}

export function CompareMatrix({ entries, onRemove, onAdd }: CompareMatrixProps) {
  const tools = entries.map((entry) => entry.tool);
  const intersection = sharedChainIds(tools);
  const canAdd = tools.length < MAX_COMPARE_TOOLS;
  const showProbeRow = entries.some((entry) => entry.trust_probe);

  return (
    <div className="compare-matrix-wrap" data-testid="compare-matrix">
      <table className="compare-matrix">
        <thead>
          <tr>
            <th scope="col" className="compare-matrix-corner">
              Attribute
            </th>
            {entries.map((entry) => (
              <th key={entry.tool.slug} scope="col" className="compare-matrix-tool-col">
                <div className="compare-matrix-tool-header">
                  <div className="compare-matrix-tool-identity">
                    <ToolLogo
                      name={entry.tool.name}
                      logoUrl={entry.tool.logo_url}
                      logoMonogram={entry.tool.logo_monogram}
                      status={entry.tool.status}
                      size={36}
                    />
                    <div>
                      <Link href={`/tools/${entry.tool.slug}`} className="compare-matrix-tool-name">
                        {entry.tool.name}
                      </Link>
                    </div>
                  </div>
                  <button
                    type="button"
                    className="compare-matrix-remove"
                    aria-label={`Remove ${entry.tool.name} from comparison`}
                    data-testid={`compare-remove-${entry.tool.slug}`}
                    onClick={() => onRemove(entry.tool.slug)}
                  >
                    <X size={18} aria-hidden />
                  </button>
                </div>
              </th>
            ))}
            {canAdd && (
              <th scope="col" className="compare-matrix-add-col">
                <CompareAddToolTypeahead
                  selectedSlugs={tools.map((tool) => tool.slug)}
                  onSelect={onAdd}
                />
              </th>
            )}
          </tr>
        </thead>
        <tbody>
          {COMPARE_ROWS.map((row) => {
            const diff = rowValuesDiffer(tools, row.key);
            return (
              <tr key={row.key}>
                <th scope="row" className="compare-matrix-row-label">
                  {row.label}
                </th>
                {entries.map((entry) => (
                  <MatrixCell
                    key={`${row.key}-${entry.tool.slug}`}
                    entry={entry}
                    rowKey={row.key}
                    diff={diff}
                    sharedChains={intersection}
                  />
                ))}
                {canAdd && <td className="compare-matrix-add-spacer" aria-hidden />}
              </tr>
            );
          })}
          {showProbeRow && (
            <tr data-testid="compare-matrix-probe-row">
              <th scope="row" className="compare-matrix-row-label">
                x402 probe
              </th>
              {entries.map((entry) => (
                <ProbeMatrixCell key={`probe-${entry.tool.slug}`} entry={entry} />
              ))}
              {canAdd && <td className="compare-matrix-add-spacer" aria-hidden />}
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}