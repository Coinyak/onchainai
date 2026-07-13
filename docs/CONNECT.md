# Connect OnchainAI to Your Agent

> Canonical connection guide. Live interactive version: [onchain-ai.xyz/connect](https://www.onchain-ai.xyz/connect).

OnchainAI exposes a **hybrid** MCP surface (same tools, different billing):

| Path | Who | Billing |
|------|-----|---------|
| `https://www.onchain-ai.xyz/mcp` | Site Connect hub, Claude/Cursor plugin, direct agents | **Free discovery** (`search_tools`, detail, install guide, categories, compare, …). **Premium always paid:** `export_toolkit`, `recommend_verified_tool`, `gap_audit` at **$0.01 USDC** (Base, Axis B) or OKX fallback; `check_endpoint_health` ~$0.001 USDC (K2/CDP). |
| `https://www.onchain-ai.xyz/mcp/okx` | **OKX marketplace A2MCP only** | **Pay-per-call** ~$0.1 USDT0 on X Layer (OKX Broker) for every `tools/call` when the OKX gate is active. |

```
# Default for users of this site / plugin (free discovery) — USE THIS
https://www.onchain-ai.xyz/mcp

# OKX marketplace listing only (paid package) — do not use unless integrating OKX A2MCP
https://www.onchain-ai.xyz/mcp/okx
```

- Transport: **streamable HTTP** (JSON-RPC 2.0 over `POST`; `GET` returns discovery JSON 200)
- Auth: none on public tools. Rate limited per IP.
- **Unmetered on both paths:** `GET`, `initialize`, `tools/list`.
- **Website UI browse** stays free (same catalog as free MCP discovery).
- Agent Sync still needs a linked Bearer token (account link ≠ payment).
- **Listing policy:** OKX (and any marketplace that requires a paid SKU) must use **`/mcp/okx`**. Smithery/mcp.so/etc. that mirror the free site path use `/mcp` and must not claim a paid discovery SKU for that URL. See `docs/listings/directory-forms.md` §Product policy.
- Official endpoints are only `…/mcp` and `…/mcp/okx`. Anything else claiming to be OnchainAI is not ours.

### Public `/mcp` tool billing (agents: default path)

| Free discovery | Premium (x402 on `/mcp` only) |
|----------------|-------------------------------|
| `search_tools`, `get_tool_detail`, `get_install_guide`, `list_categories`, `get_dashboard_snapshot`, `compare_tools`, `get_price_history`, `get_x402_trends` | `export_toolkit`, `recommend_verified_tool`, `gap_audit` → **$0.01 USDC**; `check_endpoint_health` → ~**$0.001 USDC** |

Claude Code / Cursor cannot complete x402 handshakes; free tools work out of the box. Premium calls may show “Connection closed” until paid via a wallet-capable client or REST premium routes.

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
`link_url` pointing to `/connect#agent-sync`. Read-only discovery tools on
**`/mcp` need no linked token and no payment.** Agent Sync is account link only.
(If you wrongly connect to **`/mcp/okx`**, every `tools/call` is pay-per-call when
the OKX gate is active — coding agents should not use that path.)

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
- Third-party x402/paid tools in the **catalog** → `get_install_guide` includes
  an `x402_notice` and referral metadata. Disclose price and wallet requirement
  *before* the user calls that third-party tool.
  `payment_verified`/`x402_endpoint_verified`/`price_verified` all true =
  "operator verified"; otherwise say "not yet verified".
- OnchainAI is a **directory** MCP (discovery/metadata). It does **not** custody
  wallets or proxy third-party payments. Optional **OnchainAI-owned** premium
  tools on `/mcp` (`export_toolkit`, `recommend_verified_tool`, `gap_audit` at
  **$0.01 USDC**; `check_endpoint_health` ~**$0.001 USDC**) settle to our payee
  via x402 — not the same as catalog third-party x402 metadata.

## Listed on (external discovery)

Copy-paste payloads: `docs/listings/directory-forms.md`.

| Channel | URL / artifact | Status |
|---------|----------------|--------|
| Official MCP Registry | [io.github.Coinyak/onchainai](https://registry.modelcontextprotocol.io) v0.2.0 live | Live; free `/mcp` — republish if description still says all-paid |
| web3-mcp-hub | [rudazy/web3-mcp-hub#1](https://github.com/rudazy/web3-mcp-hub/pull/1) | Open · switch copy to free `/mcp` blurb |
| awesome-crypto-mcp-servers | [hive-intel/awesome-crypto-mcp-servers#209](https://github.com/hive-intel/awesome-crypto-mcp-servers/pull/209) | Open · switch copy to free `/mcp` blurb |
| Self catalog | [onchain-ai.xyz/tools/onchainai](https://www.onchain-ai.xyz/tools/onchainai) | Seeded (official); site browse free |
| Smithery / mcp.so / PulseMCP / Glama | `docs/listings/directory-forms.md` | Free `/mcp` blurb · publish with account login |
| OKX AI Agent Marketplace | [okx.ai/agents](https://okx.ai/agents) — ASP #4609 | **Must list `https://www.onchain-ai.xyz/mcp/okx`** · 1 SKU `$0.1` · re-point listing after hybrid deploy · was on `/mcp` |
| x402 Bazaar (seller) | CDP merchant discovery | **Blocked** — needs `EVM_PRIVATE_KEY` (Base USDC) for one paid settle |
| Base Builder Code | [dashboard.base.org](https://dashboard.base.org) | Applied `bc_ljttbnhv` |

## Troubleshooting

| Symptom | Explanation |
|---|---|
| Browser shows JSON on GET `/mcp` | Expected discovery payload. Tool calls use POST JSON-RPC from an MCP client. |
| `429 Too Many Requests` | Per-IP rate limit. Back off and retry. |
| Tool not found by slug | Slugs come from `search_tools` results — don't guess them. |
| Client only supports stdio | Use the `mcp-remote` bridge above. |
| HTTP 402 on `search_tools` via Claude | You are on the **paid** path (`/mcp/okx`) or a stale client. Site/plugin must use **`https://www.onchain-ai.xyz/mcp`** (free discovery). |
| `Connection closed` on `check_endpoint_health` | Expected on Claude Code/Cursor when that tool is x402-gated — HTTP 402 is not an MCP JSON-RPC result. Prefer free `get_tool_detail` for x402 flags, or paid REST `GET /api/v2/premium/check-endpoint-health/{slug}` with an x402 wallet client. **OKX agents** using `/mcp/okx` pay per call for the whole package. |
