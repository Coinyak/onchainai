# Security & Review: Hybrid MCP Billing (OKX path split)

**Date:** 2026-07-13  
**Branch:** `feat/hybrid-mcp-billing-okx-path`  
**Reviewer role:** Security & Review (read-first)  
**Scope:** Hybrid MCP billing — free public `/mcp` vs paid `/mcp/okx` package  

## Verdict

**Approve with follow-ups (no P0).**

Public discovery is free on `POST /mcp`; OKX full-package metering is path-gated to `POST /mcp/okx`; premium trio cannot run unpaid on MCP (Axis B → OKX fallback → HTTP 503). No critical free-premium leak or paid-discovery leak on the public path was found in `src/server/mcp/call.rs` or `src/server/okx_payment.rs`. No code fix applied (backend already corrected the hybrid gate).

---

## Product intent (checked)

| Intent | Code status |
|--------|-------------|
| Public `POST /mcp`: free discovery | **Pass** — OKX package gate requires `okx_package_mode` |
| Premium always paid on `/mcp` ($0.01 Axis B trio; K2 ~$0.001 health) | **Pass (MCP)** — Axis B if active, else OKX, else 503; K2 uses CDP `require_payment` (503 if unconfigured) |
| `POST /mcp/okx`: full package $0.1 OKX only | **Pass when OKX gate active** — all `OKX_GATED_ROUTES` charged via handler-level OKX |
| Default agent connect = free `/mcp` | **Pass** — Connect hub, plugin, `server.json`, frontend rewrites |

---

## Architecture (as implemented)

```
POST /mcp          → McpBillingMode::Public      → okx_package_mode=false
POST /mcp/okx      → McpBillingMode::OkxPackage  → okx_package_mode=true
GET  /mcp|/mcp/okx → billing + billing_detail JSON
```

**Gate matrix (`gate_tool_payment` in `src/server/mcp/call.rs`):**

1. **OKX package full meter** only if  
   `okx_package_mode && okx_premium_gate_active && okx_client.is_some() && is_okx_package_tool(name)`  
   → `require_okx_payment` ($0.1 USDT0, resource URL `/mcp/okx`).

2. **Premium trio** (`export_toolkit`, `recommend_verified_tool`, `gap_audit`) when not already OKX-package-charged:  
   - Axis B if `config.is_active()` → `require_axis_b_payment`  
   - else if OKX gate active → `require_okx_payment`  
   - else → **HTTP 503** `mcp_premium_misconfigured` (never free)

3. **Everything else on public path** (discovery + agent-sync tools) → no package gate;  
   `check_endpoint_health` still uses K2 CDP in dispatch when `payment_already_gated == false`.

**Double-charge guard:** `should_skip_cdp_for_okx(okx_package_mode, okx_premium_gate_active, tool)`  
requires **package path + active gate + package tool** — public `/mcp` never skips CDP for OKX.

**Startup:** `okx_premium_gate_active = okx_client.is_some()` only when OKX server init succeeds **and** `build_okx_routes()` non-empty (pay-to set). Middleware only covers REST premium paths, not JSON-RPC body tools.

---

## Findings

### P0 — Critical (billing leak / bypass)

**None.**

| Check | Result |
|-------|--------|
| Discovery paid on `/mcp` when OKX prod credentials active | **No** — package mode false on public path |
| Premium free on `/mcp` when Axis B off | **No** — OKX fallback or 503 |
| Premium free on `/mcp` when both rails off | **No** — 503 |
| Payment bypass via wrong path gating | **No** — mode is route-selected, not client-claimed |
| K2 free when CDP unconfigured | **No** — `require_payment` returns 503 if `!config.enabled` |

---

### P1 — High (security/product consistency, should fix before treating launch as complete)

#### P1-1. REST premium free when Axis B off and OKX off

**Files:** `src/server/api_v2/x402_premium.rs` (`post_recommend_verified_tool`, `post_gap_audit`)

MCP now refuses unpaid premium (503). REST still runs recommend/gap-audit unpaid when `load_mcp_premium_config` is inactive and OKX middleware is not applied.

- **Risk:** API clients can obtain the same paid product for free while MCP correctly hard-stops.
- **Intent gap:** “Premium always paid” is MCP-complete, REST-incomplete.
- **Fix:** Mirror MCP policy: if Axis B inactive and OKX not metering the route → 503 (or always require one rail).

#### P1-2. Public MCP premium OKX fallback advertises resource URL `/mcp/okx`

**Files:** `src/server/okx_payment.rs` (`okx_a2mcp_endpoint`, `okx_resource_info`); used from `call.rs` premium fallback when Axis B off but OKX active on **public** path.

- **Risk:** 402 `resource.url` is `…/mcp/okx` while the client called `…/mcp`. Facilitator/wallet UX and resource binding may confuse agents; settlement may still work if only amount/payee are checked.
- **Fix:** Pass actual request path into `require_okx_payment` / resource builder, or disable OKX fallback on public path and force Axis B / 503 only.

#### P1-3. Soft agent copy still says premium is “optional / when enabled”

**Files (examples):**

- `plugin/onchainai/skills/onchainai-crypto-tools/SKILL.md` — “operator-toggled x402”, “may charge when enabled”
- `frontend/components/connect/ConnectPageContent.tsx` — “may charge via x402 when enabled”
- Some CONNECT/llms language softens “always paid” to “optional”

Code: premium trio **never free** (402 or 503). Copy understates that.

- **Risk:** Agents may retry or assume tools are free when they get 503/402.
- **Fix:** Align copy with “always paid when rails configured; 503 if rails missing” (Axis B toggle only chooses rail, does not open free premium).

#### P1-4. REST recommend/gap-audit price becomes $0.1 when OKX middleware is live

When `okx_premium_gate_active`, REST skips Axis B (`should_skip_cdp_for_okx(true, …)`) and relies on OKX middleware at package price ($0.1), not Axis B $0.01.

- **Risk:** Same product SKU charges different amounts REST vs public MCP Axis B; agents/docs quoting $0.01 may be wrong for REST under OKX.
- **Mitigation/docs:** Explicitly document “REST premium under OKX gate = package SKU”; or stop applying OKX middleware to REST and keep Axis B $0.01 for non-marketplace.

#### P1-5. Stale comment in OKX routes builder

**File:** `src/server/okx_payment.rs` ~L161  

Still says MCP `` `/mcp` `` is handler-gated; should say `` `/mcp/okx` `` only.

---

### P2 — Medium / polish / missing coverage

#### P2-1. No unit tests for `gate_tool_payment` decision matrix

Existing tests cover:

- `should_skip_cdp_for_okx` arity/path semantics (`okx_payment_tests.rs`)
- default $0.01 (`mcp_x402`)
- GET info billing labels + tool description strings (`mcp/tests.rs`)

Missing: pure matrix tests (or thin public helper) for:

| mode | OKX active | tool | expected |
|------|------------|------|----------|
| Public | true | `search_tools` | no OKX gate |
| Public | true/false | `export_toolkit` | paid/503 not free |
| OkxPackage | true | `search_tools` | OKX 402 |
| OkxPackage | false | `search_tools` | free (degraded) |

#### P2-2. `/mcp/okx` degrades to free discovery when OKX gate inactive

Documented (“when gate active”), but marketplace listings that always claim paid must not hit a degraded free path. Ops: keep OKX credentials + pay-to live for ASP #4609 after re-pointing URL to `/mcp/okx`.

#### P2-3. Migration `040_mcp_premium_price_one_cent.sql`

Sets price/display/network defaults; does **not** enable Axis B or set payee. Correct (operator toggle). Ops must ensure pay_to + enabled for $0.01 rail in prod, else premium MCP returns 503 (safe) not $0.01.

#### P2-4. Clippy noise

`unused import: PUBLIC_TOOL_CATEGORY_IDS` in `mcp/call.rs` (and other pre-existing unused imports in lib test build).

#### P2-5. External listing lag (ops, not code)

Docs correctly mark OKX ASP must re-point to `/mcp/okx`. Community PRs (web3-mcp-hub, awesome-crypto) may still say full-paywall on `/mcp` until republished.

#### P2-6. `tools/list` is path-agnostic

Same tool schemas on both endpoints. Descriptions now state hybrid prices (good). OKX clients still see free-discovery wording until first 402 on package path — acceptable if GET `/mcp/okx` `billing_detail` is used by listing validators.

---

## Security checklist

| Topic | Assessment |
|-------|------------|
| Payment bypass via query/header claiming package mode | N/A — mode is server route enum only |
| Double charge (OKX + Axis B / CDP) | Mitigated by early return + `should_skip_cdp_for_okx` |
| Wrong path: middleware on `/mcp` JSON-RPC | Middleware is path-exact REST only; MCP body gate is handler-level |
| Dev payment bypass (`ONCHAINAI_MCP_X402_DEV_ACCEPT`) | Gated to local SIWX domain — OK |
| Secrets in client | No change; still server-only OKX/CDP keys |
| Resource host leakage (Railway) | Still pinned via `SITE_ORIGIN` / `public_resource_url` |
| Cache headers for MCP paths | `/mcp` and `/mcp/okx` both no-store style via `cache_control_for_response` |

---

## Agent-facing pricing accuracy

| Surface | Accuracy |
|---------|----------|
| `GET /mcp` billing + billing_detail | Good — free_discovery + $0.01 premium + K2 |
| `GET /mcp/okx` billing + billing_detail | Good — $0.1 package + points back to public free |
| `tool_definitions` premium trio / compare / health | Good after hybrid description updates |
| `server.json` | Points at `/mcp` free discovery + optional premium |
| Plugin skill / Connect soft copy | **P1-3** soft “when enabled” |
| `docs/MCP_EXAMPLE_PROMPTS.md` | Prices corrected ($0.01 / ~$0.001; hybrid note) |

---

## Contradictions (code vs intent / docs)

| Item | Status |
|------|--------|
| Old “every tools/call paid on /mcp when OKX on” | **Resolved** in CONNECT, free-tier guardian, directory-forms, call gate |
| `mcp_x402` module doc “free until operator enables” | **Resolved** — always paid; toggle is rail only |
| REST free premium vs MCP always paid | **Open P1-1** |
| Plugin “operator-toggled” vs always-paid | **Open P1-3** |

---

## Migration note

`migrations/040_mcp_premium_price_one_cent.sql`:

- Sets `mcp_premium_price = '$0.01'`, display/network defaults.
- Does not force `mcp_premium_enabled`.
- Aligns DB default with `DEFAULT_MCP_PREMIUM_PRICE` / atomic `10000` USDC tests.

---

## Test evidence

Commands run (lib, feature `ssr`):

```bash
cargo test --features ssr --lib okx_ -- --nocapture
# 17 passed (okx_payment + mcp_okx_info_states_package_billing)

cargo test --features ssr --lib mcp_x402 -- --nocapture
# 5 passed (incl. default_mcp_premium_price_is_one_cent)

cargo test --features ssr --lib mcp_info -- --nocapture
# 1 passed (public free_discovery + billing_detail)

cargo test --features ssr --lib tool_descriptions_state_hybrid -- --nocapture
# 1 passed (hybrid description prices)
```

**Not run:** full `cargo test --features ssr --lib` suite; browser/visual QA; live 402 smoke against prod/staging.

**Code fix in this review:** none (no P0 in exclusive critical paths).

---

## Recommended follow-ups (priority order)

1. **P1-1** Enforce always-paid (or 503) on REST recommend/gap-audit when no rail active.  
2. **P1-2** Fix OKX resource URL for public-path premium fallback (or drop fallback).  
3. **P1-3** Align plugin/Connect copy with always-paid / 503 semantics.  
4. **P2-1** Add gate matrix unit tests (extract pure predicate from `gate_tool_payment` if needed).  
5. **Ops** Re-point OKX ASP #4609 to `https://www.onchain-ai.xyz/mcp/okx`; enable Axis B payee for $0.01 SKU in prod.  
6. **P1-5** Fix stale `/mcp` comment in `build_okx_routes`.

---

## Summary table

| Severity | Count | Blocks merge? |
|----------|-------|---------------|
| P0 | 0 | — |
| P1 | 5 | No for MCP hybrid core; yes if REST must match “always paid” before ship |
| P2 | 6 | No |

**Final recommendation:** **Approve** the hybrid MCP path split for merge/deploy of the free `/mcp` + paid `/mcp/okx` design. Track P1-1/P1-2/P1-3 as fast follow-ups so product intent is consistent across REST, payment resource URLs, and agent-facing copy.
