"use client";

import { useMemo, useState } from "react";
import type { Tool } from "@/lib/api";
import { ConnectGuideBlocks } from "@/components/connect/ConnectGuideBlocks";
import { CodingClientLogo } from "@/components/tools/CodingClientLogo";
import { logoIdForToolInstallClient } from "@/lib/coding-clients";
import {
  TOOL_INSTALL_CLIENTS,
  buildPublicInstallGuide,
  toolInstallClientLabel,
  type PublicInstallGuide,
  type ToolInstallClient,
} from "@/lib/install-guide";

const COMPACT_INSTALL_CLIENTS: ToolInstallClient[] = ["cursor", "vscode", "generic", "more"];

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
  const [client, setClient] = useState<ToolInstallClient>(compact ? "cursor" : "generic");
  const [copyRevealed, setCopyRevealed] = useState(false);
  const clientOptions = compact ? COMPACT_INSTALL_CLIENTS : TOOL_INSTALL_CLIENTS;

  const guide = useMemo(
    () => buildPublicInstallGuide(tool, tool.slug, client),
    [tool, client],
  );

  const blocks = useMemo(() => {
    const raw = guide.connect_blocks ?? [];
    if (!compact) return raw;
    return raw.filter((block) => block.title !== "Stdio bridge").slice(0, 1);
  }, [compact, guide.connect_blocks]);
  return (
    <section
      className={`install-section install-guide-panel${compact ? " install-guide-panel-compact" : ""}`}
      aria-labelledby="install-guide-heading"
    >
      {showProgress && (
        <p className="install-progress-hint text-body-sm text-secondary mb-3" role="status">
          {compact
            ? "Pick a client and copy the config."
            : "Choose a client, review install risk, then copy the config."}
        </p>
      )}
      <h3 id="install-guide-heading" className={compact ? "preview-section-heading" : "install-heading"}>
        {compact ? "Install" : "Safe install"}
      </h3>
      <div className="install-platform-group" role="tablist" aria-label="Choose client">
        {clientOptions.map((value) => (
          <button
            key={value}
            type="button"
            role="tab"
            aria-selected={client === value}
            className={client === value ? "install-platform-btn active" : "install-platform-btn"}
            data-testid={value === "more" ? "install-more-tab" : undefined}
            onClick={() => {
              setClient(value);
              setCopyRevealed(false);
            }}
          >
            {(() => {
              const logoId = logoIdForToolInstallClient(value);
              return (
                <>
                  {logoId && (
                    <CodingClientLogo
                      id={logoId}
                      label={toolInstallClientLabel(value)}
                      size={16}
                      decorative
                    />
                  )}
                  <span>{toolInstallClientLabel(value)}</span>
                </>
              );
            })()}
          </button>
        ))}
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