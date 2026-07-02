"use client";

import { useState } from "react";
import type { Tool } from "@/lib/api";
import { displayInstallCommand } from "@/lib/format";
import { HighlightedCommand } from "@/components/tools/HighlightedCommand";

type InstallTab = "generic" | "claude" | "cursor";

interface InstallSectionProps {
  tool: Tool;
}

export function InstallSection({ tool }: InstallSectionProps) {
  const [tab, setTab] = useState<InstallTab>("generic");
  const generic = displayInstallCommand(tool);

  const tabs: { id: InstallTab; label: string }[] = [
    { id: "claude", label: "Claude" },
    { id: "cursor", label: "Cursor" },
    { id: "generic", label: "Generic" },
  ];

  return (
    <section className="detail-section">
      <h2 className="text-h2 mb-3">Install</h2>
      <div className="install-tabs" role="tablist">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            role="tab"
            aria-selected={tab === t.id}
            className={tab === t.id ? "install-tab active" : "install-tab"}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div className="install-tab-panel">
        {tab === "generic" && generic && <HighlightedCommand command={generic} />}
        {tab === "claude" && tool.mcp_endpoint && (
          <HighlightedCommand
            command={`Add MCP server: ${tool.mcp_endpoint}`}
          />
        )}
        {tab === "cursor" && generic && (
          <HighlightedCommand command={generic} />
        )}
        {!generic && !tool.mcp_endpoint && (
          <p className="text-body-sm text-secondary">No install command available.</p>
        )}
      </div>
    </section>
  );
}