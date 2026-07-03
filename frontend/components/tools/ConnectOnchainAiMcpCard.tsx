"use client";

import { useState } from "react";
import Link from "next/link";
import { ConnectGuideBlocks } from "@/components/connect/ConnectGuideBlocks";
import {
  CONNECT_CARD_CLIENTS,
  DEFAULT_CONNECT_CLIENT,
  buildOnchainaiConnectGuide,
  connectClientLabel,
  type ConnectCardClient,
} from "@/lib/mcp-connect";

interface ConnectOnchainAiMcpCardProps {
  /** Kept for PromoCards API compatibility; Phase 9 uses canonical HTTP endpoint. */
  mcpEndpoint?: string;
}

export function ConnectOnchainAiMcpCard(_props: ConnectOnchainAiMcpCardProps) {
  const [client, setClient] = useState<ConnectCardClient>(DEFAULT_CONNECT_CLIENT);
  const guide = buildOnchainaiConnectGuide(client);

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
        role="tablist"
        aria-label="Connect OnchainAI MCP client"
      >
        {CONNECT_CARD_CLIENTS.map((value) => (
          <button
            key={value}
            type="button"
            role="tab"
            id={`connect-tab-${value}`}
            aria-selected={client === value}
            aria-controls={`connect-panel-${value}`}
            className={
              client === value ? "install-platform-btn active" : "install-platform-btn"
            }
            onClick={() => setClient(value)}
          >
            {connectClientLabel(value)}
          </button>
        ))}
        <Link
          href="/connect"
          className="install-platform-btn connect-more-tab"
          role="tab"
          data-testid="connect-more-clients-link"
        >
          More →
        </Link>
      </div>
      <div
        id={`connect-panel-${client}`}
        role="tabpanel"
        aria-labelledby={`connect-tab-${client}`}
        className="mt-4 min-w-0"
      >
        <ConnectGuideBlocks blocks={guide.blocks} />
      </div>
    </div>
  );
}