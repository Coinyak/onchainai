# External directory submission copy-paste

> Generated for operator one-click submit. Update the status table in `docs/CONNECT.md` after each success.

## Product policy (2026-07-13 owner — hybrid billing)

| Surface | Endpoint | Billing |
|---------|----------|---------|
| **Website** (`onchain-ai.xyz` UI) | N/A | Free catalog browse |
| **Site / plugin / direct agents** | `POST https://www.onchain-ai.xyz/mcp` | **Free discovery** (`search_tools`, detail, install guide, categories, compare, …). **Premium on this path only:** `export_toolkit`, `recommend_verified_tool`, `gap_audit` = **$0.01 USDC** (Base); `check_endpoint_health` ≈ **$0.001 USDC**. |
| **OKX marketplace A2MCP only** | `POST https://www.onchain-ai.xyz/mcp/okx` | **Paid package** — every `tools/call` ~**$0.1** X Layer USDT0 (OKX Broker) when gate active |
| **Unmetered on both MCP paths** | — | `initialize`, `tools/list` (plus `GET /mcp` discovery; plain `GET /mcp/okx` answers the 402 x402 challenge — OKX endpoint review requirement) |

**Rules**

1. **OKX (and paid marketplaces):** list **`/mcp/okx` only**. Fee field / copy = pay-per-call $0.1. Smoke on that path: plain `GET` → **HTTP 402** (OKX `x402-check` rejects 200 as "not a valid x402 service") and unpaid `tools/call` → **HTTP 402** + payment-required.
2. **Free directories / site Connect / plugin / coding agents:** list **`/mcp`**. Claim free discovery. Do **not** point free listings or Claude/Cursor/plugin at `/mcp/okx`.
3. Never use Railway hostname, bare http, or localhost in public listings.
4. Value-first copy (trust / install-risk / vet before install). Lead with capability; fee only in the fee field (OKX path). On free listings, optional one-line note that a few premium tools may 402 is fine — do **not** say every `tools/call` is paid on `/mcp`.
5. Endpoint host always `https://www.onchain-ai.xyz`.

### Canonical free MCP blurb (site, Claude plugin, free directories)

```
Find, compare, and vet crypto MCP/CLI/SDK/API tools with trust scores and install-risk before your agent installs anything. Search, detail, compare, install guides, and x402 metadata — free discovery on https://www.onchain-ai.xyz/mcp. Optional premium tools (export/recommend/gap audit $0.01 USDC; live probe ~$0.001) may require x402.
```

### Canonical OKX / paid-package blurb

```
Find, compare, and vet crypto tools with trust and install-risk before install. Search, detail, compare, probes, verified picks, gap audits. OKX A2MCP package: pay-per-call (x402) on every tools/call via https://www.onchain-ai.xyz/mcp/okx (~$0.1 USDT0 X Layer).
```

**Fee line (OKX / paid path only):** `0.1` USDT0 per `tools/call` (X Layer) — ASP #4609 package on **`/mcp/okx`**.

## Done (2026-07-04)

| Item | Link |
|------|------|
| Prod catalog self-list | `slug=onchainai` via `scripts/seed-onchainai-listing.mjs` |
| web3-mcp-hub PR | https://github.com/rudazy/web3-mcp-hub/pull/1 |
| awesome-crypto-mcp-servers PR | https://github.com/hive-intel/awesome-crypto-mcp-servers/pull/209 |
| GitHub repo topics | `mcp`, `x402`, `crypto`, `ai-agents`, `rust`, `nextjs`, `web3` |
| `server.json` | **Published** — `io.github.Coinyak/onchainai` v0.2.0 (2026-07-04); republish if description still says all-paid (free `/mcp` blurb) |
| OKX ASP #4609 | **LISTED on OKX.AI 2026-07-17** (passed review; visible/searchable/recommendable) · endpoint `https://www.onchain-ai.xyz/mcp/okx` · service `33054` · fee `$0.1` · tx `0x15819294…` · contract: GET + unpaid `tools/call` must keep answering 402 (guarded by `scripts/k2-prod-smoke.sh`; needs `OKX_*` env vars on Railway) |
| awesome-crypto PR #209 | Switch fork copy to free `/mcp` blurb (see `awesome-crypto-mcp-servers.md`) |
| web3-mcp-hub PR #1 | Switch fork copy to free `/mcp` blurb |
| MCP HTTP proof | https://www.onchain-ai.xyz/.well-known/mcp-registry-auth (deployed) |
| MCP DNS TXT | `scripts/godaddy-mcp-registry-txt.sh` (apex; GoDaddy API keys) |

## Smithery — https://smithery.ai

| Field | Value |
|-------|-------|
| Name | OnchainAI |
| URL | https://www.onchain-ai.xyz/mcp |
| Transport | HTTP / streamable-http |
| Pricing | Free discovery on `/mcp` (default listing). Use `/mcp/okx` only if listing a paid SKU. |
| Description | Use canonical free MCP blurb above (or OKX blurb if submitting the paid package URL). |
| Repo | https://github.com/Coinyak/onchainai |

## mcp.so — https://mcp.so

| Field | Value |
|-------|-------|
| Server name | OnchainAI |
| MCP URL | https://www.onchain-ai.xyz/mcp |
| Tags | crypto, web3, x402, mcp, ai-agents |
| Description | Crypto tool intelligence MCP for agents — ranked search, trust/install-risk, compare. Free discovery on public `/mcp`. |

## PulseMCP — https://www.pulsemcp.com

| Field | Value |
|-------|-------|
| Name | OnchainAI |
| Endpoint | https://www.onchain-ai.xyz/mcp |
| Category | Crypto / Web3 |
| Pricing note | Free discovery on `/mcp` |
| GitHub | https://github.com/Coinyak/onchainai |
| Description | Use canonical free MCP blurb above. |

## Glama — https://glama.ai/mcp/servers

| Field | Value |
|-------|-------|
| Name | OnchainAI |
| Remote URL | https://www.onchain-ai.xyz/mcp |
| Homepage | https://www.onchain-ai.xyz/connect |
| Description | Use canonical free MCP blurb above. |

## Cursor Directory

Use deeplink from https://www.onchain-ai.xyz/connect or MCP URL `https://www.onchain-ai.xyz/mcp`.  
Listing text: canonical free MCP blurb — free discovery on public `/mcp` (not the OKX package path).

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

> **ASP #4609** · Path A A2MCP · hybrid (2026-07-13): **list only** `https://www.onchain-ai.xyz/mcp/okx`.
> Re-submit via `./scripts/register-okx-asp.sh` after endpoint re-point (Agentic Wallet login required).
>
> **Copy policy:** public listing text is **value-first** (outcome: vet tools before install). Fee lives in the structured fee field only — do not lead marketing copy with `$0.1`. Do **not** claim free discovery on this SKU: when OKX gate is on, every `tools/call` on **`/mcp/okx`** is metered.
>
> **Hybrid note:** Site Connect, Claude/Cursor plugin, and free directories use **`/mcp`** (free discovery + small premium set). Do **not** paste `/mcp/okx` into those surfaces.
>
> **History:** Rejected 2026-07-08 (protocol: CDP/Base vs OKX Broker/X Layer USDT0) → fixed. Rejected [T2] (missing public HTTPS endpoint) → endpoint was `https://www.onchain-ai.xyz/mcp` (pre-hybrid). Hybrid splits paid package to `/mcp/okx`; 402 `resource.url` pinned to public origin (PR #76 lineage).

### Policy — OKX bundled SKU is **path-isolated** (hybrid)

**Owner decision (2026-07-13 hybrid supersedes 2026-07-12 “all external MCP paid”):**

| Surface | Free / unmetered | Metered |
|---------|------------------|---------|
| Website UI | Full catalog browse | — |
| Public `POST /mcp` (site, plugin, coding agents) | Discovery `tools/call` + unmetered methods | Premium only: `export_toolkit`, `recommend_verified_tool`, `gap_audit` ($0.01 USDC); `check_endpoint_health` (~$0.001 USDC) |
| OKX package `POST /mcp/okx` (gate **on**) | `GET`, `initialize`, `tools/list` | **All** `tools/call` (incl. `search_tools`) ~$0.1 USDT0 |
| Always | No custody; third-party x402 is metadata only | — |

Qodo / free-tier bots may still flag OKX package metering — **accept for `/mcp/okx` only**. Do **not** claim free discovery on the OKX listing URL. Free-discovery claims are correct for **`/mcp`**.

### Quick reference (do not confuse)

| What | Value |
|------|--------|
| **OKX A2MCP endpoint field** | `https://www.onchain-ai.xyz/mcp/okx` |
| **Default / free agent endpoint** | `https://www.onchain-ai.xyz/mcp` (not for OKX listing) |
| **Never list as endpoint** | `*.up.railway.app`, homepage-only, `http://`, localhost |
| **402 `resource.url` (code)** | Public origin via `SITE_ORIGIN` (`src/server/okx_payment.rs`) for `/mcp/okx` |
| **Listing copy source of truth** | `scripts/register-okx-asp.sh` (this doc must match) |
| **Fee** | Structured field `0.1` only — not the marketing headline |
| **Smoke** | `GET /mcp/okx` → 200 · `initialize` → 200 · unpaid `tools/call` → **402** |
| **False negative** | `onchainos agent x402-check` is often GET-only (“not 402”); ignore for pass/fail |

### A2MCP endpoint (required for OKX listing)

```
https://www.onchain-ai.xyz/mcp/okx
```

- Transport: streamable HTTP · `POST` JSON-RPC 2.0 · `GET` discovery JSON 200
- Do **not** use: homepage only, `http://`, localhost, `*.up.railway.app`, bare `/mcp` for this SKU
- Note: `onchainos agent x402-check` may GET-only and report “not 402”; metering is on `POST tools/call`

### Submitted listing copy (canonical — keep in sync with `scripts/register-okx-asp.sh`)

| Field | Value |
|-------|-------|
| Agent ID | `4609` |
| Name | `OnchainAI` |
| Profile description | Find, compare, and vet crypto MCP/CLI/SDK/API tools with trust scores and install-risk before your agent installs anything. |
| Service name | `OnchainAI MCP` |
| Service type | `A2MCP` |
| Service description | *(two lines, no URLs — OKX D1/D6)* see below |
| Endpoint | `https://www.onchain-ai.xyz/mcp/okx` |
| Fee (structured field only) | `0.1` USDT0 per `tools/call` (flat SKU) |
| Network / asset | X Layer (`eip155:196`) · USDT0 `0x779ded0c9e1022225f8e0630b35a9b54be713736` |
| Facilitator | OKX Broker |
| Pay-to | `0x2af05c1661da38a2919dc27b4c8b71cb91c30017` |
| Logo | `public/brand/okx-ai-agent-cover.png` (full-bleed 1:1 from official mark; OKX listing) |
| Repo / docs | https://github.com/Coinyak/onchainai · https://www.onchain-ai.xyz/connect |

**Service description (submitted):**

```
Crypto tool intelligence for AI agents: ranked search, trust and install-risk signals, side-by-side compare, install guides, x402 metadata, live endpoint probes, verified picks, and gap audits — so agents vet tools before they install or pay third parties. Maintained catalog, not a raw link dump.
Provide a JSON-RPC tools/call body (tool name plus arguments). If payment is required, settle the challenge and retry with a payment-signature header.
```

**Hooks (use in prose; not fee-led):**

- Look up, compare, and probe crypto tools before your agent installs the wrong one.
- Maintained catalog with trust/install-risk — not a raw link dump.

### Operator notes (not public marketing)

| Topic | Fact |
|-------|------|
| Metered on `/mcp/okx` | All MCP `tools/call` when OKX package gate is active (including `search_tools`, compare, probes, Agent Sync tools when linked) |
| Free on `/mcp` | Discovery tools; premium subset only ($0.01 / ~$0.001 as above) |
| Unmetered both paths | `GET` discovery; `initialize`; `tools/list`; website UI |
| SKU shape (OKX listing) | One flat fee field — no free/premium tool split on **`/mcp/okx`** |
| Resubmit | `OKX_ASP_AGENT_ID=4609 ./scripts/register-okx-asp.sh` |
| Smoke | GET `/mcp/okx` → 200; POST `initialize` → 200; unpaid POST `tools/call` → 402 + payment-required (X Layer USDT0) |

```bash
# Discovery (OKX package path)
curl -sS -o /dev/null -w "%{http_code}\n" https://www.onchain-ai.xyz/mcp/okx
# expect 200

# Initialize
curl -sS -X POST https://www.onchain-ai.xyz/mcp/okx \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"okx-review","version":"0.0.1"}}}'
# expect 200 + serverInfo

# Payment challenge (402) — this is the real A2MCP check, not GET-only x402-check
curl -sS -D - -o /dev/null -X POST https://www.onchain-ai.xyz/mcp/okx \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_tools","arguments":{"query":"uniswap"}}}'
# expect 402 and payment-required header on /mcp/okx when gate active

# Contrast: free discovery on public /mcp (search_tools should NOT 402)
curl -sS -D - -o /dev/null -X POST https://www.onchain-ai.xyz/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_tools","arguments":{"query":"uniswap"}}}'
# expect 200 JSON-RPC result (not 402)
```

### Railway env vars (set by `deploy-railway.sh`)

| Variable | Description |
|----------|-------------|
| `OKX_API_KEY` | OKX platform API key (HMAC auth) |
| `OKX_SECRET_KEY` | OKX platform secret key |
| `OKX_PASSPHRASE` | OKX API passphrase |
| `OKX_PAY_TO_ADDRESS` | X Layer payout address (`0x2af05c1661da38a2919dc27b4c8b71cb91c30017`) |
| `OKX_PREMIUM_PRICE_USD` | Per-call fee string for **`/mcp/okx`** package (`$0.1`; defaults to `$0.1` if unset) — **not** public `/mcp` premium prices |