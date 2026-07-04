import { clientApiBase } from "@/lib/api";

export type AgentLinkClient = "claude-code" | "cursor" | "generic";

export interface AgentLinkClientConfig {
  id: AgentLinkClient;
  label: string;
  steps: string[];
  footerNote?: string;
  pluginCallout?: string;
}

function apiBaseForCurl(): string {
  const base = clientApiBase();
  return base || "";
}

export function buildDeviceStartCurl(client: AgentLinkClient): string {
  const base = apiBaseForCurl();
  const url = base ? `${base}/api/v2/agent/device/start` : "/api/v2/agent/device/start";
  return `curl -sS -X POST "${url}" \\
  -H "Content-Type: application/json" \\
  -d '{"client":"${client}"}'`;
}

export function buildDevicePollCurl(): string {
  const base = apiBaseForCurl();
  const url = base ? `${base}/api/v2/agent/device/poll` : "/api/v2/agent/device/poll";
  return `curl -sS -X POST "${url}" \\
  -H "Content-Type: application/json" \\
  -d '{"device_code":"PASTE_DEVICE_CODE_HERE"}'`;
}

export const DEVICE_START_MOCK = `{
  "device_code": "xK9mP2vL8nQ4rT6wY1zA3bC5dE7fG0h",
  "user_code": "K7M3-9P2X",
  "verification_uri": "https://www.onchain-ai.xyz/connect#agent-sync",
  "expires_in": 900,
  "interval": 5
}`;

export const AGENT_LINK_CLIENTS: AgentLinkClientConfig[] = [
  {
    id: "claude-code",
    label: "Claude Code",
    pluginCallout:
      "Using the OnchainAI plugin? After install, your agent may print a code when you save a tool — skip curl and use I already have a code below.",
    steps: [
      "Open a terminal in your project folder.",
      "Run the start command below.",
      "Copy the user_code from the output (like K7M3-9P2X).",
      "Enter it in I already have a code on this page and click Connect.",
      "Run the poll command until status is approved, then set ONCHAINAI_AGENT_TOKEN.",
    ],
    footerNote: "Codes expire in 15 minutes. Run start again if needed.",
  },
  {
    id: "cursor",
    label: "Cursor",
    steps: [
      "Open the terminal in Cursor (View → Terminal).",
      "Run the start command below.",
      "Copy the user_code from the output.",
      "Enter it below and click Connect.",
      "Poll until approved, then add Authorization: Bearer ${ONCHAINAI_AGENT_TOKEN} to .cursor/mcp.json.",
    ],
    footerNote: "Codes expire in 15 minutes.",
  },
  {
    id: "generic",
    label: "Other",
    steps: [
      "Open a terminal where your MCP client runs.",
      "Run the start command below (client: generic).",
      "Copy the user_code from the output.",
      "Enter it below and click Connect.",
      "Poll until approved; set ONCHAINAI_AGENT_TOKEN in your client env.",
    ],
    footerNote: "See MCP clients section below for more editors.",
  },
];