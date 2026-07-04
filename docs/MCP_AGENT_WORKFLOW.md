# MCP Agent Workflow

> Related: [[AGENT_HARNESS]] | [[MULTI_AGENT_COORDINATION]] | [[BUILD_DEPLOY_RULES]] | [[../AGENTS.md]]

Executable gates (`./scripts/*`) are **proof**. MCP (Vercel, Railway, GitHub, onchainai) is **observability** — read logs and status first; never substitute MCP for gate scripts.

## MCP config files (repo)

| File | Consumer | Transport |
|------|----------|-----------|
| `.mcp.json` | MCP standard / plugin bundles | `vercel` URL; `railway` + `onchainai` stdio |
| `.cursor/mcp.json` | Cursor IDE | same canonical servers |
| `.grok/config.toml` | Grok CLI (project-scoped) | `vercel` URL; `railway` + `onchainai` command/args |

Canonical servers (must match across all three):

- `vercel` → `https://mcp.vercel.com/onchain-ai/onchainai`
- `railway` → `railway mcp` (CLI on PATH)
- `onchainai` → `npx mcp-remote https://www.onchain-ai.xyz/mcp`

Parity gate: `./scripts/check-mcp-config-parity.sh` (also run from `agent-harness-check.sh`).

### Prerequisites

- **Railway CLI** — `railway` on `PATH`; run `railway login` once per machine before `railway` MCP works.
- **OAuth is per-machine** — Vercel/Railway/GitHub MCP tokens live in local credential stores (e.g. `~/.grok/mcp_credentials.json`). Re-auth on each device; never copy tokens into the repo or CI.

## Stack split

| Surface | Host | MCP | Agent edits |
|---------|------|-----|-------------|
| Next.js UI | Vercel (`www.onchain-ai.xyz`) | `vercel` | `frontend/` |
| Rust API, auth, crawler, product MCP | Railway | `railway` | `src/` (API), `Dockerfile.api` |
| Code & PRs | GitHub | `github` | repo-wide (one writer per path) |
| Live catalog / trust | Production | `onchainai` | read-only queries |

Vercel rewrites `/api`, `/auth`, `/onboarding`, `/mcp` to Railway. Classify failures at the proxy boundary before fixing.

## MCP routing (decision tree)

1. **Product data** (tool count, categories, install guide) → `onchainai` MCP
2. **Frontend build, preview/prod deploy, Next runtime** → `vercel` MCP (team `onchain-ai`, project `onchainai`)
3. **API 5xx, container crash, Dockerfile deploy, env on Railway** → `railway` MCP
4. **PR status, diff, review** (only when user names a PR) → GitHub MCP
5. **Local edits, compile, gates** → shell scripts; no MCP

## Deploy/Ops rules

1. **MCP is read-first** — deployment status, build/runtime logs, env *names* (never values), domain/rewrite health.
2. **Scripts perform deploys** — `./scripts/deploy-railway.sh`, `./scripts/post-deploy-verify.sh https://www.onchain-ai.xyz`. Do not redeploy via MCP unless the user explicitly asks in the same session.
3. **Pre-deploy checklist** (user-requested Railway deploy): `disk-guard.sh` → tests → `release-build.sh` → `verify-bundle.sh` → deploy script → `post-deploy-verify.sh`.
4. **Production cuts from `main`** — Railway/Vercel auto-deploy on `main` push; `deploy-railway.sh` **refuses** non-`main` unless `--force-non-main`. Spec: `docs/superpowers/specs/2026-07-05-split-deploy-automation-spec.md`.
5. **Report evidence** — list MCP queries (read-only), scripts run, and PASS/FAIL markers. Do not claim prod healthy without smoke/post-deploy output.

## Security rules

1. **Never commit or echo secrets** — `SUPABASE_SERVICE_KEY`, `JWT_SECRET`, OAuth tokens, full env dumps. Use `[REDACTED]` and var names only.
2. **`~/.grok/mcp_credentials.json` is machine-local** — re-auth OAuth per device; never copy into repo or CI.
3. **Human confirmation before destructive MCP** — domain purchase, deployment delete/rollback, prod env writes, DNS changes, DB resets.
4. **Redact log excerpts** — summarize errors with line hints; do not paste raw blobs that may contain tokens.
5. **x402 stays metadata-only** — no custody or fund-moving automation via MCP.

## Frontend + Vercel

1. **UI lives in `frontend/`** — not legacy `src/pages` / `src/components` (Leptos reference only).
2. **Local loop** — `cd frontend && npm run dev`; handoff: `npm run lint && npm run build`.
3. **After meaningful UI change** — use Vercel MCP to confirm latest preview/production deploy (build OK, no runtime errors on changed routes).
4. **Preserve `data-testid`** — update smoke scripts if selectors change.

## Verification tiers (with MCP)

| Change type | MCP (observe) | Scripts (prove) |
|-------------|---------------|-----------------|
| UI-only | Vercel deploy/logs after local `npm run build` | `ui-change-gate.sh` full tier |
| API-only | Railway runtime logs on failure | `cargo test`, clippy, fmt |
| Full-stack | Vercel + Railway as needed | matrix in [[MULTI_AGENT_COORDINATION]] |
| Deploy | Both surfaces post-deploy | `deploy-railway.sh` + `post-deploy-verify.sh` |

## Anti-patterns

- Redeploying via MCP on first 500 without logs, checklist, or user request.
- Dumping full Railway/Vercel env lists into chat for “debugging”.
- Fixing UI in Leptos while production serves Next.js on Vercel.