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
| CSS | `style/output.css` |

**Do not mix partial builds:**

- ❌ `cargo build --release --features ssr` alone (server only; no WASM/pkg refresh)
- ❌ Rebuilding only WASM/pkg while keeping an hours-old binary
- ❌ `cargo run --features ssr` debug server against release `target/site/pkg/`
- ✅ `./scripts/release-build.sh` → restart server → smoke test

Verify coherence before starting the server:

```bash
./scripts/verify-bundle.sh
```

Checks `target/release/onchainai`, `target/site/pkg/*`, and `style/output.css` mtimes (default ±60s tolerance; override with `ONCHAINAI_BUNDLE_MAX_SKEW_SEC`).

---

## 2. Local dev workflow

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

Smoke scripts encode these checks: `scripts/smoke-test.sh`, `scripts/browser-smoke.mjs`.

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

> **한국어 요약:** 8시간 넘게 돌던 로컬 서버(구 바이너리)와 새로 빌드한 WASM/pkg가 어긋나 서버 함수 역직렬화 오류 및 UI 불일치 발생. 프로세스 종료 후 동일 빌드 바이너리로 재시작하여 해결. 프로덕션은 정상.

---

## 5. Disk management (local macOS)

Leptos SSR + WASM builds are large (`target/` often 10–50GB). Linker failures on macOS also write **multi-GB** snapshots under `/tmp/onchainai*.ld-snapshot` — these are safe to delete and are a common hidden disk drain.

**When `./scripts/disk-guard.sh` fails (<25GB free):**

1. Preview cleanup: `./scripts/clean-build-artifacts.sh --dry-run`
2. Run cleanup: `./scripts/clean-build-artifacts.sh` (`cargo clean` + `/tmp` linker snapshots)
3. Re-check: `df -h` — aim for **≥25GB** free before `cargo leptos build --release`
4. If still tight: `ONCHAINAI_DISK_GUARD_FORCE=1 ./scripts/release-build.sh` (emergency only)

**During development:**

- Prefer `cargo check --features ssr --lib` over full `cargo test` when disk is low — test binaries can trigger the same macOS `makeSymbolStringInPlace` linker bug as release builds.
- After any failed `cargo test` / `cargo build` link on macOS, run `./scripts/clean-build-artifacts.sh` to remove `/tmp` linker snapshots before the next build.
- Never let `target/` and `/tmp/onchainai*.ld-snapshot` grow together unchecked.

---

## 6. Pre-deploy checklist

Run in order before `./scripts/deploy-railway.sh`:

1. **Disk guard** — `./scripts/disk-guard.sh` (or clean per §5; `ONCHAINAI_DISK_GUARD_FORCE=1` if emergency)
2. **Compile check** — `cargo check --features ssr --lib` (full `cargo test --features ssr` when disk/linker allow)
3. **Release build** — `./scripts/release-build.sh` (not partial `cargo build`)
4. **Bundle verify** — `./scripts/verify-bundle.sh`
5. **Local smoke** — restart release binary, then:
   - `./scripts/smoke-test.sh http://localhost:3000`
   - `node scripts/browser-smoke.mjs http://localhost:3000` (if Playwright installed)
6. **Deploy** — `./scripts/deploy-railway.sh`
7. **Post-deploy verify** — `./scripts/post-deploy-verify.sh https://www.onchain-ai.xyz`

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
./scripts/smoke-test.sh http://localhost:3000
./scripts/deploy-railway.sh
./scripts/post-deploy-verify.sh https://www.onchain-ai.xyz
```

See also: [[../AGENTS.md]] Deploy runbook, [[MVP_DESIGN]] build section.