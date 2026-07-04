/** Home card + tool detail tab clients — general/universal path first (MCP_ADD_FLOW_SPEC). */
export type ConnectCardClient =
  | "generic"
  | "claude"
  | "cursor"
  | "vscode"
  | "chatgpt"
  | "codex";

export type ToolInstallClient = ConnectCardClient | "more";

export const CONNECT_CARD_CLIENTS: ConnectCardClient[] = [
  "generic",
  "claude",
  "cursor",
  "vscode",
  "chatgpt",
  "codex",
];

export const DEFAULT_CONNECT_CLIENT: ConnectCardClient = "generic";

export const TOOL_INSTALL_CLIENTS: ToolInstallClient[] = [
  ...CONNECT_CARD_CLIENTS,
  "more",
];

export function connectClientLabel(client: ConnectCardClient): string {
  switch (client) {
    case "generic":
      return "Universal";
    case "codex":
      return "Codex CLI";
    case "chatgpt":
      return "ChatGPT connector";
    case "claude":
      return "Claude";
    case "cursor":
      return "Cursor";
    case "vscode":
      return "VS Code";
  }
}

export function toolInstallClientLabel(client: ToolInstallClient): string {
  if (client === "more") return "More";
  return connectClientLabel(client);
}