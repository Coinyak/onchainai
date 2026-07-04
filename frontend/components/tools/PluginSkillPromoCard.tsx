"use client";

import Link from "next/link";
import { HighlightedCommand } from "@/components/tools/HighlightedCommand";
import { CopyButton } from "@/components/ui/CopyButton";
import {
  ONCHAINAI_PLUGIN_INSTALL_CMD,
  ONCHAINAI_PLUGIN_MARKETPLACE_CMD,
} from "@/lib/mcp-deeplinks";
import { copyLabelAria } from "@/lib/install-guide";

export function PluginSkillPromoCard() {
  return (
    <div
      className="promo-card border border-[#E5E5E5] rounded-lg p-6 bg-white min-w-0"
      data-testid="plugin-skill-promo-card"
    >
      <h3 className="text-[16px] font-semibold mb-2">Claude Code Plugin &amp; Skill</h3>
      <p className="text-[14px] text-[#6B6B6B] mb-4 leading-relaxed">
        Install the official plugin for MCP auto-connect,{" "}
        <code className="text-code">/find-tool</code>, and the{" "}
        <code className="text-code">onchainai-crypto-tools</code> skill.
      </p>
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
      <Link
        href="/connect#connect-plugin-heading"
        className="inline-flex items-center justify-center h-10 px-4 mt-4 rounded-lg border border-border-strong bg-neutral-bg text-primary text-[14px] font-medium no-underline hover:bg-neutral-surface"
      >
        Full setup →
      </Link>
    </div>
  );
}