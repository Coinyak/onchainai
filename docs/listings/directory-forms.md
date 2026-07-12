# External directory submission copy-paste

> Generated for operator one-click submit. Update the status table in `docs/CONNECT.md` after each success.

## Product policy (2026-07-12 owner)

| Surface | Billing |
|---------|---------|
| **Website** (`onchain-ai.xyz` UI, public web browse) | May stay free (discovery / catalog browse) |
| **External MCP** (OKX, Smithery, mcp.so, PulseMCP, Glama, Cursor Directory, agent clients on `POST /mcp`) | **Paid** — same public endpoint as OKX Path A |
| **Unmetered on MCP** | `GET /mcp`, `initialize`, `tools/list` only |
| **Metered on MCP (prod, OKX gate on)** | **Every** `tools/call` — incl. `search_tools` — **$0.1** X Layer USDT0 (OKX Broker) |

**Rules for all external listings**

1. **Do not** claim “free discovery”, “free MCP”, or “no payment” for `POST /mcp` tools/call.
2. Package like OKX: **one endpoint**, multi-tool capability in the description, fee in the fee field (or “x402 pay-per-call” if the directory has no fee field).
3. Value-first copy (trust / install-risk / vet before install). Do not lead with `$0.1`.
4. Endpoint always `https://www.onchain-ai.xyz/mcp` — never Railway hostname, http, or localhost.
5. Smoke before submit: unpaid `tools/call` → **HTTP 402** + payment-required.

### Canonical external MCP blurb (all directories)

```
Find, compare, and vet crypto MCP/CLI/SDK/API tools with trust scores and install-risk before your agent installs anything. Search, detail, compare, install guides, x402 metadata, live endpoint probes, verified picks, and gap audits — maintained catalog, not a raw link dump. Remote MCP: pay-per-call (x402) on tools/call; website browse may be free.
```

**Fee line (when the form has a fee / pricing field):** `0.1` USDT0 per `tools/call` (X Layer) — same SKU as OKX ASP #4609.  
**If no fee field:** state “x402 micropayment required on tools/call” in the description; never “free”.

## Done (2026-07-04)

| Item | Link |
|------|------|
| Prod catalog self-list | `slug=onchainai` via `scripts/seed-onchainai-listing.mjs` |
| web3-mcp-hub PR | https://github.com/rudazy/web3-mcp-hub/pull/1 |
| awesome-crypto-mcp-servers PR | https://github.com/hive-intel/awesome-crypto-mcp-servers/pull/209 |
| GitHub repo topics | `mcp`, `x402`, `crypto`, `ai-agents`, `rust`, `nextjs`, `web3` |
| `server.json` | **Published** — `io.github.Coinyak/onchainai` **v0.2.1** (2026-07-12) paid description live |
| OKX ASP #4609 | **Re-submitted 2026-07-12** — Path A `$0.1` bundled SKU; profile update on-chain; activate pending OKX |
| awesome-crypto PR #209 | Paid listing copy pushed to fork branch |
| web3-mcp-hub PR #1 | Paid listing copy pushed to fork branch |
| MCP HTTP proof | https://www.onchain-ai.xyz/.well-known/mcp-registry-auth (deployed) |
| MCP DNS TXT | `scripts/godaddy-mcp-registry-txt.sh` (apex; GoDaddy API keys) |

## Smithery — https://smithery.ai

| Field | Value |
|-------|-------|
| Name | OnchainAI |
| URL | https://www.onchain-ai.xyz/mcp |
| Transport | HTTP / streamable-http |
| Pricing | Paid — x402 pay-per-call on `tools/call` (~$0.1 / call when OKX gate active) |
| Description | Find, compare, and vet crypto tools with trust and install-risk before install. Search, compare, probes, verified picks, gap audits. Remote MCP tools/call is pay-per-call (x402); not a free public dump. |
| Repo | https://github.com/Coinyak/onchainai |

## mcp.so — https://mcp.so

| Field | Value |
|-------|-------|
| Server name | OnchainAI |
| MCP URL | https://www.onchain-ai.xyz/mcp |
| Tags | crypto, web3, x402, mcp, ai-agents, paid |
| Description | Crypto tool intelligence MCP for agents — ranked search, trust/install-risk, compare, x402 probes. Pay-per-call (x402) on tools/call. |

## PulseMCP — https://www.pulsemcp.com

| Field | Value |
|-------|-------|
| Name | OnchainAI |
| Endpoint | https://www.onchain-ai.xyz/mcp |
| Category | Crypto / Web3 |
| Pricing note | x402 pay-per-call on tools/call |
| GitHub | https://github.com/Coinyak/onchainai |
| Description | Use canonical external MCP blurb above. |

## Glama — https://glama.ai/mcp/servers

| Field | Value |
|-------|-------|
| Name | OnchainAI |
| Remote URL | https://www.onchain-ai.xyz/mcp |
| Homepage | https://www.onchain-ai.xyz/connect |
| Description | Use canonical external MCP blurb above (paid tools/call). |

## Cursor Directory

Use deeplink from https://www.onchain-ai.xyz/connect or MCP URL above.  
Listing text: same paid blurb — do not mark as free MCP.

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

> **ASP #4609** · Path A A2MCP · re-submit via `./scripts/register-okx-asp.sh` (Agentic Wallet login required).
>
> **Copy policy:** public listing text is **value-first** (outcome: vet tools before install). Fee lives in the structured fee field only — do not lead marketing copy with `$0.1`. Do not claim free discovery on this SKU: when OKX gate is on, every MCP `tools/call` is metered.
>
> **History:** Rejected 2026-07-08 (protocol: CDP/Base vs OKX Broker/X Layer USDT0) → fixed. Rejected [T2] (missing public HTTPS endpoint) → endpoint `https://www.onchain-ai.xyz/mcp`; 402 `resource.url` pinned to public origin (PR #76).

### Policy — OKX bundled SKU = external paid standard (intentional)

**Owner decision (2026-07-12):** website may stay free; **every external MCP surface is paid**, same model as OKX Path A.

When OKX credentials are active (prod), marketplace + any remote client on `POST /mcp` is **one flat A2MCP SKU**. Every MCP `tools/call` is metered — including discovery tools. No free/premium tool split on external listings.

| Surface | Free / unmetered | Metered |
|---------|------------------|---------|
| OKX gate **off** (dev / emergency) | Discovery `tools/call` + unmetered methods | Premium tools only (CDP/Base) |
| OKX gate **on** (prod Path A) | `GET /mcp`, `initialize`, `tools/list`, **website UI** | **All** MCP `tools/call` (incl. `search_tools`) |
| Always | No custody; third-party x402 is metadata only | — |

Qodo / free-tier bots may flag “discovery must not enforce x402” — **accept**: external paid is the product rule; do not “fix” by claiming free discovery on listings while prod returns 402.

### Quick reference (do not confuse)

| What | Value |
|------|--------|
| **OKX A2MCP endpoint field** | `https://www.onchain-ai.xyz/mcp` |
| **Never list as endpoint** | `*.up.railway.app`, homepage-only, `http://`, localhost |
| **402 `resource.url` (code)** | Same public origin via `SITE_ORIGIN` (`src/server/okx_payment.rs`, PR #76) |
| **Listing copy source of truth** | `scripts/register-okx-asp.sh` (this doc must match) |
| **Fee** | Structured field `0.1` only — not the marketing headline |
| **Smoke** | `GET /mcp` → 200 · `initialize` → 200 · unpaid `tools/call` → **402** |
| **False negative** | `onchainos agent x402-check` is often GET-only (“not 402”); ignore for pass/fail |

### A2MCP endpoint (required)

```
https://www.onchain-ai.xyz/mcp
```

- Transport: streamable HTTP · `POST` JSON-RPC 2.0 · `GET` discovery JSON 200
- Do **not** use: homepage only, `http://`, localhost, `*.up.railway.app`
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
| Endpoint | `https://www.onchain-ai.xyz/mcp` |
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
| Metered | All MCP `tools/call` methods when OKX package is active (including `search_tools`, compare, probes, Agent Sync tools when linked) |
| Unmetered | `GET /mcp` discovery; `initialize`; `tools/list`; website UI |
| SKU shape | One flat fee field — no free/premium tool split on the OKX listing |
| Resubmit | `OKX_ASP_AGENT_ID=4609 ./scripts/register-okx-asp.sh` |
| Smoke | GET `/mcp` → 200; POST `initialize` → 200; unpaid POST `tools/call` → 402 + payment-required (X Layer USDT0) |

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

# Payment challenge (402) — this is the real A2MCP check, not GET-only x402-check
curl -sS -D - -o /dev/null -X POST https://www.onchain-ai.xyz/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_tools","arguments":{"query":"uniswap"}}}'
# expect 402 and payment-required header
```

### Railway env vars (set by `deploy-railway.sh`)

| Variable | Description |
|----------|-------------|
| `OKX_API_KEY` | OKX platform API key (HMAC auth) |
| `OKX_SECRET_KEY` | OKX platform secret key |
| `OKX_PASSPHRASE` | OKX API passphrase |
| `OKX_PAY_TO_ADDRESS` | X Layer payout address (`0x2af05c1661da38a2919dc27b4c8b71cb91c30017`) |
| `OKX_PREMIUM_PRICE_USD` | Per-call fee string for app config (`$0.1`; defaults to `$0.1` if unset) — **not** listing marketing copy |