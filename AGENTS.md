# OnchainAI

Crypto tool directory — auto-discover, normalize, and expose fragmented crypto tools (MCP, CLI, SDK, API, x402, RWA, AI agents) for humans and agents.

Rust single binary: Leptos SSR + Axum + rmcp + sqlx + tokio-cron-scheduler.

## Commands

- `cargo build`: Compile (debug)
- `cargo build --release`: Production build
- `cargo run`: Start server (port 3000) + crawler scheduler
- `cargo test`: Run all tests
- `cargo test -- --nocapture`: Tests with stdout
- `cargo clippy -- -W clippy::all`: Lint (must pass before commit)
- `cargo fmt --check`: Format check
- `sqlx migrate run`: Apply DB migrations (needs DATABASE_URL)
- `sqlx prepare`: Generate sqlx query cache (after schema changes)
- `docker build -t onchainai .`: Build Docker image
- `docker run -p 3000:3000 onchainai`: Run container

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
