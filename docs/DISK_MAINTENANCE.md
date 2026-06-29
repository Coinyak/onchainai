# Disk Maintenance Memo

> Personal disk hygiene log for `love@mac` (228GB SSD).  
> Read with [BUILD_DEPLOY_RULES.md](./BUILD_DEPLOY_RULES.md) before OnchainAI builds.

---

## 2026-06-27 — Full cleanup (orchestrated)

### Starting point

- **Disk:** 228GB total, **~175GB used**, **~13GB free** (94%)
- **Symptom:** Rust/Leptos builds hit `No space left on device`; macOS linker snapshots in `/tmp`.

### Root cause (not just OnchainAI)

| Layer | Size | Notes |
|-------|------|-------|
| `~/` (home) | ~63GB | Projects, AI tools, Library |
| `private/var/folders/...` (per-user temp) | **~42GB hidden** | Not visible in `du ~/Library` |
| `/Applications` | ~18GB | Duplicate `* 2.app` installs |
| `/opt/homebrew` | ~6GB | |

**Largest hidden junk:** macOS app code-sign clones left behind by Chrome/Codex updates.

### Actions performed

| Step | Target | ~Freed |
|------|--------|--------|
| A | `var/folders/.../X/com.google.Chrome.code_sign_clone` | 26GB |
| A | `var/folders/.../X/com.openai.codex.code_sign_clone` | 12GB |
| B | `var/folders/.../T/kiro-cli-download*` | 4.1GB |
| C | `Documents/BTC agent/target` (`cargo clean`) | 4.8GB |
| C | `tiny/target` (`cargo clean`) | 1.6GB |
| D | `Library/Application Support/Claude/vm_bundles/*` | 10GB |
| E | `/Applications/Visual Studio Code 2.app`, `Claude 2.app` | ~1.2GB |
| F | `~/.codex/sessions/*`, `logs_2.sqlite` (backup → `~/disk-cleanup-archive-2026-06-27/`) | ~3.6GB |
| Extra | Factory log truncate, Telegram Sparkle cache, OnchainAI `clean-build-artifacts.sh`, `/tmp` ld-snapshots | variable |

**Before deleting code_sign_clone:** quit Chrome, Codex, Kiro (or kill `kiro_cli_desktop`).

### After cleanup (measured)

| Metric | Before | After |
|--------|--------|-------|
| Used | ~175GB (94%) | **~99GB (53%)** |
| Free | ~13GB | **~91GB** |
| `~/` size | ~63GB | **~44GB** |

Codex log backup: `~/disk-cleanup-archive-2026-06-27/logs_2.sqlite.bak`

Re-check anytime:

```bash
df -h /System/Volumes/Data
du -sh ~/ /System/Volumes/Data/private/var/folders/k7/_r0bjtp12dngr0ncryvtt4mc0000gn
```

---

## Recurring auto-growers (watch list)

| Path | Growth | Prevention |
|------|--------|------------|
| `~/.factory/logs/droid-log-single.log` | Unbounded single file | `truncate -s 0` when >500MB; add cron |
| `private/var/folders/.../X/*code_sign_clone` | Chrome/Codex updates | Quarterly delete after quitting apps |
| `private/var/folders/.../T/kiro-cli-download*` | Kiro CLI | Delete after Kiro updates |
| `/tmp/onchainai*.ld-snapshot` | Failed Rust links | `OnchainAI/scripts/clean-build-artifacts.sh` |
| `*/target/` (Rust) | Every build | `cargo clean` when project idle |
| `HyperMax/reports/trade-cycles/` | Was **64GB** once | Retention policy / `feat/log-retention-prune` |
| `~/.codex/logs_2.sqlite` | Session logs | Periodic delete or vacuum |
| `~/.hermes/state.db` | Agent sessions | Review size monthly |

---

## OnchainAI build scripts (reference)

`disk-guard.sh` runs before heavy builds (`release-build.sh`). Defaults: **≥25GB free** and **`target/` ≤35GB** (integer GB, floored). Tune without editing scripts:

| Variable | Default | Purpose |
|----------|---------|---------|
| `ONCHAINAI_MIN_FREE_GB` | 25 | Minimum free disk before build |
| `ONCHAINAI_MAX_TARGET_GB` | 35 | Maximum `target/` size |
| `ONCHAINAI_STALE_MAIN_CRATE_PRUNE_GB` | 16 | Start pruning stale local `onchainai` debug artifact groups when `target/` exceeds this size |
| `ONCHAINAI_STALE_MAIN_CRATE_KEEP` | 3 | Keep the newest N local debug artifact groups |
| `ONCHAINAI_DISK_GUARD_AUTOCLEAN` | 1 | Auto-run `--incremental-only` when over threshold |
| `ONCHAINAI_DISK_GUARD_FORCE` | 0 | Skip checks (emergency only) |

**Cleanup ladder (fast → slow):**

0. Automatic before heavy builds: `disk-guard.sh` sweeps linker snapshots, then prunes stale local `onchainai` debug artifact groups once `target/` exceeds `ONCHAINAI_STALE_MAIN_CRATE_PRUNE_GB`; third-party compiled deps are kept.
1. `./scripts/clean-build-artifacts.sh --stale-main-crate --stale-main-crate-keep 3` — remove old hashed `onchainai` / `libonchainai` debug groups; keeps latest local groups and dependency cache.
2. `./scripts/clean-build-artifacts.sh --incremental-only` — drops `target/*/incremental/` only; keeps compiled deps.
3. `./scripts/clean-build-artifacts.sh --dry-run` — preview full clean + `/tmp` linker snapshots.
4. `./scripts/clean-build-artifacts.sh` — full `cargo clean` + linker snapshots (slow next build).

`Cargo.toml` already limits debug bloat: `[profile.dev] debug = "line-tables-only"` and `[profile.dev.package."*"] debug = false`.

---

## Quick audit (monthly)

```bash
df -h /System/Volumes/Data
du -sh ~/ ~/.factory/logs ~/.codex ~/.hermes/state.db
du -sh /System/Volumes/Data/private/var/folders/k7/_r0bjtp12dngr0ncryvtt4mc0000gn/X/*code_sign_clone 2>/dev/null
find /Users/love -name target -type d -prune -exec du -sh {} \; 2>/dev/null | sort -hr | head -5
cd ~/OnchainAI && ./scripts/disk-guard.sh
```

---

## 2026-06-29 — Root cause: linker-snapshot sweep was a no-op (now automated)

### Symptom

Disk down to **~7GB free**. `/private/tmp` held **55GB** of
`libonchainai.dylib-*.ld-snapshot` dirs (~2.5GB each) — one per dylib link,
never removed.

### Root cause (a real bug, not just hygiene)

`clean-build-artifacts.sh` already swept these, but scanned `find /tmp ...`.
On macOS `/tmp` is a **symlink** to `/private/tmp`, and BSD `find` does **not**
descend a symlinked start path without `-H`/`-L`. So the sweep matched nothing
and had **never actually run** on macOS — snapshots piled up unbounded.
`disk-guard.sh`'s auto-clean called `--incremental-only`, which inherited the
same broken sweep, so the guard never caught it either.

### Fix (this branch)

| Change | File |
|--------|------|
| Resolve `/tmp` → real path (and dedupe `$TMPDIR`) before `find`; sweep now works | `scripts/clean-build-artifacts.sh` |
| New `--snapshots-only` flag: sweep snapshots, never touch `target/` | `scripts/clean-build-artifacts.sh` |
| Sweep snapshots first, every run (size-independent) | `scripts/disk-guard.sh` |
| Per-user scheduled sweep (macOS LaunchAgent), portable installer | `scripts/install-disk-autoclean.sh` |

### Work rule — run once after clone

```bash
./scripts/install-disk-autoclean.sh   # daily 13:00 + at login; sweeps snapshots only
```

This makes the disk self-maintaining: build as often as you like, snapshots are
swept automatically. Manual one-off: `./scripts/clean-build-artifacts.sh --snapshots-only`.
Uninstall: `./scripts/install-disk-autoclean.sh --uninstall`.

---

## Safe cleanup script (orchestrator)

```bash
# Quit apps first!
osascript -e 'quit app "Google Chrome"' 2>/dev/null
osascript -e 'quit app "Codex"' 2>/dev/null

VARF=$(getconf DARWIN_USER_TEMP_DIR 2>/dev/null | sed 's|/T/$||' | sed 's|/T||')
# Or fixed path for this Mac:
VARF="/System/Volumes/Data/private/var/folders/k7/_r0bjtp12dngr0ncryvtt4mc0000gn"

rm -rf "$VARF/X/com.google.Chrome.code_sign_clone" \
       "$VARF/X/com.openai.codex.code_sign_clone" \
       "$VARF/T/kiro-cli-download"*

truncate -s 0 ~/.factory/logs/droid-log-single.log 2>/dev/null
cd ~/OnchainAI && ./scripts/clean-build-artifacts.sh
brew cleanup -s
```

---

## Korean summary (한국어 요약)

- **228GB 디스크의 94%는 “정상 사용”이 아니라 누적 캐시·임시파일·중복 앱·Rust target 때문.**
- **가장 큰 숨은 원인:** `private/var/folders` 안 Chrome/Codex `code_sign_clone` (~38GB).
- **OnchainAI 빌드 전:** 여유 **25GB+** 권장 (`disk-guard.sh`).
- **이 문서와 `BUILD_DEPLOY_RULES.md`를 함께 볼 것.**
