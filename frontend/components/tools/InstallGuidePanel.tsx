"use client";

import { useMemo, useState } from "react";
import type { Tool } from "@/lib/api";
import { HighlightedCommand } from "@/components/tools/HighlightedCommand";
import { CopyButton } from "@/components/ui/CopyButton";
import {
  ALL_SELECTABLE_PLATFORMS,
  type InstallPlatform,
  type PublicInstallGuide,
  buildPublicInstallGuide,
  copyAllowed,
  copyLabelAria,
  displayGuideText,
  platformLabel,
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
  const [platform, setPlatform] = useState<InstallPlatform>("claude");
  const [copyRevealed, setCopyRevealed] = useState(false);

  const guide = useMemo(
    () => buildPublicInstallGuide(tool, tool.slug, platform),
    [tool, platform],
  );

  const copyText = displayGuideText(guide);
  const copyAria = copyLabelAria(guide.copy_label);
  const showShellPrefix = platform === "generic_mcp";
  const canCopy = copyAllowed(guide.copy_gate, copyRevealed);

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
      <div className="install-platform-group" role="group" aria-label="Choose client">
        {ALL_SELECTABLE_PLATFORMS.map((value) => (
          <button
            key={value}
            type="button"
            className={
              platform === value ? "install-platform-btn active" : "install-platform-btn"
            }
            aria-pressed={platform === value}
            onClick={() => setPlatform(value)}
          >
            {platformLabel(value)}
          </button>
        ))}
      </div>
      {guide.warning && (
        <p className="install-warning" role="alert">
          {guide.warning}
        </p>
      )}
      {canCopy ? (
        <div className="tool-install-stack">
          <div className="tool-install">
            <HighlightedCommand
              command={copyText}
              showPrefix={showShellPrefix}
              showCopy={false}
            />
            <CopyButton text={copyText} label={copyAria} />
          </div>
        </div>
      ) : guide.copy_gate === "blocked" ? (
        <p className="install-warning" role="alert">
          Copy is blocked for critical-risk install commands.
        </p>
      ) : guide.copy_gate === "reveal_first" ? (
        <button
          type="button"
          className="install-reveal-btn"
          onClick={() => setCopyRevealed(true)}
        >
          Reveal copy action
        </button>
      ) : null}
      <ul className="install-steps">
        {guide.steps.map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ul>
      <InstallGuideMeta guide={guide} />
    </section>
  );
}

function InstallGuideMeta({ guide }: { guide: PublicInstallGuide }) {
  if (!guide.x402_notice && !guide.referral_disclosure && guide.docs_links.length === 0) {
    return null;
  }
  return (
    <div className="install-guide-meta mt-3">
      {guide.x402_notice && <p className="text-body-sm text-secondary">{guide.x402_notice}</p>}
      {guide.referral_disclosure && (
        <p className="text-body-sm text-secondary">{guide.referral_disclosure}</p>
      )}
      {guide.docs_links.length > 0 && (
        <ul className="install-docs-links">
          {guide.docs_links.map((link) => (
            <li key={link.url}>
              <a href={link.url} target="_blank" rel="noopener noreferrer" className="external-link">
                {link.label}
              </a>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}