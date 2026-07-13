/** Public install guide assembly. */
import type { PublicTool } from "@/lib/api";
import type { ToolInstallClient } from "@/lib/mcp-connect-clients";
export {
  TOOL_INSTALL_CLIENTS,
  toolInstallClientLabel,
  type ToolInstallClient,
} from "@/lib/mcp-connect-clients";
import {
  type InstallPlatform,
  type PublicInstallGuide,
  copyGateForRisk,
  installWarningText,
} from "./install-guide-shared";
import {
  buildToolClientBlocks,
  isMcpCatalogTool,
} from "./install-guide-client-blocks";
import {
  primaryInstallCommand,
  toolGuideMeta,
} from "./install-guide-commands";

export {
  primaryInstallCommand,
  toolHasInstallPath,
  addMcpActionLabel,
  addMcpHref,
  addMcpHrefFromCompare,
  toolGuideMeta,
  claudeMcpConfig,
} from "./install-guide-commands";

export function buildPublicInstallGuide(
  tool: PublicTool,
  slug: string,
  client: ToolInstallClient,
): PublicInstallGuide {
  if (tool.install_risk_level === "critical") {
    return {
      slug,
      tool_name: tool.name,
      platform: client,
      risk_level: tool.install_risk_level,
      risk_reasons: tool.install_risk_reasons,
      warning: "Install guidance blocked: critical risk pending operator review.",
      blocked: true,
      copy_gate: "blocked",
      command: null,
      config_json: null,
      copy_text: null,
      copy_label: "Copy blocked",
      steps: [
        "This tool has a critical-risk install command.",
        "Public install guidance is withheld until an operator reviews the listing.",
      ],
      connect_blocks: [],
      ...toolGuideMeta(tool),
      docs_links: [],
      x402_notice: null,
      referral_disclosure: null,
    };
  }

  const command = primaryInstallCommand(tool);
  if (tool.type === "skill" && !isMcpCatalogTool(tool)) {
    const steps = command
      ? [
          "Install the skill using the command below (e.g. clawhub or your agent skills runtime).",
          "Do not paste this into MCP server settings — skills are not MCP configs.",
          "Open the docs link for usage after install.",
        ]
      : [
          "No install command is listed for this tool.",
          "Use the repository or docs links below for setup.",
        ];
    return {
      slug,
      tool_name: tool.name,
      platform: client,
      risk_level: tool.install_risk_level,
      risk_reasons: tool.install_risk_reasons,
      warning: installWarningText(tool.install_risk_level),
      blocked: false,
      copy_gate: copyGateForRisk(tool.install_risk_level),
      command,
      config_json: null,
      copy_text: command,
      copy_label: "Copy command",
      steps,
      connect_blocks: command
        ? [
            {
              steps: ["Run the install command, then open the docs for usage."],
              copyText: command,
              copyLabel: "Copy command",
              showShellPrefix: true,
            },
          ]
        : [],
      ...toolGuideMeta(tool),
    };
  }

  if (
    (tool.type === "cli" || tool.type === "sdk" || tool.type === "api") &&
    !isMcpCatalogTool(tool)
  ) {
    const steps = command
      ? [
          "Run the install command in your terminal or package manager.",
          "Open the repository or docs link for setup and API keys.",
        ]
      : [
          "No install command is listed for this tool.",
          "Use the repository or docs links below for setup.",
        ];
    return {
      slug,
      tool_name: tool.name,
      platform: client,
      risk_level: tool.install_risk_level,
      risk_reasons: tool.install_risk_reasons,
      warning: installWarningText(tool.install_risk_level),
      blocked: false,
      copy_gate: copyGateForRisk(tool.install_risk_level),
      command,
      config_json: null,
      copy_text: command,
      copy_label: "Copy command",
      steps,
      connect_blocks: command
        ? [
            {
              steps: ["Copy and run the install command below."],
              copyText: command,
              copyLabel: "Copy command",
              showShellPrefix: true,
            },
          ]
        : [],
      ...toolGuideMeta(tool),
    };
  }

  const blocks = buildToolClientBlocks(tool, slug, client);
  const primary = blocks.find((b) => b.copyText) ?? blocks[0];
  const copyGate = copyGateForRisk(tool.install_risk_level);

  return {
    slug,
    tool_name: tool.name,
    platform: client,
    risk_level: tool.install_risk_level,
    risk_reasons: tool.install_risk_reasons,
    warning: installWarningText(tool.install_risk_level),
    blocked: false,
    copy_gate: copyGate,
    command: primaryInstallCommand(tool),
    config_json: primary.configJson ?? null,
    copy_text: primary.copyText,
    copy_label: primary.copyLabel,
    steps: primary.steps,
    connect_blocks: blocks,
    ...toolGuideMeta(tool),
  };
}