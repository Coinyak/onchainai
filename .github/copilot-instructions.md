# Copilot instructions — OnchainAI

**Primary source of truth: [`AGENTS.md`](../AGENTS.md).** Read it first. It defines commands, architecture, code style, and rules. This file mirrors the essentials for GitHub Copilot.

## Project

Crypto tool directory. Rust single binary: Leptos SSR + Axum + rmcp (MCP) + sqlx + tokio-cron-scheduler.

## Build / test (server features required)

The crate default feature set is empty. Plain `cargo` commands that touch server code need `--features ssr`:

- `cargo build --features ssr` — debug build
- `cargo test --features ssr` — tests
- `cargo clippy --features ssr -- -W clippy::all` — lint (must pass before commit)
- `cargo fmt --check` — format check
- `cargo leptos build --release` — full build (SSR binary + WASM + CSS)

## Disk hygiene (IMPORTANT — keep builds from filling the disk)

Rust debug builds bloat `target/` fast. Expected, not a bug. Repo is configured to control it — follow this so disk never fills mid-session:

- **Automatic, do not revert:** `Cargo.toml` `[profile.dev]` uses `debug = "line-tables-only"` and strips dependency debug info → slower `target/debug` growth.
- **Before any heavy/release build:** run `./scripts/disk-guard.sh`. It auto-cleans incremental caches when free disk `< 25GB` or `target/ > 35GB`, then re-checks.
- **Between sessions / after many builds:** `./scripts/clean-build-artifacts.sh --incremental-only` (fast; keeps compiled deps).
- **Only when disk is tight:** `./scripts/clean-build-artifacts.sh` or `cargo clean` (next build is a slow full recompile).
- **Never** commit `target/`, `.playwright-cli/`, or build artifacts.

## Rules (see AGENTS.md for full list)

- Never commit `.env`; never expose `SUPABASE_SERVICE_KEY` / `JWT_SECRET` to client.
- sqlx parameterized queries only (`query_as!`, `$1` binding). No string interpolation.
- All DB access via Leptos server functions. All user input validated.
- No `unwrap()` in production — use `?` / anyhow / thiserror.
- No emojis in UI text. Lucide SVG icons only. UI text in English.
- Admin routes (`/admin/*`) require server-side `is_admin = true`.
- Run `cargo clippy` + `cargo fmt --check` before committing. Add/update tests for changed code.
- Conventional commits. Default branch `main` (Railway production deploys from `main`). Feature branches `feat/`, `fix/`, `docs/`. Squash merge only.
