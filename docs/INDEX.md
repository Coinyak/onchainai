# OnchainAI Documentation Index

> Obsidian-compatible knowledge base. All docs are cross-linked markdown.
> AI agents: read AGENTS.md first, then relevant docs below.

## Architecture

- [[MVP_DESIGN]] — Full architecture: Rust single binary, DB schema, crawler, MCP server, auth, admin panel, build order
- [[SECURITY]] — Security design: 3-way auth, SIWX, RLS policies, web security headers, rate limiting, admin access control

## Design

- [[UI_UX_DESIGN]] — Full UI spec: pages, components, sidebar, preview panel, bottom sheet, mobile, admin dashboard
- [[../DESIGN]] — Stitch DESIGN.md: design tokens (colors, typography, components) for AI UI generation

## Agent Configuration

- [[../AGENTS.md]] — Instructions for all coding agents (30+ tools supported)
- [[../CLAUDE.md]] — Claude Code entry point (imports AGENTS.md)

## External References

- [AGENTS.md spec](https://agents.md/) — Open standard, Linux Foundation
- [Harness Engineering](https://martinfowler.com/articles/harness-engineering.html) — Martin Fowler / Thoughtworks
- [Stitch DESIGN.md](https://stitch.withgoogle.com/docs/design-md/overview) — Google Stitch spec
- [SIWX (x402 V2)](https://docs.x402.org/extensions/sign-in-with-x) — CAIP-122 wallet auth
- [Supabase RLS](https://supabase.com/docs/guides/database/postgres/row-level-security) — Row Level Security
