# Connect OnchainAI to Your Agent

> Canonical connection guide. Live interactive version: [onchain-ai.xyz/connect](https://www.onchain-ai.xyz/connect).

OnchainAI exposes one read-only, no-auth MCP endpoint:

```
https://www.onchain-ai.xyz/mcp
```

- Transport: **streamable HTTP** (JSON-RPC 2.0 over `POST /mcp`; `GET` returns `405 Allow: POST` by design)
- Auth: none. Rate limited per IP.
- Free tools: `search_tools`, `get_tool_detail`, `get_install_guide`, `list_categories`, `get_dashboard_snapshot`, `compare_tools`, `export_toolkit` · Paid: `check_endpoint_health` (x402 per call) · Linked account (Agent Sync): `save_to_toolkit`, `save_stack_to_blueprint`, `link_status`
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

## Agent Sync — save tools to your web toolkit

Link your OnchainAI account so coding tools can **explicitly** save tools to
`/toolkit` and (optionally) append nodes to today's agent-session blueprint.

**Canonical UI:** [onchain-ai.xyz/connect#agent-sync](https://www.onchain-ai.xyz/connect#agent-sync)

### Recommended: device flow (no manual token copy)

1. Sign in on the website → open **Connect** → **Link your agent**.
2. In Claude Code / Cursor, start the device link (or use the guided steps on Connect).
3. Enter the short code shown in your coding app on the website — the token is
   delivered to the agent automatically.

### Manual token (advanced)

- Mint a token once on Connect (prefix shown later; plaintext shown **once**).
- Set env `ONCHAINAI_AGENT_TOKEN=oai_ag_…` or add an HTTP header on the MCP client:

```json
{
  "mcpServers": {
    "onchainai": {
      "type": "http",
      "url": "https://www.onchain-ai.xyz/mcp",
      "headers": {
        "Authorization": "Bearer ${ONCHAINAI_AGENT_TOKEN:-}"
      }
    }
  }
}
```

Plugin **0.2.0+** ships this header pattern in `plugin/onchainai/.mcp.json`.

### Authenticated MCP tools

With a valid Bearer token, `tools/list` also exposes:

- `save_to_toolkit` — save one slug to My Toolkit (`source=agent`)
- `save_stack_to_blueprint` — save slugs to toolkit + today's `Agent session · {date}` blueprint
- `link_status` — check whether the client is linked

Without a token, `save_to_toolkit` returns `link_required` with
`link_url` pointing to `/connect#agent-sync`. Read-only tools stay public.

### Transport note

**Prefer streamable HTTP** for Agent Sync (headers reach the server). Stdio
`mcp-remote` bridges may not forward `Authorization` — use HTTP transport or REST
`POST /api/v2/agent/sync/tool` with Bearer when bridging is required.

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

## Listed on (external discovery)

Submit status tracked in `docs/X402_ROADMAP.md` §10. Copy-paste payloads: `docs/listings/directory-forms.md`.

| Channel | URL / artifact | Status |
|---------|----------------|--------|
| Official MCP Registry | [io.github.Coinyak/onchainai](https://registry.modelcontextprotocol.io) v0.2.0 | Published 2026-07-04 |
| web3-mcp-hub | [rudazy/web3-mcp-hub#1](https://github.com/rudazy/web3-mcp-hub/pull/1) | Open |
| awesome-crypto-mcp-servers | [hive-intel/awesome-crypto-mcp-servers#209](https://github.com/hive-intel/awesome-crypto-mcp-servers/pull/209) | Open |
| Self catalog | [onchain-ai.xyz/tools/onchainai](https://www.onchain-ai.xyz/tools/onchainai) | Seeded (official) |
| Smithery / mcp.so / PulseMCP / Glama | See `docs/listings/directory-forms.md` | Operator submit |
| x402 Bazaar (seller) | CDP Facilitator settle — no registration form | After `check_endpoint_health` prod settle |
| Base Builder Code | [dashboard.base.org](https://dashboard.base.org) | Operator register app + domain |

## Troubleshooting

| Symptom | Explanation |
|---|---|
| `405 Method Not Allowed` on GET | Expected — the endpoint is POST-only JSON-RPC. Point an MCP client at it, not a browser. |
| `429 Too Many Requests` | Per-IP rate limit. Back off and retry. |
| Tool not found by slug | Slugs come from `search_tools` results — don't guess them. |
| Client only supports stdio | Use the `mcp-remote` bridge above. |
| `Connection closed` on `check_endpoint_health` | Expected on Claude Code/Cursor — HTTP 402 is not an MCP JSON-RPC result. Use free `get_tool_detail` (x402 flags) or REST `GET /api/v2/premium/check-endpoint-health/{slug}` with an x402 wallet client. |
