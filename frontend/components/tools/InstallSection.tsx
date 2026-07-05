"use client";

import { InstallGuidePanel } from "@/components/tools/InstallGuidePanel";
import type { PublicTool } from "@/lib/api";
import { toolHasInstallPath } from "@/lib/install-guide";

interface InstallSectionProps {
  tool: PublicTool;
  compact?: boolean;
}

export function InstallSection({ tool, compact = false }: InstallSectionProps) {
  if (!toolHasInstallPath(tool)) {
    return (
      <section className="detail-section">
        <h2 className="text-h2 mb-3">Install</h2>
        <p className="text-body-sm text-secondary">No install command available.</p>
      </section>
    );
  }

  return <InstallGuidePanel tool={tool} compact={compact} />;
}