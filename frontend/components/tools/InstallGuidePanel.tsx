"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import type { Tool } from "@/lib/api";
import { ConnectGuideBlocks } from "@/components/connect/ConnectGuideBlocks";
import {
  TOOL_INSTALL_CLIENTS,
  buildPublicInstallGuide,
  toolInstallClientLabel,
  type PublicInstallGuide,
  type ToolInstallClient,
} from "@/lib/install-guide";

interface InstallGuidePanelProps {
  tool: Tool;
  compact?: boolean;
  showProgress?: boolean;
}

export function InstallGuidePanel({
  tool,
  compact = false,
  showProgress = false,
}: InstallGuidePanelProps) {
  const [client, setClient] = useState<ToolInstallClient>("chatgpt");
  const [copyRevealed, setCopyRevealed] = useState(false);

  const guide = useMemo(
    () => buildPublicInstallGuide(tool, tool.slug, client),
    [tool, client],
  );

  const blocks = guide.connect_blocks ?? [];
  return (
    <section
      className={`install-section install-guide-panel${compact ? " install-guide-panel-compact" : ""}`}
      aria-labelledby="install-guide-heading"
    >
      {showProgress && (
        <p className="install-progress-hint text-body-sm text-secondary mb-3" role="status">
          Choose a client, review install risk, then copy the config.
        </p>
      )}
      <h3 id="install-guide-heading" className="install-heading">
        Safe install
      </h3>
      <div className="install-platform-group" role="tablist" aria-label="Choose client">
        {TOOL_INSTALL_CLIENTS.map((value) =>
          value === "more" ? (
            <Link
              key={value}
              href="/connect"
              className="install-platform-btn"
              data-testid="install-more-clients-link"
            >
              {toolInstallClientLabel(value)} →
            </Link>
          ) : (
            <button
              key={value}
              type="button"
              role="tab"
              aria-selected={client === value}
              className={
                client === value ? "install-platform-btn active" : "install-platform-btn"
              }
              onClick={() => {
                setClient(value);
                setCopyRevealed(false);
              }}
            >
              {toolInstallClientLabel(value)}
            </button>
          ),
        )}
      </div>
      {guide.warning && (
        <p className="install-warning" role="alert">
          {guide.warning}
        </p>
      )}
      {guide.copy_gate === "blocked" ? (
        <p className="install-warning" role="alert">
          Copy is blocked for critical-risk install commands.
        </p>
      ) : guide.copy_gate === "reveal_first" && !copyRevealed ? (
        <>
          <ul className="install-steps">
            {guide.steps.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ul>
          <button
            type="button"
            className="install-reveal-btn"
            onClick={() => setCopyRevealed(true)}
          >
            Reveal copy action
          </button>
        </>
      ) : (
        <ConnectGuideBlocks blocks={blocks} moreHref="/connect" />
      )}
      <InstallGuideMeta guide={guide} />
    </section>
  );
}

function InstallGuideMeta({ guide }: { guide: PublicInstallGuide }) {
  if (!guide.x402_notice && !guide.referral_disclosure) {
    return null;
  }
  return (
    <div className="install-guide-meta mt-3">
      {guide.x402_notice && <p className="text-body-sm text-secondary">{guide.x402_notice}</p>}
      {guide.referral_disclosure && (
        <p className="text-body-sm text-secondary">{guide.referral_disclosure}</p>
      )}
    </div>
  );
}