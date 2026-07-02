# OnchainAI Documentation Index

> Obsidian-compatible knowledge base. All docs are cross-linked markdown.
> AI agents: read AGENTS.md first, then relevant docs below.

## Architecture

- [[MVP_DESIGN]] — Full architecture: Rust single binary, DB schema, crawler, MCP server, auth, admin panel, build order
- [[SECURITY]] — Security design: 3-way auth, SIWX, RLS policies, web security headers, rate limiting, admin access control

## Design

- [[UI_UX_DESIGN]] — Full UI spec: pages, components, sidebar, preview panel, bottom sheet, mobile, admin dashboard
- [[../DESIGN]] — Stitch DESIGN.md: design tokens (colors, typography, components) for AI UI generation

## Operations & Deploy

- [[OPERATOR_GUIDE]] — Operator/admin capabilities (Korean): what each role can do without code changes
- [[BUILD_DEPLOY_RULES]] — Build coherence, smoke gates, Railway Dockerfile deploy

## Agent Configuration

- [[../AGENTS.md]] — Short routing entry point for all coding agents
- [[AGENT_HARNESS]] — Wiki-style agent workflow and executable UI/auth/routing gates
- [[MCP_AGENT_WORKFLOW]] — Vercel/Railway/GitHub/onchainai MCP routing and deploy observability
- [[MULTI_AGENT_COORDINATION]] — Five-subagent roster, DAG, verification matrix
- [[handoff-packet-template]] — Copy-paste handoff between subagents
- [[AGENT_READINESS_REPORT]] — Environment readiness report for coding agents
- [[../CLAUDE.md]] — Claude Code entry point (imports AGENTS.md)

## External References

- [AGENTS.md spec](https://agents.md/) — Open standard, Linux Foundation
- [Harness Engineering](https://martinfowler.com/articles/harness-engineering.html) — Martin Fowler / Thoughtworks
- [Stitch DESIGN.md](https://stitch.withgoogle.com/docs/design-md/overview) — Google Stitch spec
- [SIWX (x402 V2)](https://docs.x402.org/extensions/sign-in-with-x) — CAIP-122 wallet auth
- [Supabase RLS](https://supabase.com/docs/guides/database/postgres/row-level-security) — Row Level Security
