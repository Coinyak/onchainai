# Agent Harness

> Related docs: [[../AGENTS.md]] | [[AGENT_READINESS_REPORT]] | [[BUILD_DEPLOY_RULES]] | [[UI_UX_DESIGN]] | [[../DESIGN]]

This page is the wiki-style operating contract for coding agents. Keep `AGENTS.md` short; put reusable agent procedure here and enforce as much as possible with scripts.

## 3-Command UI Workflow

| When | Command | Purpose |
|------|---------|---------|
| Once after clone | `./scripts/install-agent-hooks.sh` | Git pre-commit blocks stale UI commits (any tool) |
| While iterating | `./scripts/dev-watch.sh` | Coherent SSR+WASM rebuild + live reload |
| Before handoff/commit | `./scripts/ui-change-gate.sh` | Release build, restart, smoke, screenshots |

`restart-dev.sh`, `verify-bundle.sh`, and deploy scripts run inside the gate or operator deploy flow — not as separate agent UI steps.

## Principle

Agents should rely on executable gates, not memory. The repo's common failure mode is a Leptos SSR/WASM/CSS mismatch: code changes are present, but localhost serves an old binary or the browser hydrates with old WASM. That makes UI changes look missing and can make existing buttons, sidebar controls, filters, or auth UI stop working.

## Local Trouble Doctor

When a user says localhost is stale, buttons do not work, or the server seems off, run:

```bash
./scripts/local-doctor.sh
```

The doctor is read-only: it checks the listener, stale `target/dev-server.pid`, bundle coherence, local cache headers for `/` and `/pkg/*`, and `localhost` vs `127.0.0.1` guidance. It prints the next command instead of killing or starting processes. Use it before asking the user to clear browser cache; hard refresh is only for an already-open tab that still holds an old in-memory bundle after the server is healthy.

## Note-Taking Rule

Future agent notes must follow the same LLM-wiki pattern:

- Keep `AGENTS.md` under 70 lines as a routing entry point.
- Do not append long procedures, incident writeups, or tool-specific checklists to `AGENTS.md`.
- Put detailed notes in the relevant topic doc, then add or update one short route link only if discoverability is missing.
- Prefer executable scripts and gates over prose-only reminders when the note describes repeatable verification.
- Run `./scripts/agent-harness-check.sh` after changing agent instructions, docs routing, or gate scripts.
- Run `./scripts/agent-readiness-report.sh` when preparing the repo for broader multi-agent work.

## Before Editing

- Run `git status --short`.
- Inspect existing diffs before touching a dirty file.
- Keep the scope to the requested route, component, server function, or script.
- Preserve existing navigation, auth controls, filters, sidebar collapse/expand, load more, modal behavior, and `data-testid` selectors unless the task explicitly changes them.

## UI/Auth/Routing Gate

Run this before final handoff if the change touches `src/pages`, `src/components`, `style`, `src/app.rs`, auth shell/nav code, UI server functions, browser smoke expectations, or route behavior:

```bash
./scripts/ui-change-gate.sh
```

The gate runs:

- `agent-harness-check.sh`
- coherent Leptos release build and restart via `restart-dev.sh`
- `verify-bundle.sh`
- curl smoke
- browser smoke
- local auth smoke when available
- desktop/mobile visual snapshots

Do not use `cargo build --features ssr`, `cargo build --release --features ssr`, or `cargo run --features ssr` as final verification for UI/auth/routing work. Use `cargo check --features ssr` for non-UI compile checks only.

## Universal Enforcement (Any Coding Tool)

Codex, Copilot, Droid, Cursor, Claude Code, Grok, VS Code, or terminal — the same
rules apply. Run once after clone:

```bash
./scripts/install-agent-hooks.sh
```

**Primary (tool-agnostic):** sets `git config core.hooksPath scripts/git-hooks`.
The pre-commit hook runs `ui-staleness-check.sh --staged` and blocks commits when
staged UI sources are newer than the built WASM bundle. No IDE integration required.

**Optional (IDE-only):** committed stop hooks give earlier feedback before commit
when the tool supports them:

- `.cursor/hooks.json` — Cursor / Grok (`stop` + `subagentStop`)
- `.claude/settings.json` — Claude Code; Cursor loads it with **Third-party skills**

Shared checker: `scripts/ui-staleness-check.sh` (exit 2 = stale). Fast UI loop:
`./scripts/dev-watch.sh`. Personal Claude overrides: `.claude/settings.local.json`.

**CI (merge gate):** `.github/workflows/ci.yml` — `rust` + `agent-harness` always;
`ui-coherence` (release build + verify + smoke + browser) when UI paths change.

**Git pre-push:** `scripts/git-hooks/pre-push` runs `ui-staleness-check.sh --worktree`
(catches `--no-verify` commits before push).

**Cursor:** `.cursor/rules/onchainai-agents.mdc` loads the 3-command workflow in Cursor/Grok.

**Optional bootstrap:** `./scripts/agent-start.sh` (hooks + disk guard + reminders).

**Gate tiers:** `./scripts/ui-change-gate.sh --tier smoke` (build + curl only, ~5 min warm);
default `--tier full` adds browser, auth shell, and snapshots. `--harness-only` validates
CLI/options without build (alias: `--check-only`).

If using a non-default local port, prefer:

```bash
./scripts/ui-change-gate.sh --port 3001
```

Use `./scripts/agent-harness-check.sh` for the full harness contract (includes `test-local-doctor.sh`, `test-ui-staleness-check.sh`, and `ui-change-gate.sh --check-only`). Use `./scripts/ui-change-gate.sh --check-only` to validate gate CLI/options only without build/restart/browser work.

`--base` is accepted only for explicit local URLs. If `PORT` or `--port` is already set, the URL port must match it. Otherwise the gate derives the restart port from `--base`, so it cannot restart one server and smoke-test another.

## If The Gate Cannot Run

Report the exact failed or skipped command. Do not claim browser, auth, or visual QA passed unless the command actually ran. If Playwright is missing, say that and continue with the strongest available non-browser checks.

## L5 Governance

Branch protection and review routing (CODEOWNERS, PR template, `ci-success` merge gate):

- `docs/BRANCH_PROTECTION.md` — require **`ci-success`** on `main`
- `./scripts/configure-branch-protection.sh --check-only` — validate `gh` auth and print admin steps

## Multi-Agent + MCP

When work spans `frontend/`, `src/`, and `migrations/`, spawn up to five subagents with exclusive path ownership. Coordinator merges handoffs and runs the full gate.

- Roster, DAG, verification matrix: [[MULTI_AGENT_COORDINATION]]
- Handoff packet template: [[handoff-packet-template]]
- MCP routing (observe logs/deploys; scripts prove deploys): [[MCP_AGENT_WORKFLOW]]
- Skill entry: `.agents/skills/multi-agent-coordination/SKILL.md`

**MCP vs scripts:** Vercel/Railway/GitHub MCP for read-only investigation. Production deploys stay on `./scripts/deploy-railway.sh` and `./scripts/post-deploy-verify.sh` unless the user explicitly requests a deploy in-session.

## Operator Note

After a passing local gate, SSR HTML should revalidate and auth/API responses are not stored. An already-open browser tab can still hold an old in-memory WASM bundle until reload; use hard refresh (`Cmd+Shift+R`) only if that tab still looks stale after reload.
