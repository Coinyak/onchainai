<p align="center">
  <img src="public/brand/onchainai-logo.png" alt="OnchainAI logo" width="96" />
</p>

# OnchainAI

> Crypto tools, unified. Discover, vet, and install MCP, CLI, SDK, API, x402, RWA, and AI-agent tools — from the web or straight from your agent.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/API-Rust%20%2B%20Axum-orange.svg)](https://github.com/tokio-rs/axum)
[![Frontend](https://img.shields.io/badge/Frontend-Next.js-black.svg)](https://nextjs.org)
[![MCP](https://img.shields.io/badge/MCP-onchain--ai.xyz%2Fmcp-8A2BE2.svg)](https://www.onchain-ai.xyz/connect)
[![Live](https://img.shields.io/badge/Live-onchain--ai.xyz-2ea44f.svg)](https://www.onchain-ai.xyz)

**Live: [onchain-ai.xyz](https://www.onchain-ai.xyz)** · **Connect hub: [onchain-ai.xyz/connect](https://www.onchain-ai.xyz/connect)**

## Why?

Crypto tooling is fragmented across CryptoSkill, Smithery, npm, GitHub topics, and
dozens of separate registries — and most of it is hard to judge: is this MCP server
official? Is the install command safe? Does it charge x402 payments on every call?

OnchainAI auto-discovers tools from multiple sources, normalizes them into one
3-axis taxonomy (Function × Asset Class × Actor), scores trust and install risk,
and exposes everything to both humans (web UI) and agents (MCP server, plugin,
skill) — so "find me a safe tool to bridge USDC to Base" is one question, not an
afternoon of research.

## Use it from your agent (60 seconds)

Default MCP is a **no-auth directory** endpoint (discovery/metadata — not wallet
custody). **Free discovery** on public `/mcp` (search, detail, compare, install
guides, categories, …). Optional OnchainAI-owned premium tools may return HTTP 402
and settle via x402 (**$0.01 USDC** for `export_toolkit` /
`recommend_verified_tool` / `gap_audit`; ~**$0.001 USDC** for
`check_endpoint_health`). Website browse stays free.

```
https://www.onchain-ai.xyz/mcp
```

OKX marketplace integrators only: paid package path
`https://www.onchain-ai.xyz/mcp/okx` (~$0.1 every `tools/call` when the OKX gate
is active). Coding agents and the Claude plugin must use **`/mcp`**, not `/mcp/okx`.

| Client | Setup |
|---|---|
| **Claude Code** | `claude mcp add --transport http onchainai https://www.onchain-ai.xyz/mcp` |
| **Claude Desktop / Web** | Settings → Connectors → Add custom connector → paste the URL above |
| **Cursor / VS Code** | One-click deeplinks on the [connect hub](https://www.onchain-ai.xyz/connect), or paste the JSON below |
| **ChatGPT** | Settings → Connectors (Developer mode) → new connector with the URL above |
| **Anything else** | `npx add-mcp https://www.onchain-ai.xyz/mcp`, or `npx mcp-remote https://www.onchain-ai.xyz/mcp` for stdio-only clients |

```json
{
  "mcpServers": {
    "onchainai": { "type": "http", "url": "https://www.onchain-ai.xyz/mcp" }
  }
}
```

Full per-client walkthroughs (Codex, Windsurf, Gemini CLI, …): [docs/CONNECT.md](docs/CONNECT.md) or the live [/connect](https://www.onchain-ai.xyz/connect) page.

### MCP tools (public `POST /mcp`)

| Tool | Billing | What it does |
|---|---|---|
| `search_tools` | Free | Search by capability ("bridge USDC to Base"), filter by category/chain, sort by relevance/trust/stars/recent |
| `get_tool_detail` | Free | Full metadata for one tool: trust score, install risk, chains, repo, x402 pricing |
| `get_install_guide` | Free | Platform-specific install steps (claude / cursor / generic / cli) with safety gating — `critical`-risk commands are withheld |
| `list_categories` | Free | Browse the taxonomy with tool counts |
| `get_dashboard_snapshot` | Free | Public coverage snapshot: totals, categories, trust, x402, featured |
| `compare_tools` | Free | Side-by-side comparison of 2–4 tools on trust, risk, chains, pricing |
| `get_price_history` / `get_x402_trends` | Free | Probe history and catalog x402 trends (metadata) |
| `export_toolkit` | **$0.01 USDC** | Export a JSON + markdown install kit by slugs or category |
| `recommend_verified_tool` | **$0.01 USDC** | Pick one verified/live tool for an intent with rejection reasons |
| `gap_audit` | **$0.01 USDC** | Decompose an intent and report catalog coverage gaps |
| `check_endpoint_health` | ~**$0.001 USDC** | Live endpoint probe + 30-day uptime for a listed x402 tool (HTTP 402 handshake) |

Linking your account from a coding agent (`/connect#agent-sync`) unlocks three more
(account link ≠ payment): `save_to_toolkit`, `save_stack_to_blueprint`, and `link_status`.

Full hybrid table (incl. `/mcp/okx`): [docs/CONNECT.md](docs/CONNECT.md).

### Claude Code plugin

One command wires up the MCP server, a `/find-tool` command, and a crypto-tools skill:

```
/plugin marketplace add Coinyak/onchainai
/plugin install onchainai@onchainai
```

The bundle lives in [`plugin/onchainai/`](plugin/onchainai/) — it connects only
OnchainAI's own read-only endpoint, never auto-runs install commands, and always
discloses x402 pricing before recommending a paid tool.

### Agent skill

[`plugin/onchainai/skills/onchainai-crypto-tools/SKILL.md`](plugin/onchainai/skills/onchainai-crypto-tools/SKILL.md)
teaches an agent *when* to search the directory, how to rank trust signals, and the
hard safety rules (never install `critical`-risk tools, always surface x402 costs).
It ships with the plugin; you can also copy the skill directory into
`~/.claude/skills/` or upload it to any agent runtime that supports Agent Skills.

## Features

- **Auto-discovery crawler** — CryptoSkill, GitHub topics, npm, web3-mcp-hub, MCP registry, on cron schedules with dedup + relevance scoring
- **3-axis classification** — Function × Asset Class × Actor, plus chain tagging
- **Trust & safety pipeline** — trust scores, identity-cluster checks (repo/npm/homepage), install-risk analysis with a hard block on `critical` commands, operator review queue, quarantine
- **x402 awareness** — paid tools carry price metadata, verification flags, and referral/attribution disclosure ([policy below](#x402--referral-policy))
- **MCP server for agents** — the read-only tools above, rate-limited and sanitized
- **Claude Code plugin + skill** — one-command onboarding for agent users
- **Auth** — GitHub OAuth (primary); email magic link where configured
- **Community layer** — submissions, comments, upvotes, bookmarks, toolkit, compare, blueprints
- **Admin dashboard** — tool review, categories, crawler control, featured carousel, users, site settings

## x402 & referral policy

OnchainAI is a **tool directory** MCP (discovery/metadata), not a custody wallet:

- **Third-party x402** in the catalog is attribution and trust metadata only — we publish price/endpoint flags (`payment_verified`, `x402_endpoint_verified`, `price_verified`) and never proxy those payments.
- We record anonymous referral/attribution events (views, install-guide fetches) to support revenue-share agreements with tool owners.
- We **never** hold user funds, act as a third-party payment gateway, or invent undocumented `referrer`/`split` payment fields.
- **OnchainAI-owned** premium MCP tools (`export_toolkit`, `recommend_verified_tool`, `gap_audit`, `check_endpoint_health`) may settle x402 to **our** payee wallet when called — that is selling our own service, not custodying others.
- Unverified third-party x402 tools remain visible when they pass the normal public quality gate — verification is a badge, not a hiding mechanism.

Details: [docs/X402_REFERRAL_SPEC.md](docs/X402_REFERRAL_SPEC.md), hybrid connect: [docs/CONNECT.md](docs/CONNECT.md).

## Architecture

```
┌─ Vercel ──────────────┐      ┌─ Railway ─────────────────────────┐
│ Next.js frontend      │──────│ Rust binary (Axum)                │
│ /connect, /tools, ... │ /api │  ├─ REST API (/api/v2)            │
│ proxies /api /auth    │ /mcp │  ├─ MCP server (POST /mcp)        │
│ /mcp to the API       │      │  ├─ Auth (GitHub OAuth + JWT)     │
└───────────────────────┘      │  └─ Crawler (tokio-cron)          │
                               └────────────┬──────────────────────┘
                                            │ sqlx
                               ┌─ Supabase Postgres (RLS) ─┐
                               └───────────────────────────┘
```

- **API/MCP**: Rust — Axum + sqlx + tokio-cron-scheduler, one binary (`cargo run --features ssr`)
- **Frontend**: Next.js (App Router) on Vercel, rewrites `/api`, `/auth`, `/mcp` to the Railway API
- **DB**: Supabase Postgres with RLS; the public read gate is enforced in both SQL policies and server queries

## Run it yourself

```bash
git clone https://github.com/Coinyak/onchainai
cd onchainai
cp .env.example .env        # fill in Supabase URL/keys, GitHub OAuth, JWT secret

# API (port 3000)
cargo run --features ssr

# Frontend (port 3001, proxying to the API)
cd frontend && npm ci && API_PROXY_TARGET=http://localhost:3000 npm run dev -- --port 3001
```

> The crate's default feature set is empty so server deps don't leak into any
> WASM/client build. Plain `cargo` commands that touch server code need
> `--features ssr`.

Common checks:

```bash
cargo test --features ssr                      # tests
cargo clippy --features ssr -- -W clippy::all  # lint
cargo fmt --check                              # format
./scripts/agent-harness-check.sh               # agent/dev harness self-check
```

## Repository layout

| Path | What lives there |
|---|---|
| `src/` | Rust API: MCP server, REST `/api/v2`, auth, crawler, trust/risk pipeline |
| `frontend/` | Next.js app (Vercel): public UI + admin dashboard |
| `plugin/onchainai/` | Claude Code plugin bundle (MCP config, `/find-tool`, skill) |
| `migrations/` | sqlx Postgres migrations (RLS policies included) |
| `docs/` | Design docs, specs, operator guide ([index](docs/INDEX.md)) |
| `scripts/` | Dev/deploy/verification gates (see `AGENTS.md`) |
| `agent/`, `.agents/` | Skills and harness config for AI coding agents working on this repo |

## Documentation

| File | Purpose |
|---|---|
| [docs/CONNECT.md](docs/CONNECT.md) | Connect any MCP client, install the plugin/skill |
| [docs/MVP_DESIGN.md](docs/MVP_DESIGN.md) | Architecture, DB schema, crawler, MCP server |
| [docs/X402_REFERRAL_SPEC.md](docs/X402_REFERRAL_SPEC.md) | x402 referral/attribution design (no-custody) |
| [docs/LAUNCH_READINESS_SPEC.md](docs/LAUNCH_READINESS_SPEC.md) | Public-launch checklist and roadmap spec |
| [docs/SECURITY.md](docs/SECURITY.md) | Security design: auth, RLS, headers, rate limiting |
| [docs/UI_UX_DESIGN.md](docs/UI_UX_DESIGN.md) | Full UI spec |
| [docs/OPERATOR_GUIDE.md](docs/OPERATOR_GUIDE.md) | Operator/admin playbook (Korean) |
| [docs/BUILD_DEPLOY_RULES.md](docs/BUILD_DEPLOY_RULES.md) | Build coherence, smoke gates, Railway/Vercel deploy |
| [AGENTS.md](AGENTS.md) | Entry point for AI coding agents |

## Contributing & security

- Contributions welcome — see [CONTRIBUTING.md](CONTRIBUTING.md). The fastest way to help is submitting missing tools at [onchain-ai.xyz/submit](https://www.onchain-ai.xyz/submit).
- Security reports: see [SECURITY.md](SECURITY.md). Please do not open public issues for vulnerabilities.

## License

[MIT](LICENSE)
