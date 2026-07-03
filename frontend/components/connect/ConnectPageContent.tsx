"use client";

import type { ReactNode } from "react";
import {
  Bot,
  Code,
  MessageSquare,
  MousePointerClick,
  Plug,
  Sparkles,
  Terminal,
  Wind,
} from "lucide-react";
import { ConnectGuideBlocks } from "@/components/connect/ConnectGuideBlocks";
import { HighlightedCommand } from "@/components/tools/HighlightedCommand";
import { CopyButton } from "@/components/ui/CopyButton";
import {
  CONNECT_PAGE_CLIENTS,
  ONCHAINAI_MCP_HTTP_URL,
  ONCHAINAI_MCP_UNIVERSAL_CMD,
  type ConnectPageClient,
} from "@/lib/mcp-connect";
import { copyLabelAria } from "@/lib/install-guide";

const CLIENT_ICONS: Record<ConnectPageClient["icon"], ReactNode> = {
  terminal: <Terminal size={20} strokeWidth={1.75} aria-hidden />,
  "message-square": <MessageSquare size={20} strokeWidth={1.75} aria-hidden />,
  "mouse-pointer-click": <MousePointerClick size={20} strokeWidth={1.75} aria-hidden />,
  code: <Code size={20} strokeWidth={1.75} aria-hidden />,
  wind: <Wind size={20} strokeWidth={1.75} aria-hidden />,
  sparkles: <Sparkles size={20} strokeWidth={1.75} aria-hidden />,
  bot: <Bot size={20} strokeWidth={1.75} aria-hidden />,
  plug: <Plug size={20} strokeWidth={1.75} aria-hidden />,
};

export function ConnectPageContent() {
  const universalAria = copyLabelAria("Copy command");

  return (
    <div
      className="connect-page px-gutter md:px-6 py-8 md:py-10 max-w-[960px]"
      data-testid="connect-page"
    >
      <h1 className="text-h1 font-bold mb-3">Connect OnchainAI MCP</h1>
      <p className="text-secondary text-body-md leading-relaxed mb-6 max-w-[720px]">
        Add the OnchainAI search MCP server to your agent or editor. Each client has its own
        install path below.
      </p>

      <section className="connect-universal mb-8" aria-labelledby="connect-universal-heading">
        <h2 id="connect-universal-heading" className="text-h3 font-semibold mb-3">
          Universal install
        </h2>
        <div className="tool-install-stack">
          <div className="tool-install">
            <HighlightedCommand
              command={ONCHAINAI_MCP_UNIVERSAL_CMD}
              showPrefix
              showCopy={false}
            />
            <CopyButton text={ONCHAINAI_MCP_UNIVERSAL_CMD} label={universalAria} />
          </div>
        </div>
        <p className="text-body-sm text-secondary mt-3">
          Official endpoint:{" "}
          <code className="text-code">{ONCHAINAI_MCP_HTTP_URL}</code>
        </p>
        <p className="text-body-sm text-secondary mt-2">
          The only official OnchainAI MCP endpoint is{" "}
          <code className="text-code">{ONCHAINAI_MCP_HTTP_URL}</code>.
        </p>
      </section>

      <section aria-labelledby="connect-clients-heading">
        <h2 id="connect-clients-heading" className="text-h3 font-semibold mb-4">
          MCP clients
        </h2>
        <div className="connect-client-grid">
          {CONNECT_PAGE_CLIENTS.map((client) => (
            <article
              key={client.id}
              className="connect-client-card"
              data-testid={`connect-client-${client.id}`}
            >
              <div className="connect-client-card-header">
                <span className="connect-client-icon">{CLIENT_ICONS[client.icon]}</span>
                <h3 className="connect-client-card-title">{client.label}</h3>
              </div>
              <ConnectGuideBlocks blocks={client.blocks} />
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}