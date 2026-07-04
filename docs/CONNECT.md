# Connect OnchainAI to Your Agent

> Canonical connection guide. Live interactive version: [onchain-ai.xyz/connect](https://www.onchain-ai.xyz/connect).

OnchainAI exposes one free, read-only, no-auth MCP endpoint:

```
https://www.onchain-ai.xyz/mcp
```

- Transport: **streamable HTTP** (JSON-RPC 2.0 over `POST /mcp`; `GET` returns `405 Allow: POST` by design)
- Auth: none. Rate limited per IP.
- Tools: `search_tools`, `get_tool_detail`, `get_install_guide`, `list_categories`, `get_dashboard_snapshot`
- This is the **only** official endpoint. Anything else claiming to be OnchainAI is not ours.

## Claude Code (CLI)

```bash
claude mcp add --transport http onchainai https://www.onchain-ai.xyz/mcp
```

Restart Claude Code and check with `/mcp`.

**Better: install the plugin** — MCP server + `/find-tool` command + crypto-tools skill in one step:

```
/plugin marketplace add Coinyak/onchainai
/plugin install onchainai@onchainai
```

## Claude Desktop / Claude Web

Settings → **Connectors** → **Add custom connector** → name it `OnchainAI`, URL
`https://www.onchain-ai.xyz/mcp`, save, enable it in your chat.

## Cursor

Use the one-click **Add to Cursor** deeplink on [/connect](https://www.onchain-ai.xyz/connect),
or add to `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "onchainai": { "url": "https://www.onchain-ai.xyz/mcp" }
  }
}
```

## VS Code (GitHub Copilot MCP)

**Add to VS Code** deeplink on [/connect](https://www.onchain-ai.xyz/connect), or
`MCP: Add Server` → HTTP → paste the endpoint URL.

## ChatGPT

Settings → Connectors → enable Developer mode (Advanced settings) → create a
connector named `OnchainAI` with MCP server URL `https://www.onchain-ai.xyz/mcp`.

## Codex / Windsurf / Gemini CLI / other HTTP-capable clients

Add an MCP server of type `http` (sometimes called `streamable-http` or just
`url`) pointing at the endpoint. Generic JSON shape:

```json
{
  "mcpServers": {
    "onchainai": { "type": "http", "url": "https://www.onchain-ai.xyz/mcp" }
  }
}
```

## Stdio-only clients (older Claude Desktop, misc.)

Bridge with [`mcp-remote`](https://www.npmjs.com/package/mcp-remote):

```json
{
  "mcpServers": {
    "onchainai": {
      "command": "npx",
      "args": ["mcp-remote", "https://www.onchain-ai.xyz/mcp"]
    }
  }
}
```

Or auto-detect your client with `npx add-mcp https://www.onchain-ai.xyz/mcp`.

## Using the skill without the plugin

Copy [`plugin/onchainai/skills/onchainai-crypto-tools/`](../plugin/onchainai/skills/onchainai-crypto-tools/)
into your skills directory (Claude Code: `~/.claude/skills/`), or upload the
folder to any runtime that supports Agent Skills. The skill assumes the
`onchainai` MCP server is connected.

## What to ask once connected

- "Find me an MCP server to swap on Base — safest option first."
- "Compare the top Solana wallet SDKs in the OnchainAI directory."
- "What does it cost to call <tool>? Is its x402 pricing verified?"
- "Give me the Cursor install config for <tool slug>."
- With the plugin: `/find-tool bridge USDC from Ethereum to Base`

## Safety semantics your agent should respect

- `install_risk_level = "critical"` → the guide is **blocked**; the raw command
  is withheld server-side. Do not attempt installation.
- `install_risk_level = "high"` → warn the user before showing install steps.
- x402/paid tools → `get_install_guide` includes an `x402_notice` and referral
  metadata. Disclose price and wallet requirement *before* the user calls the
  tool. `payment_verified`/`x402_endpoint_verified`/`price_verified` all true =
  "operator verified"; otherwise say "not yet verified".
- OnchainAI never processes payments — metadata only.

## Troubleshooting

| Symptom | Explanation |
|---|---|
| `405 Method Not Allowed` on GET | Expected — the endpoint is POST-only JSON-RPC. Point an MCP client at it, not a browser. |
| `429 Too Many Requests` | Per-IP rate limit. Back off and retry. |
| Tool not found by slug | Slugs come from `search_tools` results — don't guess them. |
| Client only supports stdio | Use the `mcp-remote` bridge above. |
