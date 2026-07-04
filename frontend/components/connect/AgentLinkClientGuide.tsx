"use client";

import { useState } from "react";
import { HighlightedCommand } from "@/components/tools/HighlightedCommand";
import { CopyButton } from "@/components/ui/CopyButton";
import { copyLabelAria } from "@/lib/install-guide";
import { CodingClientLogo } from "@/components/tools/CodingClientLogo";
import {
  AGENT_LINK_CLIENTS,
  buildDevicePollCurl,
  buildDeviceStartCurl,
  DEVICE_START_MOCK,
  type AgentLinkClient,
} from "@/lib/agent-link-guide";
import { logoIdForAgentLinkClient } from "@/lib/coding-clients";

export function AgentLinkClientGuide() {
  const [client, setClient] = useState<AgentLinkClient>("claude-code");
  const config = AGENT_LINK_CLIENTS.find((c) => c.id === client) ?? AGENT_LINK_CLIENTS[0];
  const startCmd = buildDeviceStartCurl(client);
  const pollCmd = buildDevicePollCurl();

  return (
    <div className="agent-link-client-guide" data-testid="agent-link-client-tabs">
      <p className="text-body-sm text-secondary mb-3">
        Pick your app, run one command in the terminal, then enter the short code on this page.
      </p>
      <div
        className="install-platform-group agent-link-tabs"
        role="tablist"
        aria-label="Coding app for Agent Sync"
      >
        {AGENT_LINK_CLIENTS.map((c) => (
          <button
            key={c.id}
            type="button"
            role="tab"
            id={`agent-link-tab-${c.id}`}
            aria-selected={client === c.id}
            aria-controls={`agent-link-panel-${c.id}`}
            className={client === c.id ? "install-platform-btn active" : "install-platform-btn"}
            data-testid={`agent-link-tab-${c.id}`}
            onClick={() => setClient(c.id)}
          >
            {(() => {
              const logoId = logoIdForAgentLinkClient(c.id);
              return (
                <>
                  {logoId && (
                    <CodingClientLogo id={logoId} label={c.label} size={16} decorative />
                  )}
                  <span>{c.label}</span>
                </>
              );
            })()}
          </button>
        ))}
      </div>
      <div
        id={`agent-link-panel-${client}`}
        role="tabpanel"
        aria-labelledby={`agent-link-tab-${client}`}
        className="agent-link-tab-panel mt-4"
      >
        {config.pluginCallout && (
          <p className="agent-link-plugin-callout text-body-sm mb-3">{config.pluginCallout}</p>
        )}
        <ol className="install-steps agent-link-steps">
          {config.steps.map((step) => (
            <li key={step}>{step}</li>
          ))}
        </ol>
        <div className="tool-install-stack mt-3">
          <div className="tool-install">
            <HighlightedCommand command={startCmd} showPrefix={false} showCopy={false} />
            <CopyButton text={startCmd} label={copyLabelAria("Copy start command")} />
          </div>
        </div>
        <p className="text-body-sm text-secondary mt-3" id="agent-link-code-hint">
          Look for <code className="text-code">user_code</code> in the response — an 8-character
          code like <strong>K7M3-9P2X</strong>. Uses A–Z and 2–9 only; no 0, O, 1, or I.
        </p>
        <div
          className="agent-link-code-sample"
          aria-hidden
          data-testid="agent-link-code-example"
        >
          K7M3-9P2X
        </div>
        <details className="agent-link-response-example mt-3">
          <summary className="text-body-sm font-medium cursor-pointer text-primary">
            Example response
          </summary>
          <pre
            className="agent-link-terminal-mock mt-2 p-3 rounded-sm bg-neutral-surface text-code"
            aria-label="Example terminal output"
          >
            {DEVICE_START_MOCK}
          </pre>
        </details>
        <div className="tool-install-stack mt-4">
          <p className="text-body-sm font-medium mb-1">After Connect, poll in your terminal:</p>
          <div className="tool-install">
            <HighlightedCommand command={pollCmd} showPrefix={false} showCopy={false} />
            <CopyButton text={pollCmd} label={copyLabelAria("Copy poll command")} />
          </div>
        </div>
        {config.footerNote && (
          <p className="text-body-sm text-secondary mt-2">{config.footerNote}</p>
        )}
      </div>
    </div>
  );
}