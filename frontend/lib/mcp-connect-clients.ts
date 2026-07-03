/** Phase 9.2 home card + tool detail tab clients (vendor-neutral order). */
export type ConnectCardClient = "chatgpt" | "claude" | "cursor" | "vscode";

export type ToolInstallClient = ConnectCardClient | "more";

export const CONNECT_CARD_CLIENTS: ConnectCardClient[] = [
  "chatgpt",
  "claude",
  "cursor",
  "vscode",
];

export const DEFAULT_CONNECT_CLIENT: ConnectCardClient = "chatgpt";

export const TOOL_INSTALL_CLIENTS: ToolInstallClient[] = [
  ...CONNECT_CARD_CLIENTS,
  "more",
];

export function connectClientLabel(client: ConnectCardClient): string {
  switch (client) {
    case "chatgpt":
      return "ChatGPT";
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