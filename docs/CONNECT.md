# Connect OnchainAI to Your Agent

> Canonical connection guide. Live interactive version: [onchain-ai.xyz/connect](https://www.onchain-ai.xyz/connect).

OnchainAI exposes one read-only, no-auth MCP endpoint:

```
https://www.onchain-ai.xyz/mcp
```

- Transport: **streamable HTTP** (JSON-RPC 2.0 over `POST /mcp`; `GET /mcp` returns discovery JSON 200)
- Auth: none. Rate limited per IP.
- **Billing (prod, external MCP):** every `tools/call` is pay-per-call on X Layer USDT0 via OKX Broker (~$0.1 flat SKU) — **including** `search_tools`. Unmetered: `GET /mcp`, `initialize`, `tools/list`. **Website UI browse may stay free**; remote agent clients and marketplace listings are paid. When OKX is off, CDP/Base fallback may meter only premium tools (`check_endpoint_health`, `export_toolkit`, `recommend_verified_tool`, `gap_audit`). Agent Sync needs a linked token **and**, when OKX is active, payment on `tools/call`.
- **Listing policy:** external directories (OKX, Smithery, mcp.so, …) list this MCP as **paid**, never “free discovery”. See `docs/listings/directory-forms.md` §Product policy.
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
`link_url` pointing to `/connect#agent-sync`. Read-only tools need no linked token; when OKX A2MCP is active they still require payment on `tools/call`.

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

Copy-paste payloads: `docs/listings/directory-forms.md`.

| Channel | URL / artifact | Status |
|---------|----------------|--------|
| Official MCP Registry | [io.github.Coinyak/onchainai](https://registry.modelcontextprotocol.io) **v0.2.1** (latest) | **Published 2026-07-12** — paid tools/call description live |
| web3-mcp-hub | [rudazy/web3-mcp-hub#1](https://github.com/rudazy/web3-mcp-hub/pull/1) | Open · **paid copy pushed** 2026-07-12 |
| awesome-crypto-mcp-servers | [hive-intel/awesome-crypto-mcp-servers#209](https://github.com/hive-intel/awesome-crypto-mcp-servers/pull/209) | Open · **paid copy pushed** 2026-07-12 |
| awesome-x402 | [xpaysh/awesome-x402#811](https://github.com/xpaysh/awesome-x402/pull/811) | Open · **submitted 2026-07-12** (MCP section, paid remote) |
| Self catalog | [onchain-ai.xyz/tools/onchainai](https://www.onchain-ai.xyz/tools/onchainai) | Seeded (official); site browse free |
| Smithery | [coinyak/onchainai](https://smithery.ai/servers/coinyak/onchainai) (also [hoyeon4315/onchainai](https://smithery.ai/servers/hoyeon4315/onchainai)) | **Published 2026-07-12** · external `https://www.onchain-ai.xyz/mcp` (paid tools/call) |
| mcp.so | [chatmcp/mcpso#3123](https://github.com/chatmcp/mcpso/issues/3123) | **Submitted 2026-07-12** (issue + full listing comment) |
| PulseMCP / Glama / mcpservers.org | browser forms | Opened submit flows; no public write API (Cloudflare / login) — may crawl Official Registry |
| awesome-mcp-servers (punkpeye / appcypher) | mcp-submit | Already listed (tool reported) |
| OKX AI Agent Marketplace | [okx.ai/agents](https://okx.ai/agents) — ASP #4609 | **Re-submitted 2026-07-12** · 1 SKU `$0.1` · update tx `0x5bb50900…` · AI review “suggested pass”; activate still pending OKX QA |
| x402 Bazaar (seller) | Multi-price CDP + bazaar extension | **LIVE total=3** merchant `0x2af05c…` — health `$0.001`, recommend `$0.01`, gap_audit `$0.01` (Base USDC). OKX Path A separate `$0.1` flat. |
| Base Builder Code | [dashboard.base.org](https://dashboard.base.org) | Applied `bc_ljttbnhv` |

## Troubleshooting

| Symptom | Explanation |
|---|---|
| Browser shows JSON on GET `/mcp` | Expected discovery payload. Tool calls use POST JSON-RPC from an MCP client. |
| `429 Too Many Requests` | Per-IP rate limit. Back off and retry. |
| Tool not found by slug | Slugs come from `search_tools` results — don't guess them. |
| Client only supports stdio | Use the `mcp-remote` bridge above. |
| `Connection closed` on `check_endpoint_health` | Expected on Claude Code/Cursor — HTTP 402 is not an MCP JSON-RPC result. **OKX on:** every MCP `tools/call` (including `get_tool_detail`) is metered until paid; use website UI / unmetered `tools/list` / `GET /mcp` discovery, or REST with an x402 wallet client. **OKX off (CDP fallback):** free `get_tool_detail` for x402 flags, or paid REST `GET /api/v2/premium/check-endpoint-health/{slug}`. |
