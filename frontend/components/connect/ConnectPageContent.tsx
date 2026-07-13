"use client";

import { AgentLinkSection } from "@/components/connect/AgentLinkSection";
import { K2ProbeReceiptSection } from "@/components/connect/K2ProbeReceiptSection";
import { CodingClientLogo } from "@/components/tools/CodingClientLogo";
import { ConnectGuideBlocks } from "@/components/connect/ConnectGuideBlocks";
import { HighlightedCommand } from "@/components/tools/HighlightedCommand";
import { CopyButton } from "@/components/ui/CopyButton";
import {
  CONNECT_PAGE_CLIENTS,
  ONCHAINAI_MCP_HTTP_JSON,
  ONCHAINAI_MCP_HTTP_URL,
  ONCHAINAI_MCP_STDIO_JSON,
  ONCHAINAI_MCP_UNIVERSAL_CMD,
} from "@/lib/mcp-connect";
import {
  ONCHAINAI_PLUGIN_INSTALL_CMD,
  ONCHAINAI_PLUGIN_MARKETPLACE_CMD,
} from "@/lib/mcp-deeplinks";
import { copyLabelAria } from "@/lib/install-guide";
import { logoIdForConnectPageClient } from "@/lib/coding-clients";

const GITHUB_CONNECT_GUIDE =
  "https://github.com/Coinyak/onchainai/blob/main/docs/CONNECT.md";

const PLUGIN_INSTALL_BOTH = `${ONCHAINAI_PLUGIN_MARKETPLACE_CMD}\n${ONCHAINAI_PLUGIN_INSTALL_CMD}`;

export function ConnectPageContent() {
  const universalAria = copyLabelAria("Copy command");

  return (
    <div
      className="connect-page px-gutter md:px-6 py-8 md:py-10 max-w-[960px]"
      data-testid="connect-page"
    >
      <h1 className="text-h1 font-bold mb-3">Connect OnchainAI to your agent</h1>
      <p className="text-secondary text-body-md leading-relaxed mb-4 max-w-[720px]">
        One free-discovery MCP endpoint for Claude, Cursor, ChatGPT, Codex, VS Code, and more.
        Start with universal install, or pick a client below.
      </p>
      <p
        className="text-body-sm text-secondary mb-4 max-w-[720px]"
        data-testid="connect-mcp-billing-note"
      >
        Endpoint:{" "}
        <code className="text-code">{ONCHAINAI_MCP_HTTP_URL}</code> (not{" "}
        <code className="text-code">/mcp/okx</code>). Search and detail stay free. Optional
        OnchainAI premium tools may charge via x402 when enabled (~$0.01 for export / recommend /
        gap audit; ~$0.001 for endpoint health). Claude Code often cannot settle x402 — use free
        tools when unpaid. Third-party catalog x402 tools are metadata only.
      </p>
      <p
        className="text-body-sm text-secondary mb-6 max-w-[720px] connect-agent-sync-callout"
        data-testid="connect-agent-sync-callout"
      >
        <strong className="text-primary">Using a coding agent?</strong>{" "}
        <a href="#agent-sync" className="text-primary underline-offset-2 hover:underline">
          Link your agent
        </a>{" "}
        so saved tools show up in My Toolkit.
      </p>

      <section className="connect-universal mb-8" aria-labelledby="connect-universal-heading">
        <h2 id="connect-universal-heading" className="text-h3 font-semibold mb-3">
          Universal install
        </h2>
        <p className="text-body-sm text-secondary mb-3 max-w-[720px]">
          Works with most HTTP-capable MCP clients. Official endpoint:{" "}
          <code className="text-code">{ONCHAINAI_MCP_HTTP_URL}</code> (streamable HTTP, no auth).
        </p>
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
        <ConnectGuideBlocks
          blocks={[
            {
              title: "HTTP config",
              steps: ["Paste into clients that accept mcpServers JSON."],
              copyText: ONCHAINAI_MCP_HTTP_JSON,
              copyLabel: "Copy config",
              configJson: ONCHAINAI_MCP_HTTP_JSON,
            },
            {
              title: "Stdio bridge",
              steps: ["For older clients that only support stdio MCP."],
              copyText: ONCHAINAI_MCP_STDIO_JSON,
              copyLabel: "Copy config",
              configJson: ONCHAINAI_MCP_STDIO_JSON,
            },
          ]}
        />
      </section>

      <section aria-labelledby="connect-clients-heading" className="mb-8">
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
                <span className="connect-client-icon">
                  <CodingClientLogo
                    id={logoIdForConnectPageClient(client.id)}
                    label={client.label}
                    size={24}
                    decorative
                  />
                </span>
                <h3 className="connect-client-card-title">{client.label}</h3>
              </div>
              <ConnectGuideBlocks blocks={client.blocks} />
            </article>
          ))}
        </div>
      </section>

      <section
        className="connect-plugin-section mb-8"
        aria-labelledby="connect-plugin-heading"
        data-testid="connect-plugin-card"
      >
        <h2 id="connect-plugin-heading" className="text-h3 font-semibold mb-3">
          Claude Code plugin (optional)
        </h2>
        <p className="text-secondary text-body-md leading-relaxed mb-4 max-w-[720px]">
          Claude Code only. One install ships MCP auto-connect,{" "}
          <code className="text-code">/find-tool</code>, and the{" "}
          <code className="text-code">onchainai-crypto-tools</code> skill — no separate skill
          command. Skip this if you already connected via universal install above.
        </p>
        <div className="connect-guide-block">
          <h3 className="connect-guide-block-title">Plugin install</h3>
          <ol className="install-steps">
            <li>Step 1 — In Claude Code, add the marketplace once.</li>
            <li>Step 2 — Install the plugin (MCP + skill + command ship together).</li>
            <li>Restart Claude Code and run <code className="text-code">/mcp</code> to confirm.</li>
          </ol>
          <p className="text-body-sm text-secondary mt-3 mb-1">Step 1 — Add marketplace (once)</p>
          <div className="tool-install-stack">
            <div className="tool-install">
              <HighlightedCommand
                command={ONCHAINAI_PLUGIN_MARKETPLACE_CMD}
                showPrefix={false}
                showCopy={false}
              />
              <CopyButton
                text={ONCHAINAI_PLUGIN_MARKETPLACE_CMD}
                label={copyLabelAria("Copy marketplace command")}
              />
            </div>
          </div>
          <p className="text-body-sm text-secondary mt-3 mb-1">
            Step 2 — Install plugin (includes skill)
          </p>
          <div className="tool-install-stack">
            <div className="tool-install">
              <HighlightedCommand
                command={ONCHAINAI_PLUGIN_INSTALL_CMD}
                showPrefix={false}
                showCopy={false}
              />
              <CopyButton
                text={ONCHAINAI_PLUGIN_INSTALL_CMD}
                label={copyLabelAria("Copy install command")}
              />
            </div>
          </div>
          <div className="tool-install mt-3">
            <CopyButton
              text={PLUGIN_INSTALL_BOTH}
              label={copyLabelAria("Copy both plugin commands")}
            />
            <span className="text-body-sm text-secondary">Copy both steps</span>
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

      <K2ProbeReceiptSection />

      <AgentLinkSection />
    </div>
  );
}