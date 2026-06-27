# OnchainAI

Crypto tool directory — auto-discover, normalize, and expose fragmented crypto tools (MCP, CLI, SDK, API, x402, RWA, AI agents) for humans and agents.

Rust single binary: Leptos SSR + Axum + rmcp + sqlx + tokio-cron-scheduler.

## Commands

> Note: the crate default feature set is empty (so the `ssr` server deps never
> leak into the wasm build). Plain `cargo` commands that touch server code need
> `--features ssr`. `cargo leptos` reads `bin-features`/`lib-features` and is
> unaffected.

- `cargo build --features ssr`: Compile server (debug)
- `cargo build --release --features ssr`: Production server build
- `cargo run --features ssr`: Start server (port 3000) + crawler scheduler
- `cargo leptos build --release`: Full build (SSR binary + WASM client bundle)
- `cargo test --features ssr`: Run all tests
- `cargo test --features ssr -- --nocapture`: Tests with stdout
- `cargo clippy --features ssr -- -W clippy::all`: Lint (must pass before commit)
- `cargo fmt --check`: Format check
- `sqlx migrate run`: Apply DB migrations (needs DATABASE_URL)
- `sqlx prepare`: Generate sqlx query cache (after schema changes)
- `docker build -t onchainai .`: Build Docker image
- `docker run -p 3000:3000 onchainai`: Run container
- `./scripts/disk-guard.sh`: Check free disk and `target/` size before heavy builds
- `./scripts/clean-build-artifacts.sh --dry-run`: Preview safe cleanup (`cargo clean`, `/tmp` linker snapshots, old Playwright artifacts)
- `./scripts/smoke-test.sh http://localhost:3000`: Curl smoke (pages + MCP initialize)
- `node scripts/browser-smoke.mjs http://localhost:3000`: Playwright smoke (requires `playwright` npm package)
- `./scripts/release-build.sh`: Release build with disk guard + rustup `PATH` for wasm
- `./scripts/verify-bundle.sh`: Check binary + `target/site/pkg/` + CSS are from one build (mtime spread)
- `./scripts/restart-dev.sh`: Kill :3000 → release build → verify bundle → restart → smoke test
- `./scripts/migrate-direct.sh`: Apply migrations (falls back if direct host unavailable; server also migrates on startup)
- `./scripts/deploy-railway.sh`: Sync Railway env vars, deploy Dockerfile, production smoke
- `./scripts/post-deploy-verify.sh https://www.onchain-ai.xyz`: Post-deploy curl + optional browser smoke

## Deploy runbook (operator hardening)

> **Mandatory reading:** [`docs/BUILD_DEPLOY_RULES.md`](docs/BUILD_DEPLOY_RULES.md) — golden rule (one `cargo leptos build --release` for binary + WASM/pkg + CSS), bundle-mismatch symptoms, 2026-06-27 incident, browser cache, macOS linker note.

1. **Disk:** `./scripts/disk-guard.sh` (or `ONCHAINAI_DISK_GUARD_FORCE=1` if tight)
2. **Local verify:** `cargo test --features ssr` → `./scripts/release-build.sh` → `./scripts/verify-bundle.sh`
3. **Restart (mandatory after build):** `./scripts/restart-dev.sh` (or kill :3000 and run `./target/release/onchainai`). Never leave an old process serving stale SSR against new `target/site/pkg/`.
4. **Smoke:** `./scripts/smoke-test.sh http://localhost:3000` (and optional `node scripts/browser-smoke.mjs`)
5. **DB:** Migrations run automatically on server startup (`run_migrations` in `lib.rs`). If local `sqlx migrate run` hits Supabase session pool limits, deploy still applies pending migrations on boot. Optional: Supabase SQL editor for `006`/`007`/`008`.
6. **Railway:** `./scripts/deploy-railway.sh` (requires `railway login`, `.env` secrets). Docker build runs on Railway (local Docker optional).
7. **Post-deploy:** `./scripts/post-deploy-verify.sh` — hard refresh browser if UI looks stale (`docs/BUILD_DEPLOY_RULES.md` §6)
8. **Pool sizing:** `DATABASE_MAX_CONNECTIONS` defaults to `10` (deploy script). `ToolsBrowser` uses one bundled `LoadBrowserData` RPC per navigation. Rate limits are in-process; use a single Railway replica or add shared store before scaling out.

## Architecture

- `/src/main.rs` — Entry point: Axum server + crawler scheduler (single binary)
- `/src/app.rs` — Leptos router (SSR)
- `/src/pages/` — Page components (home, tools_list, tool_detail, admin/)
- `/src/components/` — UI components (search_bar, tool_card, bottom_sheet, login_modal)
- `/src/server/functions.rs` — Leptos server functions (DB queries)
- `/src/server/mcp.rs` — MCP server (rmcp handler, 4 tools)
- `/src/crawler/` — Auto-discovery crawler (4 sources, tokio-cron-scheduler)
- `/src/crawler/sources/` — Source crawlers (cryptoskill, web3mcp, github, npm)
- `/src/auth/` — 3-way auth (github.rs, email.rs, siwx.rs)
- `/src/models/` — Structs (tool.rs, user.rs, comment.rs, category.rs)
- `/migrations/` — SQL migrations (001_init, 002_auth, 003_social)
- `/style/` — Tailwind CSS
- `/docs/` — Design docs (read these before implementing features)

## Design Docs

Read before working on a feature:
- `docs/MVP_DESIGN.md` — Architecture, DB schema, crawler, MCP server, build order
- `docs/UI_UX_DESIGN.md` — Full UI spec (pages, components, mobile, admin panel)
- `DESIGN.md` — Design tokens (Stitch spec, colors, typography, components)
- `docs/SECURITY.md` — Security rules (auth, RLS, headers, rate limiting)
- `docs/BUILD_DEPLOY_RULES.md` — SSR/WASM bundle coherence, local restart workflow, deploy checklist
- `docs/DISK_MAINTENANCE.md` — Disk audit log, hidden `var/folders` junk, monthly cleanup script

## Code Style

- Rust idioms: `?` operator, no unwrap() in production (use anyhow/thiserror)
- sqlx parameterized queries only (`query_as!` macro, `$1` binding). No string interpolation.
- Leptos: server functions for all DB access, signals for client state
- No emojis in UI text. Lucide SVG icons only.
- All UI text in English (global audience)
- Comments: minimal, only for non-obvious logic

## Rules

- Never commit `.env`. Use `.env.example` for template.
- Never expose `SUPABASE_SERVICE_KEY` or `JWT_SECRET` to client.
- All user input must be validated (validator crate).
- All HTML output is escaped by Leptos (never use raw HTML injection).
- Comments/upvotes/bookmarks require auth. Tool browsing is public.
- Admin routes (`/admin/*`) require `is_admin = true` (server-side check).
- Run `cargo clippy` and `cargo fmt --check` before committing.
- Add/update tests for changed code, even if not asked.

## Disk hygiene (ALL coding agents — Claude, Codex, Copilot, Grok, Cursor)

Rust debug builds bloat `target/` fast. This is expected, not a bug. The repo is configured to control it — follow the routine so disk never fills mid-session, with zero human babysitting:

- **Already automatic (no action):** `Cargo.toml` sets `[profile.dev] debug = "line-tables-only"` and strips dependency debug info, so `target/debug` grows far slower. Do not revert these.
- **Before any heavy/release build, run the guard first:** `./scripts/disk-guard.sh`. It auto-cleans incremental caches when free disk `< 25GB` or `target/ > 35GB`, then re-checks. Thresholds: `ONCHAINAI_MIN_FREE_GB`, `ONCHAINAI_MAX_TARGET_GB`. Override stop with `ONCHAINAI_DISK_GUARD_FORCE=1`; disable auto-clean with `ONCHAINAI_DISK_GUARD_AUTOCLEAN=0`.
- **Between work sessions / after a batch of builds:** `./scripts/clean-build-artifacts.sh --incremental-only` (fast — drops only `incremental/` caches, keeps compiled deps so the next build stays fast).
- **Only when disk is tight:** `./scripts/clean-build-artifacts.sh` (full `cargo clean` + `/tmp` linker snapshots) or `cargo clean`. Reserve this — the first build after is a slow full recompile.
- **Never** commit `target/`, `.playwright-cli/`, or any build artifact (already in `.gitignore`).
- **macOS only:** linker failures dump multi-GB `/tmp/onchainai*.ld-snapshot`; the clean script removes them. See `docs/DISK_MAINTENANCE.md`.

## Testing

- `cargo test`: Unit + integration tests
- DB tests use test database (SUPABASE_URL_TEST env var)
- RLS policies tested with pgTap (see SECURITY.md section 4.4)
- Crawler tests mock HTTP responses (wiremock crate)

## DB

- Supabase Postgres. Migrations in `/migrations/`.
- RLS enabled on ALL tables. See SECURITY.md for policies.
- Three migrations: 001_init (tools/sources), 002_auth (profiles/siwx_sessions), 003_social (comments/upvotes/bookmarks).
- After schema changes: `sqlx migrate run` then `sqlx prepare`.

## Git

- Branch: `feat/<name>`, `fix/<name>`, `docs/<name>`
- Commit: conventional commits — `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`
- Squash merge only.
- Run `cargo test` + `cargo clippy` before PR.
