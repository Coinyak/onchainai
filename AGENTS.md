# OnchainAI

Crypto tool directory: discover, normalize, and expose MCP, CLI, SDK, API, x402, RWA, and AI-agent tools.

Rust single binary: Leptos SSR + Axum + rmcp + sqlx + tokio-cron-scheduler.

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
- UI/design/layout: `DESIGN.md`, `docs/UI_UX_DESIGN.md`, `.agents/skills/onchainai-ui-workflow/SKILL.md`
- Build/deploy/SSR-WASM coherence: `docs/BUILD_DEPLOY_RULES.md`
- Security/auth/RLS/secrets: `docs/SECURITY.md`
- Architecture/schema/crawler/MCP: `docs/MVP_DESIGN.md`
- Disk/build cleanup: `docs/DISK_MAINTENANCE.md`
- x402/referrals/trust signals: `docs/X402_REFERRAL_SPEC.md`
- Operator/admin behavior: `docs/OPERATOR_GUIDE.md`

## Essential Commands

- Non-UI compile check: `cargo check --features ssr`
- Full tests: `cargo test --features ssr`
- Lint/format: `cargo clippy --features ssr -- -W clippy::all` and `cargo fmt --check`
- Agent harness check: `./scripts/agent-harness-check.sh`
- Agent readiness report: `./scripts/agent-readiness-report.sh`
- Release build: `./scripts/release-build.sh`

## Hard Rules

- UI/auth/routing work must not finish with `cargo build --features ssr`, `cargo build --release --features ssr`, or `cargo run --features ssr`; iterate with `dev-watch.sh`, finish with the UI gate. Git pre-commit + `ui-staleness-check.sh` block stale UI (any tool); IDE stop hooks are optional extras.
- Never commit `.env`, `target/`, `.playwright-cli/`, stray WASM, or build artifacts.
- Never expose `SUPABASE_SERVICE_KEY` or `JWT_SECRET` to client code.
- Validate user input; use sqlx parameterized queries; do not inject raw HTML.
- x402 is attribution/trust metadata only: do not build custody, facilitator, gateway, fund-moving, undocumented `referrer`, or `split` payment fields.
- Auth is required for comments, upvotes, bookmarks, and admin routes; admin checks must be server-side.
- After schema changes, run migrations and `sqlx prepare`.
- Before commits/PRs, run relevant tests plus clippy/format, or state exactly why not.

## Review Mode

When asked for review, lead with bugs, regressions, missing tests, security, or data-loss risks. Use concrete file/line references.