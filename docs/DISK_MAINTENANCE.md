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

## Quick audit (monthly)

```bash
df -h /System/Volumes/Data
du -sh ~/ ~/.factory/logs ~/.codex ~/.hermes/state.db
du -sh /System/Volumes/Data/private/var/folders/k7/_r0bjtp12dngr0ncryvtt4mc0000gn/X/*code_sign_clone 2>/dev/null
find /Users/love -name target -type d -prune -exec du -sh {} \; 2>/dev/null | sort -hr | head -5
cd ~/OnchainAI && ./scripts/disk-guard.sh
```

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