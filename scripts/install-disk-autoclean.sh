#!/usr/bin/env bash
# Install a per-user periodic sweep of linker snapshots so disk never silently
# fills from repeated builds. Portable: derives paths from the current checkout
# and user — no hardcoded usernames. Run once after clone (idempotent).
#
#   ./scripts/install-disk-autoclean.sh            # install (daily 13:00 + at login)
#   ./scripts/install-disk-autoclean.sh --uninstall
#
# Background: every macOS dylib link can drop a multi-GB *.ld-snapshot in the
# temp dir. They are throwaway but never auto-removed, so they pile to tens of
# GB. This schedules `clean-build-artifacts.sh --snapshots-only`, which only
# sweeps those snapshots and never touches target/, source, or git state.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLEANER="${ROOT}/scripts/clean-build-artifacts.sh"
LABEL="com.onchainai.disk-autoclean"
UNINSTALL=false
[[ "${1:-}" == "--uninstall" ]] && UNINSTALL=true

case "$(uname -s)" in
  Darwin) ;;
  *)
    echo "[disk-autoclean] non-macOS host detected."
    echo "Linker snapshots are a macOS phenomenon; no scheduled job needed."
    echo "If you still want periodic cleanup, add to your crontab:"
    echo "  0 13 * * *  ${CLEANER} --snapshots-only >/dev/null 2>&1"
    exit 0
    ;;
esac

PLIST="${HOME}/Library/LaunchAgents/${LABEL}.plist"
LOG="${HOME}/Library/Logs/onchainai-disk-autoclean.log"

if [[ "$UNINSTALL" == true ]]; then
  launchctl unload "$PLIST" 2>/dev/null || true
  rm -f "$PLIST"
  echo "[disk-autoclean] uninstalled ($PLIST removed)"
  exit 0
fi

[[ -x "$CLEANER" ]] || chmod +x "$CLEANER" 2>/dev/null || true
mkdir -p "${HOME}/Library/LaunchAgents" "${HOME}/Library/Logs"

cat > "$PLIST" <<PLIST_EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>${CLEANER}</string>
        <string>--snapshots-only</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key><integer>13</integer>
        <key>Minute</key><integer>0</integer>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${LOG}</string>
    <key>StandardErrorPath</key>
    <string>${LOG}</string>
</dict>
</plist>
PLIST_EOF

launchctl unload "$PLIST" 2>/dev/null || true
launchctl load "$PLIST"

echo "[disk-autoclean] installed: ${PLIST}"
echo "  runs: ${CLEANER} --snapshots-only  (daily 13:00 + at login)"
echo "  log:  ${LOG}"
echo "  uninstall: ./scripts/install-disk-autoclean.sh --uninstall"
