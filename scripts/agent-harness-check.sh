#!/usr/bin/env bash
# Cheap guard for the LLM-wiki agent harness contract.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

fail() {
  echo "AGENT HARNESS FAIL: $*" >&2
  exit 1
}

agents_lines="$(wc -l < AGENTS.md | tr -d '[:space:]')"
if [[ -z "$agents_lines" ]]; then
  fail "could not count AGENTS.md lines"
fi
if (( agents_lines >= 70 )); then
  fail "AGENTS.md must stay under 70 lines (current: ${agents_lines})"
fi

for path in \
  AGENTS.md \
  docs/AGENT_HARNESS.md \
  docs/BRANCH_PROTECTION.md \
  docs/BUILD_DEPLOY_RULES.md \
  docs/AGENT_READINESS_REPORT.md \
  docs/UI_UX_DESIGN.md \
  DESIGN.md \
  .github/CODEOWNERS \
  .github/pull_request_template.md \
  .agents/skills/onchainai-ui-workflow/SKILL.md \
  .cursor/hooks.json \
  .claude/settings.json \
  scripts/agent-readiness-report.sh \
  scripts/agent-readiness-report.mjs \
  scripts/configure-branch-protection.sh \
  .pr_agent.toml \
  .coderabbit.yaml \
  scripts/git-hooks/pre-commit \
  scripts/git-hooks/pre-push \
  scripts/agent-start.sh \
  scripts/dev-watch.sh \
  scripts/ui-staleness-check.sh \
  scripts/test-ui-staleness-check.sh \
  scripts/install-agent-hooks.sh \
  scripts/ui-change-gate.sh \
  scripts/sync-ui-watch-paths.mjs \
  scripts/ui-watch-paths.inc.sh \
  scripts/ui-browser-gate.mjs \
  scripts/verify-dev-watch.sh
do
  [[ -e "$path" ]] || fail "missing required route/gate file: ${path}"
done

grep -Fq 'docs/AGENT_HARNESS.md' AGENTS.md || \
  fail "AGENTS.md must route agent workflow to docs/AGENT_HARNESS.md"
grep -Fq './scripts/ui-change-gate.sh' AGENTS.md || \
  fail "AGENTS.md must mention the UI/auth/routing gate"
grep -Fq './scripts/dev-watch.sh' AGENTS.md || \
  fail "AGENTS.md must mention dev-watch.sh"
grep -Fq './scripts/install-agent-hooks.sh' AGENTS.md || \
  fail "AGENTS.md must mention install-agent-hooks.sh"
grep -Fq './scripts/agent-readiness-report.sh' AGENTS.md || \
  fail "AGENTS.md must mention the agent readiness report"

[[ -x scripts/agent-readiness-report.sh ]] || fail "scripts/agent-readiness-report.sh must be executable"
[[ -x scripts/dev-watch.sh ]] || fail "scripts/dev-watch.sh must be executable"
[[ -x scripts/git-hooks/pre-commit ]] || fail "scripts/git-hooks/pre-commit must be executable"
[[ -x scripts/git-hooks/pre-push ]] || fail "scripts/git-hooks/pre-push must be executable"
[[ -x scripts/agent-start.sh ]] || fail "scripts/agent-start.sh must be executable"
[[ -x scripts/ui-staleness-check.sh ]] || fail "scripts/ui-staleness-check.sh must be executable"
[[ -x scripts/test-ui-staleness-check.sh ]] || fail "scripts/test-ui-staleness-check.sh must be executable"
[[ -x scripts/install-agent-hooks.sh ]] || fail "scripts/install-agent-hooks.sh must be executable"
[[ -x scripts/ui-change-gate.sh ]] || fail "scripts/ui-change-gate.sh must be executable"
[[ -x scripts/verify-dev-watch.sh ]] || fail "scripts/verify-dev-watch.sh must be executable"
[[ -x scripts/configure-branch-protection.sh ]] || fail "scripts/configure-branch-protection.sh must be executable"
[[ -x .cursor/hooks/ui-staleness-stop.sh ]] || fail ".cursor/hooks/ui-staleness-stop.sh must be executable"
bash -n scripts/agent-readiness-report.sh
bash -n scripts/dev-watch.sh
bash -n scripts/ui-staleness-check.sh
bash -n scripts/test-ui-staleness-check.sh
bash -n scripts/install-agent-hooks.sh
bash -n scripts/agent-start.sh
node --check scripts/agent-readiness-report.mjs >/dev/null
node --check scripts/sync-ui-watch-paths.mjs >/dev/null
node --check scripts/ui-browser-gate.mjs >/dev/null
node --check scripts/browser-smoke.mjs >/dev/null
node --check scripts/click-test.mjs >/dev/null
bash -n scripts/ui-change-gate.sh
bash -n scripts/verify-dev-watch.sh
bash -n scripts/configure-branch-protection.sh
node scripts/sync-ui-watch-paths.mjs --check
grep -Fq 'disable_auto_feedback = true' .pr_agent.toml || \
  fail ".pr_agent.toml must disable Qodo auto feedback (disable_auto_feedback = true)"
grep -Fq 'auto_review:' .coderabbit.yaml && grep -Fq 'enabled: false' .coderabbit.yaml || \
  fail ".coderabbit.yaml must keep auto_review.enabled false"
hooks_path="$(git config --local --get core.hooksPath 2>/dev/null || true)"
if [[ "$hooks_path" != "scripts/git-hooks" ]]; then
  ./scripts/install-agent-hooks.sh >/dev/null
fi
./scripts/install-agent-hooks.sh --check-only >/dev/null
./scripts/test-ui-staleness-check.sh
./scripts/ui-change-gate.sh --check-only >/dev/null
./scripts/verify-dev-watch.sh --check-only >/dev/null
./scripts/configure-branch-protection.sh --check-only >/dev/null

echo "AGENT HARNESS PASS (AGENTS.md lines: ${agents_lines})"