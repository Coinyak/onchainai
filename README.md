# OnchainAI

> Crypto tools, unified. Discover, install, and share MCP, CLI, SDK, API, x402, RWA, and AI agent tools — all in one place.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

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
git clone https://github.com/love/onchainai
cd onchainai
cp .env.example .env
# Fill in .env (Supabase URL, keys, GitHub OAuth, JWT secret)
docker build -t onchainai .
docker run -p 3000:3000 onchainai
```

Or run directly:

```bash
cargo run
# Server at http://localhost:3000
```

## Design Docs

| File | Purpose |
|---|---|
| `docs/MVP_DESIGN.md` | Architecture, DB schema, crawler, MCP server, build order |
| `docs/UI_UX_DESIGN.md` | Full UI spec (pages, components, mobile, admin panel) |
| `DESIGN.md` | Design tokens (Stitch spec for AI UI generation) |
| `docs/SECURITY.md` | Security rules (auth, RLS, headers, rate limiting) |
| `AGENTS.md` | Instructions for AI coding agents |

## License

MIT
