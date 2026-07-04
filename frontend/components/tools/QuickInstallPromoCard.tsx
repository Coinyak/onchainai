import Link from "next/link";
import { CodingClientLogo } from "@/components/tools/CodingClientLogo";
import { onchainaiCursorDeeplink, onchainaiVscodeDeeplink } from "@/lib/mcp-deeplinks";

export function QuickInstallPromoCard() {
  return (
    <div
      className="promo-card promo-card--quick-install border border-border-strong rounded-lg p-6 bg-white min-w-0"
      data-testid="quick-install-promo-card"
    >
      <h3 className="text-[16px] font-semibold mb-2">One-click install</h3>
      <p className="text-[14px] text-[#6B6B6B] mb-4 leading-relaxed">
        Cursor and VS Code open your MCP client with OnchainAI pre-filled.
      </p>
      <div className="promo-deeplink-row" role="group" aria-label="One-click MCP install">
        <a
          href={onchainaiCursorDeeplink()}
          className="connect-deeplink-btn"
          data-testid="promo-deeplink-cursor"
        >
          <CodingClientLogo id="cursor" label="Cursor" size={18} decorative />
          <span>Add to Cursor</span>
        </a>
        <a
          href={onchainaiVscodeDeeplink()}
          className="connect-deeplink-btn"
          data-testid="promo-deeplink-vscode"
        >
          <CodingClientLogo id="vscode" label="VS Code" size={18} decorative />
          <span>Add to VS Code</span>
        </a>
      </div>
      <div className="promo-client-links">
        <Link href="/connect#connect-clients-heading" className="promo-client-link-item">
          <CodingClientLogo id="claude" label="Claude" size={16} decorative />
          <span>Claude</span>
        </Link>
        <Link href="/connect#connect-clients-heading" className="promo-client-link-item">
          <CodingClientLogo id="openai" label="Codex CLI" size={16} decorative />
          <span>Codex CLI</span>
        </Link>
        <Link href="/connect" className="promo-client-link-item">
          <CodingClientLogo id="generic" label="All clients" size={16} decorative />
          <span>All clients →</span>
        </Link>
      </div>
      <p className="promo-optional-note text-[13px] text-[#6B6B6B] leading-relaxed">
        <strong className="text-primary font-medium">Plugin</strong> is Claude Code only (
        <Link href="/connect#connect-plugin-heading" className="text-primary hover:underline">
          optional
        </Link>
        ). Codex has no plugin — use{" "}
        <code className="text-code">codex mcp add</code> on /connect.
      </p>
    </div>
  );
}