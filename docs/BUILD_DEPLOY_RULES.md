# Build & Deploy Rules

> **Authoritative work rules** for OnchainAI (Leptos SSR + WASM + server functions).
> Prevents bundle mismatch and "changes not reflecting on the website."

AI agents: read [[../AGENTS.md]] first; follow this doc before any local run or deploy.

---

## 1. Golden rule — one build, one bundle

**The release server binary, WASM/pkg assets, and CSS must all come from a single `cargo leptos build --release` (or `./scripts/release-build.sh`).**

Never run an old `target/release/onchainai` against a freshly rebuilt `target/site/pkg/`. Never serve new SSR markup while the browser hydrates with an older WASM bundle.

| Artifact | Path |
|----------|------|
| Server binary | `target/release/onchainai` |
| Client JS | `target/site/pkg/onchainai.js` |
| Client WASM | `target/site/pkg/onchainai.wasm` (+ symlink `onchainai_bg.wasm`) |
| CSS | `style/output.css` served as `/pkg/onchainai.css` |

**Do not mix partial builds:**

- ❌ `cargo build --release --features ssr` alone (server only; no WASM/pkg refresh)
- ❌ Rebuilding only WASM/pkg while keeping an hours-old binary
- ❌ `cargo run --features ssr` debug server against release `target/site/pkg/`
- ✅ `./scripts/release-build.sh` → restart server → smoke test

Verify coherence before starting the server:

```bash
./scripts/verify-bundle.sh
```

Checks `target/release/onchainai`, `target/site/pkg/*`, and the served `style/output.css` mtimes (default ±180s tolerance; override with `ONCHAINAI_BUNDLE_MAX_SKEW_SEC`).

CSS note: `target/site/pkg/onchainai.css` is a cargo-leptos placeholder and may be empty. The Axum server explicitly serves `/pkg/onchainai.css` from `style/output.css` in `src/lib.rs`. `./scripts/release-build.sh` validates that stylesheet and refreshes the release binary + served CSS mtimes after the full build succeeds. This prevents false bundle failures when cargo reuses an unchanged server binary while regenerating WASM/JS/CSS artifacts, without hiding genuinely stale artifacts from separate build sessions.

Slow-link note: macOS release server linking can legitimately finish more than 60 seconds after the WASM/pkg step. The default mtime tolerance is 180 seconds to avoid false positives while still catching stale artifacts from separate build sessions.

---

## 2. Local dev workflow

> **Agent 3-command model:** once `./scripts/install-agent-hooks.sh`, iterate `./scripts/dev-watch.sh`, finish `./scripts/ui-change-gate.sh`. Details: `docs/AGENT_HARNESS.md`.

### Iterating on UI — use the watch loop (fastest coherent path)

While actively changing UI/Leptos code, run:

```bash
./scripts/dev-watch.sh
```

This wraps `cargo leptos watch`: every save rebuilds the **SSR binary and the
WASM/JS bundle together** and live-reloads the browser on `http://127.0.0.1:3000`
(reload-port 3001). Because both artifacts always come from one build, you never
hit the bundle-mismatch failures in §3. `style/output.css` is hand-authored and
served live (`ServeFile`), so CSS edits just need a browser refresh.

> Do **not** reach for `cargo build --features ssr` to "preview" UI — it rebuilds
> the server only, so the browser hydrates a stale WASM bundle. Use the watch loop.

Universal enforcement: `./scripts/install-agent-hooks.sh` wires Git pre-commit
(`scripts/git-hooks/pre-commit` → `ui-staleness-check.sh --staged`) so **any**
coding tool is blocked from committing stale UI. Optional IDE stop hooks
(`.cursor/hooks.json`, `.claude/settings.json`) give earlier feedback. The watch
loop keeps the bundle fresh; the final gate rebuilds it.

### Before handoff / commit — run the final gate

After **any** change that affects UI, server functions, or routing:

1. **Stop** the old server on port 3000 (stale processes are the #1 cause of "changes not showing").
2. **Build** with one full release build: `./scripts/release-build.sh`
3. **Verify** bundle: `./scripts/verify-bundle.sh`
4. **Restart** the release binary (not a leftover debug `cargo run`).
5. **Smoke-test** before trusting the browser.

One-shot helper (build + restart + smoke):

```bash
./scripts/restart-dev.sh
```

For UI, auth shell, route, or Leptos server-function changes, prefer the stricter
agent gate:

```bash
./scripts/ui-change-gate.sh
```

It runs `agent-harness-check.sh`, `restart-dev.sh`, `verify-bundle.sh`, browser
smoke, local auth smoke when available, and desktop/mobile visual snapshots. If
an agent cannot run this gate, it must report the missing command or failure
explicitly instead of claiming UI QA passed.

Skip rebuild if artifacts are already fresh:

```bash
./scripts/restart-dev.sh --no-build
```

Manual equivalent:

```bash
# Stop anything on :3000
lsof -ti :3000 | xargs kill -9 2>/dev/null || true

./scripts/release-build.sh
./scripts/verify-bundle.sh

# Foreground (logs in terminal)
./target/release/onchainai

# Or background
nohup ./target/release/onchainai > /tmp/onchainai.log 2>&1 &

./scripts/smoke-test.sh http://localhost:3000
# Optional: node scripts/browser-smoke.mjs http://localhost:3000
# UI changes: node scripts/visual-snapshots.mjs http://localhost:3000 --out .playwright-cli/ui-snapshots
```

**Mandatory:** restart after every release build. A running binary does not pick up new `target/site/pkg/` or recompiled server code.

---

## 3. Symptom → cause

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `error deserializing server function arguments: missing field filters` | **Bundle mismatch** — WASM/client calls a server-fn shape the running binary does not implement (or vice versa) | Kill server → `./scripts/release-build.sh` → `./scripts/restart-dev.sh` or manual restart → smoke test |
| Home page shows old layout (e.g. `category-grid` on `/`) | **Stale SSR binary** — process started before layout change | Kill old process; start `target/release/onchainai` from latest build |
| `./scripts/smoke-test.sh` fails: `missing sidebar-brand markup` | **Wrong server** — old binary, debug build, or different checkout | Confirm PID: `lsof -i :3000`; restart matching release binary |
| `./scripts/smoke-test.sh` fails: `unexpected category-grid markup` | Stale binary serving outdated home page | Same as above |
| `not found: /pkg/...` in page or smoke | pkg not built or wrong `site-root` | Full `cargo leptos build --release`; check `target/site/pkg/` |
| Code "fixed" in repo but site unchanged | Server never restarted after build | Always restart after build (see §2) |
| Production OK, localhost broken | Local-only stale process or mixed artifacts | `verify-bundle.sh` + restart; production Docker build is self-contained |
| **Sidebar missing**, **buttons dead** (Sign in, filters, ☰), console `entered unreachable code` / hydration panic | **Bundle mismatch** (SSR binary ≠ WASM) **or** browser cached old `/pkg/onchainai.js` | `./scripts/restart-dev.sh` (never `cargo build --features ssr` alone for UI) → `verify-bundle.sh` → **`Cmd+Shift+R` hard refresh** → `node scripts/browser-smoke.mjs` |
| OAuth/login works once then nav still shows **Sign in** after SPA navigation | Non-blocking auth fetch in shell (SSR HTML ≠ hydrated DOM) | `TopNav` must use `Resource::new_blocking` keyed on pathname (see §4.1) |

Smoke scripts encode these checks: `scripts/smoke-test.sh`, `scripts/browser-smoke.mjs`, `scripts/local-auth-smoke.mjs`.

---

## 4. Debug history — 2026-06-27

**Incident:** Local dev showed `error deserializing server function arguments: missing field filters` and UI that did not match the current codebase.

**Timeline (KST / local):**

| Component | Timestamp | Notes |
|-----------|-----------|-------|
| Server process (port 3000) | Started **2026-06-26 ~22:59**, ran ~8 hours | Old binary kept serving SSR |
| `target/release/onchainai` | **2026-06-27 ~05:42** | Rebuilt without restarting server |
| `target/site/pkg/*` (WASM/JS) | **2026-06-27 ~06:46** | Rebuilt again — **newer than running binary** |

**Root cause:** SSR binary and WASM/pkg were from different builds. The browser hydrated with WASM expecting `ToolListRequest { filters, ... }` while the long-lived server process still exposed the older server-fn signature → deserialization error. Stale binary also served old home markup (`category-grid`).

**Fix:** Kill process on port 3000 → start `target/release/onchainai` from the **same** build as `target/site/pkg/` → `./scripts/smoke-test.sh http://localhost:3000` passed.

**Production:** Unaffected — Railway Docker image builds binary + pkg in one `cargo leptos build --release` layer.

---

## 4.1 Debug history — 2026-06-28 (local hydration / sidebar)

**Incident:** After sign-in UI and profile-menu work, localhost repeatedly showed **no sidebar interactivity**, **Sign in / filter buttons not clickable**, and Playwright logged `entered unreachable code` (tachys hydration).

**Root causes (two):**

1. **Partial server rebuild** — Agents ran `cargo build --release --features ssr` without `cargo leptos build --release`, so `target/release/onchainai` and `target/site/pkg/*` diverged by minutes. SSR markup and WASM handlers no longer matched.
2. **TopNav auth fetch** — A pathname-keyed `Resource::new` inside `Suspense` with an empty fallback let SSR and client disagree on shell markup. Fixed by `Resource::new_blocking` in `src/components/top_nav.rs` so auth HTML is in the initial SSR stream.

**Fix checklist (agents — run in order, do not skip):**

```bash
./scripts/ui-change-gate.sh
```

Then **hard refresh** the browser (`Cmd+Shift+R`) so cached WASM is dropped.

**Never for UI / Leptos component / auth shell changes:**

- ❌ `cargo build --features ssr`, `cargo build --release --features ssr`, or `cargo run --features ssr` as the final step
- ❌ Restarting the server without `./scripts/verify-bundle.sh`
- ❌ Assuming smoke PASS in CI means the user's tab does not need a hard refresh

**Code invariant:** Site shell auth (`TopNav`) uses **blocking** server-fn resolution for SSR/hydration parity; use pathname only as a refetch key, not as a reason to defer SSR output.

### Railway builder: Dockerfile vs RAILPACK

**Production:** Railway watches the **`main`** branch (repo default). `railway.json` pins **`builder: DOCKERFILE`** and **`dockerfilePath: Dockerfile`**.

Use **`./scripts/deploy-railway.sh`** or **`railway up`** so Railway builds from the Dockerfile.

| Method | Builder | Notes |
|--------|---------|-------|
| `./scripts/deploy-railway.sh` | Dockerfile | **Preferred** — single `cargo leptos build --release` in image |
| `railway up` (CLI) | Dockerfile | Same as deploy script |
| Git push to `main` (Railway GitHub integration) | Dockerfile | Auto-deploy when Railway is connected to `main`; must respect `railway.json` |
| Misconfigured Railway builder | Often **RAILPACK** | May ignore `railway.json` or stall in BUILDING; not validated for this repo |

If a Railway deploy sticks in **BUILDING** or uses RAILPACK instead of Dockerfile, cancel it and deploy via `./scripts/deploy-railway.sh` instead. Do not mix a RAILPACK image with a Dockerfile-built WASM/pkg bundle.

> **한국어 요약:** 8시간 넘게 돌던 로컬 서버(구 바이너리)와 새로 빌드한 WASM/pkg가 어긋나 서버 함수 역직렬화 오류 및 UI 불일치 발생. 프로세스 종료 후 동일 빌드 바이너리로 재시작하여 해결. 프로덕션은 정상.

---

## 5. Disk management (local macOS)

Leptos SSR + WASM builds are large (`target/` often 10–50GB). Linker failures on macOS also write **multi-GB** snapshots under `/tmp/onchainai*.ld-snapshot` — these are safe to delete and are a common hidden disk drain.

**`disk-guard.sh` thresholds (defaults):** free disk **≥25GB**, `target/` **≤35GB**. Override: `ONCHAINAI_MIN_FREE_GB`, `ONCHAINAI_MAX_TARGET_GB`. When over either limit it auto-runs `clean-build-artifacts.sh --incremental-only` once (`ONCHAINAI_DISK_GUARD_AUTOCLEAN=0` to disable).

**When `./scripts/disk-guard.sh` still fails:**

1. Fast reclaim: `./scripts/clean-build-artifacts.sh --incremental-only`
2. Preview full clean: `./scripts/clean-build-artifacts.sh --dry-run`
3. Full clean: `./scripts/clean-build-artifacts.sh` (`cargo clean` + `/tmp` linker snapshots)
4. Re-check: `df -h` — aim for **≥25GB** free before `cargo leptos build --release`
5. If still tight: `ONCHAINAI_DISK_GUARD_FORCE=1 ./scripts/release-build.sh` (emergency only)

**During development:**

- Prefer `cargo check --features ssr --lib` over full `cargo test` when disk is low — test binaries can trigger the same macOS `makeSymbolStringInPlace` linker bug as release builds.
- After any failed `cargo test` / `cargo build` link on macOS, run `./scripts/clean-build-artifacts.sh` to remove `/tmp` linker snapshots before the next build.
- Never let `target/` and `/tmp/onchainai*.ld-snapshot` grow together unchecked.

---

## 6. Pre-deploy checklist

Run in order before `./scripts/deploy-railway.sh`:

> **Branch:** Cut production deploys from `main`. `deploy-railway.sh` prints the current branch and warns (non-blocking) when it is not `main`.

1. **Disk guard** — `./scripts/disk-guard.sh` (or clean per §5; `ONCHAINAI_DISK_GUARD_FORCE=1` if emergency)
2. **Compile check** — `cargo check --features ssr --lib` (full `cargo test --features ssr` when disk/linker allow)
3. **Release build** — `./scripts/release-build.sh` (not partial `cargo build`)
4. **Bundle verify** — `./scripts/verify-bundle.sh`
5. **Local smoke** — restart release binary, then:
   - `./scripts/smoke-test.sh http://localhost:3000`
   - `node scripts/browser-smoke.mjs http://localhost:3000` (if Playwright installed)
6. **Deploy** — `./scripts/deploy-railway.sh` (echoes git branch; warns if not `main`; retries curl smoke before exit)
7. **Post-deploy verify** — `./scripts/post-deploy-verify.sh https://www.onchain-ai.xyz` (adds Playwright browser/click tests; curl smoke is redundant if step 6 just passed)

---

## 7. Browser cache

After deploy or a local rebuild, the browser may cache old `/pkg/onchainai.js` or `.wasm`.

- **Hard refresh:** macOS `Cmd+Shift+R`, Windows/Linux `Ctrl+Shift+R`
- **Clear site data** for `localhost:3000` or production host
- **DevTools → Network:** disable cache while debugging; confirm `/pkg/onchainai.js` loads and matches build time
- If errors persist after server restart + smoke pass, try a private/incognito window

Server-side smoke can pass while a cached client still fails — always confirm with hard refresh.

---

## 8. macOS linker note (`makeSymbolStringInPlace`)

On some macOS/Xcode toolchains, standalone:

```bash
cargo build --release --features ssr
```

may fail at link time with `makeSymbolStringInPlace` (or similar linker errors). That produces **no** new binary.

**If that happens:**

- Do **not** mix a partial WASM-only rebuild with an older `target/release/onchainai`.
- Prefer full `cargo leptos build --release` / `./scripts/release-build.sh` (often succeeds when plain `cargo build` does not).
- On Darwin, `./scripts/release-build.sh` automatically adds `RUSTFLAGS=-C symbol-mangling-version=v0` when no symbol-mangling flag is already set. This is the verified local workaround from 2026-06-27 for Apple clang linker failures while keeping the binary, WASM, JS, and served CSS in one build session.
- If link still fails, use the **last good** `target/release/onchainai` from a successful `cargo leptos build --release` **only if** `target/site/pkg/` is from that **same** build (check with `./scripts/verify-bundle.sh`).
- Never deploy or run locally when binary and pkg timestamps diverge by more than one intentional build session.

---

## Quick reference

```bash
./scripts/disk-guard.sh
cargo test --features ssr
./scripts/release-build.sh
./scripts/verify-bundle.sh
./scripts/restart-dev.sh
./scripts/agent-harness-check.sh
./scripts/ui-change-gate.sh
./scripts/smoke-test.sh http://localhost:3000
./scripts/deploy-railway.sh
./scripts/post-deploy-verify.sh https://www.onchain-ai.xyz
```

See also: [[../AGENTS.md]] Deploy runbook, [[MVP_DESIGN]] build section.
