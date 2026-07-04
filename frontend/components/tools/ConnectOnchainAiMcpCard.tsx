import Link from "next/link";
import { HighlightedCommand } from "@/components/tools/HighlightedCommand";
import { CopyButton } from "@/components/ui/CopyButton";
import { ONCHAINAI_MCP_UNIVERSAL_CMD } from "@/lib/mcp-connect";
import { copyLabelAria } from "@/lib/install-guide";

interface ConnectOnchainAiMcpCardProps {
  /** Kept for PromoCards API compatibility; Phase 9 uses canonical HTTP endpoint. */
  mcpEndpoint?: string;
}

export function ConnectOnchainAiMcpCard(_props: ConnectOnchainAiMcpCardProps) {
  return (
    <div
      className="promo-card promo-card--connect border border-border-strong rounded-lg p-6 bg-white min-w-0"
      data-testid="connect-onchainai-mcp-card"
    >
      <h3 className="text-[16px] font-semibold mb-2">Connect OnchainAI MCP</h3>
      <p className="text-[14px] text-[#6B6B6B] mb-3 leading-relaxed">
        One free endpoint — let your agent search crypto tools from Claude, Cursor, Codex, VS Code,
        and more.
      </p>
      <div className="tool-install-stack">
        <div className="tool-install">
          <HighlightedCommand
            command={ONCHAINAI_MCP_UNIVERSAL_CMD}
            showPrefix
            showCopy={false}
          />
          <CopyButton
            text={ONCHAINAI_MCP_UNIVERSAL_CMD}
            label={copyLabelAria("Copy command")}
          />
        </div>
      </div>
      <Link
        href="/connect"
        className="inline-flex items-center justify-center h-9 px-3 mt-3 rounded-lg border border-border bg-neutral-bg text-primary text-[13px] font-medium no-underline hover:bg-neutral-surface"
        data-testid="connect-more-clients-link"
      >
        HTTP config &amp; all clients →
      </Link>
    </div>
  );
}