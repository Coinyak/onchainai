---
name: multi-agent-coordination
description: >
  Spawn and coordinate up to five OnchainAI subagents with exclusive path
  ownership, handoff packets, MCP observability rules, and verification tiers.
  Use when the task spans frontend + backend + schema, or when the user asks
  for parallel subagents, multi-agent work, or MCP-aware deploy/debug workflow.
metadata:
  short-description: "5-agent roster, DAG, handoff, MCP routing"
---

# Multi-Agent Coordination

Read before spawning subagents:

1. `docs/MULTI_AGENT_COORDINATION.md` — roster, DAG, verification matrix
2. `docs/MCP_AGENT_WORKFLOW.md` — Vercel/Railway/GitHub/onchainai routing
3. `docs/handoff-packet-template.md` — required between agents

## Quick roster

| Role | Paths |
|------|--------|
| Backend Core | `src/` (API, crawler, MCP) |
| Frontend Surface | `frontend/` |
| Data & Schema | `migrations/`, `sqlx prepare` |
| Harness & Deploy | `scripts/`, `.github/`, `.grok/`, `.cursor/` |
| Security & Trust | auth, admin, install_safety, x402 |

Coordinator: DAG + merge handoffs + `agent-harness-check.sh` + user summary.

## Spawn checklist

- `git status --short`
- Assign exclusive globs; no two writers on same file
- Serialize seams: schema → API contract packet → frontend
- MCP read-only by default; deploy only via `./scripts/deploy-railway.sh` when user asks
- Handoff packet on every boundary

## MCP (observe, not replace gates)

- **vercel** — Next deploy/logs (`onchain-ai` / `onchainai`)
- **railway** — API logs, env names, deploy status
- **onchainai** — live product queries
- **github** — PR/diff when user names a PR

Never paste secrets from MCP output. Never redeploy via MCP without explicit user approval.