"use client";

import Link from "next/link";
import { Star, X } from "lucide-react";
import type { Tool } from "@/lib/api";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { ChainLogo } from "@/components/tools/ChainLogo";
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
  tools: Tool[];
  onRemove: (slug: string) => void;
  onAdd: (slug: string) => void;
}

function MatrixCell({
  tool,
  rowKey,
  diff,
  sharedChains,
}: {
  tool: Tool;
  rowKey: CompareRowKey;
  diff: boolean;
  sharedChains: Set<string>;
}) {
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

export function CompareMatrix({ tools, onRemove, onAdd }: CompareMatrixProps) {
  const intersection = sharedChainIds(tools);
  const canAdd = tools.length < MAX_COMPARE_TOOLS;

  return (
    <div className="compare-matrix-wrap" data-testid="compare-matrix">
      <table className="compare-matrix">
        <thead>
          <tr>
            <th scope="col" className="compare-matrix-corner">
              Attribute
            </th>
            {tools.map((tool) => (
              <th key={tool.slug} scope="col" className="compare-matrix-tool-col">
                <div className="compare-matrix-tool-header">
                  <div className="compare-matrix-tool-identity">
                    <ToolLogo
                      name={tool.name}
                      logoUrl={tool.logo_url}
                      logoMonogram={tool.logo_monogram}
                      size={36}
                    />
                    <div>
                      <Link href={`/tools/${tool.slug}`} className="compare-matrix-tool-name">
                        {tool.name}
                      </Link>
                    </div>
                  </div>
                  <button
                    type="button"
                    className="compare-matrix-remove"
                    aria-label={`Remove ${tool.name} from comparison`}
                    data-testid={`compare-remove-${tool.slug}`}
                    onClick={() => onRemove(tool.slug)}
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
                {tools.map((tool) => (
                  <MatrixCell
                    key={`${row.key}-${tool.slug}`}
                    tool={tool}
                    rowKey={row.key}
                    diff={diff}
                    sharedChains={intersection}
                  />
                ))}
                {canAdd && <td className="compare-matrix-add-spacer" aria-hidden />}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}