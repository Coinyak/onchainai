# External directory submission copy-paste

> Generated for operator one-click submit. Update the status table in `docs/CONNECT.md` after each success.

## Done (2026-07-04)

| Item | Link |
|------|------|
| Prod catalog self-list | `slug=onchainai` via `scripts/seed-onchainai-listing.mjs` |
| web3-mcp-hub PR | https://github.com/rudazy/web3-mcp-hub/pull/1 |
| awesome-crypto-mcp-servers PR | https://github.com/hive-intel/awesome-crypto-mcp-servers/pull/209 |
| GitHub repo topics | `mcp`, `x402`, `crypto`, `ai-agents`, `rust`, `nextjs`, `web3` |
| `server.json` | **Published** — `io.github.Coinyak/onchainai` v0.2.0 (2026-07-04) |
| MCP HTTP proof | https://www.onchain-ai.xyz/.well-known/mcp-registry-auth (deployed) |
| MCP DNS TXT | `scripts/godaddy-mcp-registry-txt.sh` (apex; GoDaddy API keys) |

## Smithery — https://smithery.ai

| Field | Value |
|-------|-------|
| Name | OnchainAI |
| URL | https://www.onchain-ai.xyz/mcp |
| Transport | HTTP / streamable-http |
| Description | Crypto tool directory MCP — search_tools, get_tool_detail, get_install_guide, list_categories, get_dashboard_snapshot. Free discovery for x402 APIs with trust metadata. |
| Repo | https://github.com/Coinyak/onchainai |

## mcp.so — https://mcp.so

| Field | Value |
|-------|-------|
| Server name | OnchainAI |
| MCP URL | https://www.onchain-ai.xyz/mcp |
| Tags | crypto, web3, x402, mcp, ai-agents |
| Description | Discover and vet crypto MCP/CLI/SDK/API/x402 tools for AI agents. |

## PulseMCP — https://www.pulsemcp.com

| Field | Value |
|-------|-------|
| Name | OnchainAI |
| Endpoint | https://www.onchain-ai.xyz/mcp |
| Category | Crypto / Web3 |
| GitHub | https://github.com/Coinyak/onchainai |

## Glama — https://glama.ai/mcp/servers

| Field | Value |
|-------|-------|
| Name | OnchainAI |
| Remote URL | https://www.onchain-ai.xyz/mcp |
| Homepage | https://www.onchain-ai.xyz/connect |

## Cursor Directory

Use deeplink from https://www.onchain-ai.xyz/connect or MCP URL above.

## Base Builder Code — https://dashboard.base.org

1. Register app **OnchainAI**
2. Verify domain **onchain-ai.xyz**
3. Settings → Builder Codes → copy `bc_…`
4. Set in Admin → Site settings → `x402_builder_code` — **applied:** `bc_ljttbnhv` (2026-07-04)

## MCP Registry — https://registry.modelcontextprotocol.io

```bash
# After DNS TXT on onchain-ai.xyz (publisher proves domain ownership)
npm i -g @modelcontextprotocol/publisher  # or per registry docs
mcp-publisher publish ./server.json
```

Repo ships `server.json` at project root (`name`: `xyz.onchain-ai/onchainai`).

## x402 Bazaar (seller — no form)

Index appears after CDP Facilitator **settle** on a paid route with Bazaar discovery metadata.

- Facilitator: `https://api.cdp.coinbase.com/platform/v2/x402`
- Self-check: `GET https://api.cdp.coinbase.com/platform/v2/x402/discovery/merchant?payTo=<X402_PAY_TO_ADDRESS>`
- Docs: https://docs.cdp.coinbase.com/x402/bazaar

## x402 community

- Slack: http://slack.x402.org — intro in #general: discovery directory, no custody
- GitHub: https://github.com/x402-foundation/x402 — ecosystem PR when list location confirmed

## OKX AI Agent Marketplace — https://okx.ai/agents

> Registration is wallet-login SPA only (no curl). **W7 resolved 2026-07-08 (Path A):** OKX Agent Payments Protocol uses OKX Broker facilitator on X Layer (eip155:196) with USDT0. Handler-level OKX gate implemented (`require_okx_payment`) for `/mcp` JSON-RPC and REST endpoints. Single price $0.1/call for all premium tools.
>
> **Rejection (ASP #4609, 2026-07-08):** "A2MCP service has not been integrated with the OKX Agent Payments Protocol standard." Root cause: prod used CDP/Base USDC, not OKX Broker/X Layer USDT0. Fix: implemented OKX handler-level gate + Railway env sync.
>
> **Rejection [T2] (2026-07-08/09):** "A2MCP service is missing a valid public HTTPS endpoint — provide one." Action: set A2MCP service URL to **`https://www.onchain-ai.xyz/mcp`** in the agent listing form and resubmit. Code also pins 402 `resource.url` to the public origin (never Railway `*.up.railway.app`).

### A2MCP endpoint (T2 — required field)

```
https://www.onchain-ai.xyz/mcp
```

- Transport: streamable HTTP · `POST` JSON-RPC 2.0 · `GET` returns discovery JSON (200)
- Do **not** use: homepage only, `http://`, localhost, or `*.up.railway.app`

### Registration metadata (Path A — full A2MCP)

| Field | Value |
|-------|-------|
| Agent name | OnchainAI — Crypto tool directory with trust probes, gap audits, and verified recommendations |
| Provider | OnchainAI |
| Service | OnchainAI Crypto Tool Directory (MCP + x402 premium) |
| **A2MCP endpoint (T2)** | **`https://www.onchain-ai.xyz/mcp`** (POST JSON-RPC, streamable-http) |
| Payment model | A2MCP / x402 (HTTP 402, OKX Agent Payments Protocol) |
| Price | $0.1 USDT0 per call (single price for all premium tools) |
| Network | X Layer (eip155:196) |
| Asset | USDT0 — `0x779ded0c9e1022225f8e0630b35a9b54be713736` (6 decimals) |
| Facilitator | OKX Broker (`https://web3.okx.com/api/v6/pay/x402`) |
| Payout wallet | `0x2af05c1661da38a2919dc27b4c8b71cb91c30017` (X Layer) |
| Premium tools | `check_endpoint_health` (Trust Probe), `export_toolkit` (Toolkit Export), `recommend_verified_tool` (Verified Recommendation), `gap_audit` (Catalog Gap Audit) — all $0.1/call |
| Free tools (same endpoint) | `search_tools`, `get_tool_detail`, `get_install_guide`, `list_categories`, `get_dashboard_snapshot`, `compare_tools`, `get_price_history`, `get_x402_trends` |
| Logo | OnchainAI official brand logo (uploaded at registration) |
| Repo | https://github.com/Coinyak/onchainai |
| Docs | https://www.onchain-ai.xyz/connect |
| Registry cross-list | `io.github.Coinyak/onchainai` v0.2.0 (`server.json`) |

### English resubmit copy-paste (Agent conversation / listing form)

Paste into the OKX agent listing / conversation interface when updating T2:

```
Please update Agent "OnchainAI" A2MCP public HTTPS endpoint and resubmit.

A2MCP service endpoint (public HTTPS):
https://www.onchain-ai.xyz/mcp

Transport: streamable HTTP (JSON-RPC 2.0 over POST /mcp)
GET /mcp returns discovery metadata (200); POST /mcp tools/call returns HTTP 402
with PAYMENT-REQUIRED when unpaid (OKX Agent Payments Protocol / x402 v2).

Payment:
- Model: A2MCP pay-per-call
- Network: X Layer (eip155:196)
- Asset: USDT0 (0x779ded0c9e1022225f8e0630b35a9b54be713736, 6 decimals)
- Price: $0.1 USDT0 per call
- Pay-to: 0x2af05c1661da38a2919dc27b4c8b71cb91c30017
- Facilitator: OKX Broker (https://web3.okx.com)

Smoke checks for reviewers:
- GET  https://www.onchain-ai.xyz/mcp → 200 JSON (endpoint, tools[])
- POST https://www.onchain-ai.xyz/mcp  method=initialize → 200 serverInfo.name=onchainai
- POST https://www.onchain-ai.xyz/mcp  method=tools/call name=search_tools (no payment) → 402 + payment-required header (network eip155:196, asset USDT0)

Website: https://www.onchain-ai.xyz
Connect docs: https://www.onchain-ai.xyz/connect
Repo: https://github.com/Coinyak/onchainai
```

### Pre-resubmit proof checklist

```bash
# Discovery
curl -sS -o /dev/null -w "%{http_code}\n" https://www.onchain-ai.xyz/mcp
# expect 200

# Initialize
curl -sS -X POST https://www.onchain-ai.xyz/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"okx-review","version":"0.0.1"}}}'
# expect 200 + serverInfo

# Payment challenge (402)
curl -sS -D - -o /dev/null -X POST https://www.onchain-ai.xyz/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_tools","arguments":{"query":"uniswap"}}}'
# expect HTTP/2 402 and payment-required header
```

### Railway env vars (set by `deploy-railway.sh`)

| Variable | Description |
|----------|-------------|
| `OKX_API_KEY` | OKX platform API key (HMAC auth) |
| `OKX_SECRET_KEY` | OKX platform secret key |
| `OKX_PASSPHRASE` | OKX API passphrase |
| `OKX_PAY_TO_ADDRESS` | X Layer payout address (`0x2af05c1661da38a2919dc27b4c8b71cb91c30017`) |
| `OKX_PREMIUM_PRICE_USD` | Price per call (`$0.1`, defaults to $0.1 if unset) |