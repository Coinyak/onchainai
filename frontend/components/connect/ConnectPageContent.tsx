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

const PLUGIN_MARKETPLACE_CMD = "/plugin marketplace add hoyeon4315-cpu/onchainai";
const PLUGIN_INSTALL_CMD = "/plugin install onchainai@onchainai";
const GITHUB_CONNECT_GUIDE =
  "https://github.com/hoyeon4315-cpu/onchainai/blob/main/docs/CONNECT.md";

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

      <section
        className="connect-plugin-section mb-8"
        aria-labelledby="connect-plugin-heading"
        data-testid="connect-plugin-card"
      >
        <h2 id="connect-plugin-heading" className="text-h3 font-semibold mb-3">
          Claude Code plugin &amp; skill
        </h2>
        <p className="text-secondary text-body-md leading-relaxed mb-4 max-w-[720px]">
          Install the official plugin for MCP auto-connect, <code className="text-code">/find-tool</code>,
          and the crypto-tools skill. Or copy the skill folder alone if you already have MCP wired.
        </p>
        <div className="connect-guide-block">
          <h3 className="connect-guide-block-title">Plugin (recommended)</h3>
          <ol className="install-steps">
            <li>In Claude Code, add this marketplace once.</li>
            <li>Install the plugin — MCP + skill + command ship together.</li>
            <li>Restart Claude Code and run <code className="text-code">/mcp</code> to confirm.</li>
          </ol>
          <div className="tool-install-stack mt-3">
            <div className="tool-install">
              <HighlightedCommand command={PLUGIN_MARKETPLACE_CMD} showCopy={false} />
              <CopyButton
                text={PLUGIN_MARKETPLACE_CMD}
                label={copyLabelAria("Copy marketplace command")}
              />
            </div>
          </div>
          <div className="tool-install-stack mt-3">
            <div className="tool-install">
              <HighlightedCommand command={PLUGIN_INSTALL_CMD} showCopy={false} />
              <CopyButton
                text={PLUGIN_INSTALL_CMD}
                label={copyLabelAria("Copy install command")}
              />
            </div>
          </div>
        </div>
        <div className="connect-guide-block mt-6">
          <h3 className="connect-guide-block-title">Skill only</h3>
          <p className="text-body-sm text-secondary mb-2">
            Copy{" "}
            <code className="text-code">plugin/onchainai/skills/onchainai-crypto-tools/</code> into{" "}
            <code className="text-code">~/.claude/skills/</code> (or upload to any Agent Skills runtime).
            The skill expects the <code className="text-code">onchainai</code> MCP server above.
          </p>
          <p className="text-body-sm text-secondary">
            Full guide:{" "}
            <a href={GITHUB_CONNECT_GUIDE} className="text-primary underline-offset-2 hover:underline">
              docs/CONNECT.md
            </a>
            {" · "}
            <a href="/llms.txt" className="text-primary underline-offset-2 hover:underline">
              llms.txt
            </a>{" "}
            for agent discovery.
          </p>
        </div>
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