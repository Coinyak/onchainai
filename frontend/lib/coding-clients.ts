import type { AgentLinkClient } from "@/lib/agent-link-guide";
import type { ConnectPageClientId } from "@/lib/mcp-connect";
import type { ToolInstallClient } from "@/lib/mcp-connect-clients";

/** Logo asset ids under /clients/*.svg */
export type CodingClientLogoId =
  | "cursor"
  | "vscode"
  | "claude"
  | "openai"
  | "gemini"
  | "windsurf"
  | "generic";

const LOGO_FILES: Record<CodingClientLogoId, string> = {
  cursor: "/clients/cursor.svg",
  vscode: "/clients/vscode.svg",
  claude: "/clients/claude.svg",
  openai: "/clients/openai.svg",
  gemini: "/clients/gemini.svg",
  windsurf: "/clients/windsurf.svg",
  generic: "/clients/generic.svg",
};

export function codingClientLogoPath(id: CodingClientLogoId): string {
  return LOGO_FILES[id];
}

export function hasCodingClientLogo(id: string): id is CodingClientLogoId {
  return id in LOGO_FILES;
}

export function logoIdForConnectPageClient(id: ConnectPageClientId): CodingClientLogoId {
  switch (id) {
    case "cursor":
      return "cursor";
    case "vscode":
      return "vscode";
    case "claude_desktop":
    case "claude_code":
      return "claude";
    case "chatgpt":
    case "codex":
      return "openai";
    case "windsurf":
      return "windsurf";
    case "gemini":
      return "gemini";
    case "generic":
      return "generic";
  }
}

export function logoIdForToolInstallClient(client: ToolInstallClient): CodingClientLogoId | null {
  switch (client) {
    case "cursor":
      return "cursor";
    case "vscode":
      return "vscode";
    case "claude":
      return "claude";
    case "chatgpt":
    case "codex":
      return "openai";
    case "generic":
      return "generic";
    case "more":
      return null;
  }
}

export function logoIdForAgentLinkClient(id: AgentLinkClient): CodingClientLogoId | null {
  switch (id) {
    case "claude-code":
      return "claude";
    case "cursor":
      return "cursor";
    case "generic":
      return "generic";
  }
}

export function logoIdForDeeplinkLabel(label: string): CodingClientLogoId | null {
  if (label.includes("Cursor")) return "cursor";
  if (label.includes("VS Code")) return "vscode";
  return null;
}