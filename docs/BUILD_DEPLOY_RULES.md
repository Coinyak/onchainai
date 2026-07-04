# Build & Deploy Rules

> **Stack:** Rust API/MCP (`src/`, Railway `Dockerfile.api`) + Next.js UI (`frontend/`, Vercel).
> **Not used:** `cargo-leptos`, Leptos SSR, WASM hydration — removed in favor of split deploy.

AI agents: read [[../AGENTS.md]] first; follow this doc before any local run or deploy.

---

## 1. Golden rule — two artifacts, one gate

**UI changes need a fresh Next.js build. API changes need a fresh Rust binary. Full local gate builds both.**

| Artifact | Path | Build |
|----------|------|--------|
| API server | `target/release/onchainai` | `cargo build --release --features ssr` |
| Next.js UI | `frontend/.next/` | `cd frontend && npm run build` |

**One command for both:**

```bash
./scripts/release-build.sh
```

Verify before smoke:

```bash
./scripts/verify-bundle.sh
```

**Do not mix partial builds for UI handoff:**

- ❌ `cargo build --release --features ssr` alone when `frontend/` changed
- ❌ `npm run build` alone when `src/` API routes changed
- ✅ `./scripts/release-build.sh` → `./scripts/ui-change-gate.sh`

---

## 2. Local dev workflow

> **Agent 3-command model:** `./scripts/install-agent-hooks.sh` → `./scripts/dev-watch.sh` → `./scripts/ui-change-gate.sh`. Details: `docs/AGENT_HARNESS.md`.

### Iterating on UI — dev watch

```bash
./scripts/dev-watch.sh
```

Starts **Rust API on `API_PORT` (default 3001)** and **Next.js on `PORT` (default 3000)**.
Next proxies `/api`, `/auth`, `/mcp` to the API via `API_PROXY_TARGET`.

Browse `http://127.0.0.1:3000`. Edit `frontend/**` for HMR. API edits need restarting the script or a separate `cargo run`.

### Before commit / handoff

```bash
./scripts/ui-change-gate.sh
```

Release build, restart API + `next start`, curl smoke, browser checks, screenshots.

### API-only work (no UI)

```bash
cargo check --features ssr
cargo test --features ssr
```

---

## 3. Production deploy

| Surface | Platform | Build |
|---------|----------|--------|
| API + MCP | Railway | `Dockerfile.api` → `cargo build --release --features ssr` |
| Web UI | Vercel | `frontend/` → `npm run build` |

Vercel sets `API_PROXY_TARGET` to the Railway URL so `/api` and `/mcp` rewrite correctly.

---

## 4. Legacy note (Leptos)

Older docs, skills, and comments may still mention `cargo leptos watch`, `target/site/pkg/`, or WASM coherence. **Ignore those for current work.** If a script still calls `cargo-leptos`, file a fix — the canonical paths are this doc and `AGENTS.md`.

---

## 5. Troubleshooting

| Symptom | Likely cause | Fix |
|---------|----------------|-----|
| UI changes not visible | Stale `frontend/.next` | `cd frontend && npm run build` or `dev-watch.sh` |
| API 401/502 from UI | API not running or wrong `API_PROXY_TARGET` | Check API on `API_PORT`; restart `dev-watch.sh` |
| Gate fails bundle verify | Partial rebuild | `./scripts/release-build.sh` |
| Low disk before release build | Large `target/` | `./scripts/disk-guard.sh` — see `docs/DISK_MAINTENANCE.md` |
| Railway deploy "succeeds" but runtime crashes on stale code (e.g. a migration version mismatch) | `railway up` run from a `git worktree` or any directory other than the one `~/.railway/config.json` has linked — it silently uploads/builds from the *linked* path, not cwd, with no error | Already fixed in `deploy-railway.sh` (`railway up "${ROOT}" --path-as-root`) — never call bare `railway up` from a worktree |

```bash
./scripts/local-doctor.sh
```

Read-only diagnosis for listener, bundle, and cache headers.