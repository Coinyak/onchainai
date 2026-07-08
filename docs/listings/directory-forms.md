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

> Registration is wallet-login SPA only (no curl). **W7 partial (2026-07-07):** public OKX Onchain OS Payments uses an own **Broker** (not CDP Facilitator); confirm `okx.ai/agents` A2MCP settle path after wallet login. If incompatible, submit **discovery-only** (free MCP tools) per Plan B — see `docs/superpowers/specs/2026-07-07-okx-x402-infra-waves.md` §3.4.

| Field | Value |
|-------|-------|
| Provider | OnchainAI |
| Service | OnchainAI Trust Probe (x402 endpoint liveness) |
| Endpoint | `https://www.onchain-ai.xyz/mcp` (POST JSON-RPC, streamable-http) |
| Payment model | A2MCP / x402 (HTTP 402) |
| Paid tools | `check_endpoint_health` (Trust Probe, $0.003 USDT on OKX / $0.001 USDC direct MCP) · `export_toolkit` (Toolkit Export, $0.01 USDT/USDC) · `recommend_verified_tool` (Verified Recommendation, $0.01 USDC direct MCP) · `gap_audit` (Catalog Gap Audit, $0.05 USDC direct MCP) |
| Payout wallet | `0x2af05c1661da38a2919dc27b4c8b71cb91c30017` (Base USDC) — **same as** prod `X402_PAY_TO_ADDRESS` and `site_settings.default_referral_payout_address` |
| Free tools (same endpoint, not OKX-listed) | `search_tools`, `get_tool_detail`, `get_install_guide`, `list_categories`, `get_dashboard_snapshot`, `compare_tools`, `get_price_history`, `get_x402_trends` |
| Repo | https://github.com/Coinyak/onchainai |
| Registry cross-list | `io.github.Coinyak/onchainai` v0.2.0 (`server.json`) |