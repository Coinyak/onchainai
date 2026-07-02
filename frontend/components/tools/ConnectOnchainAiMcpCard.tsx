"use client";

import { useMemo, useState } from "react";
import { HighlightedCommand } from "@/components/tools/HighlightedCommand";
import { CopyButton } from "@/components/ui/CopyButton";
import {
  CONNECT_CARD_PLATFORMS,
  DEFAULT_CONNECT_PLATFORM,
  type InstallPlatform,
  buildOnchainaiConnectGuide,
  copyLabelAria,
  displayGuideText,
  platformLabel,
} from "@/lib/install-guide";

interface ConnectOnchainAiMcpCardProps {
  mcpEndpoint: string;
}

export function ConnectOnchainAiMcpCard({
  mcpEndpoint,
}: ConnectOnchainAiMcpCardProps) {
  const [platform, setPlatform] = useState<InstallPlatform>(
    DEFAULT_CONNECT_PLATFORM,
  );

  const guide = useMemo(
    () => buildOnchainaiConnectGuide(platform, mcpEndpoint),
    [platform, mcpEndpoint],
  );
  const copyText = displayGuideText(guide);
  const copyAria = copyLabelAria(guide.copy_label);

  return (
    <div
      className="promo-card border border-[#E5E5E5] rounded-lg p-6 bg-white min-w-0"
      data-testid="connect-onchainai-mcp-card"
    >
      <h3 className="text-[16px] font-semibold mb-2">Connect OnchainAI MCP</h3>
      <p className="text-[14px] text-[#6B6B6B] mb-3 leading-relaxed">
        Let your agent search OnchainAI for crypto tools.
      </p>
      <div
        className="install-platform-group connect-mcp-platforms"
        role="group"
        aria-label="Connect OnchainAI MCP client"
      >
        {CONNECT_CARD_PLATFORMS.map((value) => (
          <button
            key={value}
            type="button"
            className={
              platform === value
                ? "install-platform-btn active"
                : "install-platform-btn"
            }
            aria-pressed={platform === value}
            onClick={() => setPlatform(value)}
          >
            {platformLabel(value)}
          </button>
        ))}
      </div>
      <div className="flex items-center gap-2 min-w-0 mt-4">
        <HighlightedCommand
          command={copyText}
          showPrefix={false}
          showCopy={false}
        />
        <CopyButton text={copyText} label={copyAria} />
      </div>
    </div>
  );
}