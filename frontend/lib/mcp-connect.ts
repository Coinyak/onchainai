import type { ConnectGuideBlock } from "@/lib/install-guide";
import type { ConnectCardClient } from "@/lib/mcp-connect-clients";
import {
  ONCHAINAI_CLAUDE_CODE_CMD,
  ONCHAINAI_MCP_HTTP_URL,
  ONCHAINAI_MCP_SERVER_NAME,
  ONCHAINAI_MCP_UNIVERSAL_CMD,
  onchainaiCursorDeeplink,
  onchainaiVscodeDeeplink,
} from "@/lib/mcp-deeplinks";

export {
  CONNECT_CARD_CLIENTS,
  DEFAULT_CONNECT_CLIENT,
  connectClientLabel,
  type ConnectCardClient,
} from "@/lib/mcp-connect-clients";

export {
  ONCHAINAI_MCP_HTTP_URL,
  ONCHAINAI_MCP_SERVER_NAME,
  ONCHAINAI_MCP_UNIVERSAL_CMD,
  ONCHAINAI_CLAUDE_CODE_CMD,
  onchainaiCursorDeeplink,
  onchainaiVscodeDeeplink,
} from "@/lib/mcp-deeplinks";

export interface OnchainaiConnectGuide {
  client: ConnectCardClient;
  blocks: ConnectGuideBlock[];
}

function httpMcpJsonConfig(serverName: string, url: string): string {
  return JSON.stringify(
    {
      mcpServers: {
        [serverName]: { url },
      },
    },
    null,
    2,
  );
}

export function buildOnchainaiConnectGuide(
  client: ConnectCardClient,
): OnchainaiConnectGuide {
  const jsonSnippet = httpMcpJsonConfig(
    ONCHAINAI_MCP_SERVER_NAME,
    ONCHAINAI_MCP_HTTP_URL,
  );

  switch (client) {
    case "chatgpt":
      return {
        client,
        blocks: [
          {
            steps: [
              "Enable Developer mode in ChatGPT (Settings → Connectors → Advanced settings).",
              "Open Settings → Connectors and create a new connector.",
              "Set Name to OnchainAI and MCP server URL to the endpoint below.",
              "Choose OAuth if prompted, then use Developer mode in chat to invoke the connector.",
            ],
            copyText: ONCHAINAI_MCP_HTTP_URL,
            copyLabel: "Copy endpoint URL",
          },
        ],
      };
    case "claude":
      return {
        client,
        blocks: [
          {
            title: "Claude Desktop or Web",
            steps: [
              "Open Settings → Connectors → Add custom connector.",
              "Set Name to OnchainAI and URL to the MCP endpoint below.",
              "Save and enable the connector in your Claude session.",
            ],
            copyText: ONCHAINAI_MCP_HTTP_URL,
            copyLabel: "Copy endpoint URL",
          },
          {
            title: "Claude Code CLI",
            steps: [
              "Run the command below in your project terminal.",
              "Restart Claude Code and verify the server with /mcp.",
            ],
            copyText: ONCHAINAI_CLAUDE_CODE_CMD,
            copyLabel: "Copy command",
            showShellPrefix: true,
          },
        ],
      };
    case "cursor":
      return {
        client,
        blocks: [
          {
            steps: [
              "Click Add to Cursor for a one-click install, or paste the JSON into .cursor/mcp.json.",
              "Reload MCP servers in Cursor after saving.",
            ],
            copyText: jsonSnippet,
            copyLabel: "Copy config",
            configJson: jsonSnippet,
            deeplinkHref: onchainaiCursorDeeplink(),
            deeplinkLabel: "Add to Cursor",
          },
        ],
      };
    case "vscode":
      return {
        client,
        blocks: [
          {
            steps: [
              "Click Add to VS Code for a one-click install, or use MCP: Add Server manually.",
              "Select HTTP transport, paste the endpoint URL, and name the server OnchainAI.",
              "Start the server from MCP: List Servers and authorize if prompted.",
            ],
            copyText: ONCHAINAI_MCP_HTTP_URL,
            copyLabel: "Copy endpoint URL",
            deeplinkHref: onchainaiVscodeDeeplink(),
            deeplinkLabel: "Add to VS Code",
          },
        ],
      };
  }
}

export type ConnectPageClientId =
  | "claude_code"
  | "claude_desktop"
  | "cursor"
  | "vscode"
  | "codex"
  | "windsurf"
  | "gemini"
  | "chatgpt"
  | "generic";

export interface ConnectPageClient {
  id: ConnectPageClientId;
  label: string;
  icon: string;
  blocks: ConnectGuideBlock[];
}

export const CONNECT_PAGE_CLIENTS: ConnectPageClient[] = [
  {
    id: "claude_code",
    label: "Claude Code",
    icon: "terminal",
    blocks: [
      {
        steps: [
          "Install Claude Code if needed: npm install -g @anthropic-ai/claude-code",
          "Run the command below from your project directory.",
          "Type /mcp in Claude Code to authenticate if required.",
        ],
        copyText: ONCHAINAI_CLAUDE_CODE_CMD,
        copyLabel: "Copy command",
        showShellPrefix: true,
      },
    ],
  },
  {
    id: "claude_desktop",
    label: "Claude Desktop and Web",
    icon: "message-square",
    blocks: [
      {
        steps: [
          "Open Settings → Connectors → Add custom connector.",
          "Name: OnchainAI. URL: the MCP endpoint below.",
          "Enable the connector in Claude Desktop or claude.ai.",
        ],
        copyText: ONCHAINAI_MCP_HTTP_URL,
        copyLabel: "Copy endpoint URL",
      },
    ],
  },
  {
    id: "cursor",
    label: "Cursor",
    icon: "mouse-pointer-click",
    blocks: [
      {
        steps: [
          "Use Add to Cursor for one-click install, or add the JSON to .cursor/mcp.json.",
          "Reload MCP after saving the config.",
        ],
        copyText: httpMcpJsonConfig(ONCHAINAI_MCP_SERVER_NAME, ONCHAINAI_MCP_HTTP_URL),
        copyLabel: "Copy config",
        configJson: httpMcpJsonConfig(ONCHAINAI_MCP_SERVER_NAME, ONCHAINAI_MCP_HTTP_URL),
        deeplinkHref: onchainaiCursorDeeplink(),
        deeplinkLabel: "Add to Cursor",
      },
    ],
  },
  {
    id: "vscode",
    label: "VS Code Copilot",
    icon: "code",
    blocks: [
      {
        steps: [
          "Click Add to VS Code or run MCP: Add Server → HTTP.",
          "URL: endpoint below. Name: OnchainAI.",
          "Start the server from MCP: List Servers.",
        ],
        copyText: ONCHAINAI_MCP_HTTP_URL,
        copyLabel: "Copy endpoint URL",
        deeplinkHref: onchainaiVscodeDeeplink(),
        deeplinkLabel: "Add to VS Code",
      },
    ],
  },
  {
    id: "codex",
    label: "Codex CLI",
    icon: "terminal",
    blocks: [
      {
        steps: [
          "Install Codex CLI: npm i -g @openai/codex",
          "Run the command below to register OnchainAI MCP.",
          "Complete OAuth in the browser when Codex prompts you.",
        ],
        copyText: `codex mcp add ${ONCHAINAI_MCP_SERVER_NAME} --url ${ONCHAINAI_MCP_HTTP_URL}`,
        copyLabel: "Copy command",
        showShellPrefix: true,
      },
    ],
  },
  {
    id: "windsurf",
    label: "Windsurf",
    icon: "wind",
    blocks: [
      {
        steps: [
          "Add the JSON snippet to your Windsurf mcp_config.json.",
          "Restart Windsurf Cascade to load the server.",
        ],
        copyText: JSON.stringify(
          {
            mcpServers: {
              [ONCHAINAI_MCP_SERVER_NAME]: { serverUrl: ONCHAINAI_MCP_HTTP_URL },
            },
          },
          null,
          2,
        ),
        copyLabel: "Copy config",
        configJson: JSON.stringify(
          {
            mcpServers: {
              [ONCHAINAI_MCP_SERVER_NAME]: { serverUrl: ONCHAINAI_MCP_HTTP_URL },
            },
          },
          null,
          2,
        ),
      },
    ],
  },
  {
    id: "gemini",
    label: "Gemini CLI and Code Assist",
    icon: "sparkles",
    blocks: [
      {
        steps: [
          "Add the JSON to ~/.gemini/settings.json under mcpServers.",
          "Restart your IDE or Gemini CLI and authenticate when prompted.",
        ],
        copyText: JSON.stringify(
          {
            mcpServers: {
              [ONCHAINAI_MCP_SERVER_NAME]: {
                command: "npx",
                args: ["mcp-remote", ONCHAINAI_MCP_HTTP_URL],
              },
            },
          },
          null,
          2,
        ),
        copyLabel: "Copy config",
        configJson: JSON.stringify(
          {
            mcpServers: {
              [ONCHAINAI_MCP_SERVER_NAME]: {
                command: "npx",
                args: ["mcp-remote", ONCHAINAI_MCP_HTTP_URL],
              },
            },
          },
          null,
          2,
        ),
      },
    ],
  },
  {
    id: "chatgpt",
    label: "ChatGPT",
    icon: "bot",
    blocks: [
      {
        steps: [
          "Enable Developer mode (Settings → Connectors → Advanced settings).",
          "Create a connector with Name OnchainAI and the MCP URL below.",
          "Invoke the connector from Developer mode in chat.",
        ],
        copyText: ONCHAINAI_MCP_HTTP_URL,
        copyLabel: "Copy endpoint URL",
      },
    ],
  },
  {
    id: "generic",
    label: "Generic MCP",
    icon: "plug",
    blocks: [
      {
        steps: [
          "Use the universal installer for any detected agent, or paste the JSON into your client.",
          "Official endpoint URL is listed below.",
        ],
        copyText: ONCHAINAI_MCP_UNIVERSAL_CMD,
        copyLabel: "Copy command",
        showShellPrefix: true,
      },
      {
        title: "HTTP config",
        steps: ["Paste into clients that accept mcpServers JSON."],
        copyText: httpMcpJsonConfig(ONCHAINAI_MCP_SERVER_NAME, ONCHAINAI_MCP_HTTP_URL),
        copyLabel: "Copy config",
        configJson: httpMcpJsonConfig(ONCHAINAI_MCP_SERVER_NAME, ONCHAINAI_MCP_HTTP_URL),
      },
    ],
  },
];