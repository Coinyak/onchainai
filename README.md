# OnchainAI

> Crypto tools, unified. Discover, install, and share MCP, CLI, SDK, API, x402, RWA, and AI agent tools — all in one place.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-Leptos%20SSR-orange.svg)](https://leptos.dev)
[![Live](https://img.shields.io/badge/Live-onchain--ai.xyz-2ea44f.svg)](https://www.onchain-ai.xyz)

**Live: [onchain-ai.xyz](https://www.onchain-ai.xyz)**

## Why?

Crypto tooling is fragmented across CryptoSkill, Smithery, npm, GitHub topics, and dozens of separate registries. **BOB Gateway CLI was released with AI agent docs — but no directory had it.** OnchainAI fixes this with auto-discovery.

## Features

- Auto-discovery crawler (CryptoSkill, GitHub topics, npm, web3-mcp-hub)
- 3-axis classification (Function x Asset Class x Actor)
- MCP server for agents to search tools programmatically
- x402 payment integration for paid tools
- GitHub star sync (freshness indicator)
- 3-way auth: GitHub OAuth + Email magic link + SIWX wallet (CAIP-122)
- Admin dashboard (tool approval, categories, user management, site settings)

## Stack

Rust single binary: Leptos SSR + Axum + rmcp + sqlx + tokio-cron-scheduler. Deployed on Railway. DB on Supabase Postgres.

## Quick Start

```bash
git clone https://github.com/hoyeon4315-cpu/onchainai
cd onchainai
cp .env.example .env
# Fill in .env (Supabase URL, keys, GitHub OAuth, JWT secret)
./scripts/install-agent-hooks.sh
# Optional session bootstrap: ./scripts/agent-start.sh   # once: Git pre-commit blocks stale UI commits
docker build -t onchainai .
docker run -p 3000:3000 onchainai
```

Or run directly:

```bash
cargo run --features ssr
# Server at http://localhost:3000
```

> Note: the crate's default feature set is empty so the server deps don't leak
> into the WASM client build. Plain `cargo` commands that touch server code need
> `--features ssr`. `cargo leptos build` reads features from `Cargo.toml` and is
> unaffected.

## Design Docs

| File | Purpose |
|---|---|
| `docs/MVP_DESIGN.md` | Architecture, DB schema, crawler, MCP server, build order |
| `docs/UI_UX_DESIGN.md` | Full UI spec (pages, components, mobile, admin panel) |
| `DESIGN.md` | Design tokens (Stitch spec for AI UI generation) |
| `docs/SECURITY.md` | Security rules (auth, RLS, headers, rate limiting) |
| `docs/OPERATOR_GUIDE.md` | Operator/admin capabilities (Korean) — what each role can do without code changes |
| `docs/BUILD_DEPLOY_RULES.md` | Build coherence, smoke gates, Railway Dockerfile deploy |
| `AGENTS.md` | Instructions for AI coding agents |

## License

MIT
