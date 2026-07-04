# OnchainAI

Crypto tool directory: discover, normalize, and expose MCP, CLI, SDK, API, x402, RWA, and AI-agent tools.

Rust API/MCP binary (Axum + sqlx + tokio-cron, Railway) + Next.js frontend (`frontend/`, Vercel) + Supabase Postgres.

## Agent Rule

Keep `AGENTS.md` under 70 lines. Do not expand it with procedure details; route to wiki docs and executable scripts.

## 3-Command UI Workflow

| When | Command |
|------|---------|
| Once after clone | `./scripts/install-agent-hooks.sh` |
| While iterating on UI/auth/routing | `./scripts/dev-watch.sh` |
| Before handoff/commit | `./scripts/ui-change-gate.sh` |

`restart-dev.sh`, `verify-bundle.sh`, and `deploy-railway.sh` are gate/deploy internals — agents do not run them as separate UI steps.

Also once after clone (macOS): `./scripts/install-disk-autoclean.sh` schedules an auto-sweep of multi-GB linker snapshots so disk does not silently fill. See `docs/DISK_MAINTENANCE.md`.

## Start

- Run `git status --short` before edits. Do not revert or overwrite unrelated user/agent changes.
- Read the relevant topic docs before changing that surface.
- Prefer small scoped changes; preserve existing links, buttons, auth states, filters, sidebar behavior, and `data-testid`s unless the task explicitly changes them.
- Report verification commands actually run. Do not claim browser/visual QA passed unless it ran.

## Topic Routing

- Agent workflow and gates: `docs/AGENT_HARNESS.md`
- MCP observability (Vercel/Railway): `docs/MCP_AGENT_WORKFLOW.md`
- Multi-agent coordination (5 roles): `docs/MULTI_AGENT_COORDINATION.md`
- UI/design/layout: `DESIGN.md`, `docs/UI_UX_DESIGN.md`, `.agents/skills/onchainai-ui-workflow/SKILL.md`
- Build/deploy (API + Next.js): `docs/BUILD_DEPLOY_RULES.md`; auto-deploy rules: `docs/superpowers/specs/2026-07-05-split-deploy-automation-spec.md`
- Security/auth/RLS/secrets: `docs/SECURITY.md`
- Architecture/schema/crawler/MCP: `docs/MVP_DESIGN.md`
- Disk/build cleanup: `docs/DISK_MAINTENANCE.md`
- x402 정본: `docs/X402_MONETIZATION_SPEC.md` (+ open listing `docs/X402_OPEN_LISTING_SPEC.md`, 로드맵 `docs/X402_ROADMAP.md`); referrals: `docs/X402_REFERRAL_SPEC.md`; free tier: `docs/superpowers/specs/2026-07-04-free-tier-guardian-spec.md`
- Public launch, plugin bundle (`plugin/onchainai/`), user connect surface: `docs/LAUNCH_READINESS_SPEC.md`, `docs/CONNECT.md`
- Operator/admin behavior: `docs/OPERATOR_GUIDE.md`
- Verified/official status requests: run `node scripts/verify-tool-official.mjs <slug> --apply` (rules: `docs/OPERATOR_GUIDE.md` §4) — never hand-set `tools.status`
- Promote/take-down highlight carousel cards (+ image sourcing): `docs/FEATURED_CARDS.md`

## Essential Commands

- Non-UI compile check: `cargo check --features ssr`
- Full tests: `cargo test --features ssr`
- Lint/format: `cargo clippy --features ssr -- -W clippy::all` and `cargo fmt --check`
- Agent harness check: `./scripts/agent-harness-check.sh`
- Agent readiness report: `./scripts/agent-readiness-report.sh`
- Release build: `./scripts/release-build.sh`

## Hard Rules

- UI/auth/routing: iterate `./scripts/dev-watch.sh` (Next.js + API), finish `./scripts/ui-change-gate.sh`. `cargo check --features ssr` is for API-only compile checks. Pre-commit `ui-staleness-check.sh` blocks stale Next.js bundles.
- Never commit `.env`, `target/`, `.playwright-cli/`, `frontend/.next/`, or build artifacts.
- Never expose `SUPABASE_SERVICE_KEY` or `JWT_SECRET` to client code.
- Validate user input; use sqlx parameterized queries; do not inject raw HTML.
- Third-party x402 in the catalog is metadata/attribution only. Selling OnchainAI-owned services via x402 — K2 `check_endpoint_health` + operator-toggled MCP premium (`compare_tools`/`export_toolkit`) — is allowed when the payee is our wallet; discovery (`search_tools`, `get_tool_detail`, …) stays free. Never custody, third-party payment proxy/gateway, fund-moving for others, or undocumented `referrer`/`split` fields.
- Auth is required for comments, upvotes, bookmarks, and admin routes; admin checks must be server-side.
- After schema changes, run migrations and `sqlx prepare`.
- Before commits/PRs, run relevant tests plus clippy/format, or state exactly why not.
- Never auto-trigger CI or review bots. CI is `workflow_dispatch`-only; CodeRabbit/qodo are manual (`.coderabbit.yaml`, `.pr_agent.toml`) — run them only when the user asks for a specific PR/diff, never proactively. Pushing can wake them, so use `[skip ci]` when a push should run nothing.

## Review Mode

When asked for review, lead with bugs, regressions, missing tests, security, or data-loss risks. Use concrete file/line references.