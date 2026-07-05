# Branch Protection

> Related docs: [AGENT_HARNESS](AGENT_HARNESS.md) | [ci.yml](../.github/workflows/ci.yml)

Require the **`ci-success`** check before merging to `main`. That job aggregates:

| Job | Always runs | Purpose |
|-----|-------------|---------|
| `rust` | Yes | `cargo fmt`, clippy, lib tests |
| `agent-harness` | Yes | Harness contract + readiness (L1–L4) |
| `ui-coherence` | When UI paths change | Release build, bundle verify, smoke, browser gate |
| `ci-success` | Yes | Fails if any required job failed |

Workflow file: `.github/workflows/ci.yml` — merge gate job name is **`ci-success`**.

## GitHub admin: require `ci-success` on `main`

1. Open **Settings → Branches → Branch protection rules → Add rule** (or edit the existing `main` rule).
2. **Branch name pattern:** `main`
3. Enable **Require status checks to pass before merging**.
4. Enable **Require branches to be up to date before merging** (recommended).
5. In the status-check search box, add **`ci-success`** (exact job name from the workflow).
6. Optionally enable **Require pull request reviews before merging** and **Do not allow bypassing the above settings**.
7. Save changes.

### Optional: feature branches

Repeat with pattern `feat/*` if you want the same gate on long-lived feature branches. Most teams protect only `main`.

## CLI helper

```bash
./scripts/configure-branch-protection.sh --check-only   # validate gh auth + print steps
./scripts/configure-branch-protection.sh --apply        # attempt to set protection (needs admin)
```

Without admin rights, use the manual steps above or ask a repo admin to run `--apply`.

## PR review bots (manual only)

This repo does **not** use automatic Qodo or Copilot PR reviews (saves quota).

| Bot | Auto review | How to request |
|-----|-------------|----------------|
| **Qodo** | Off — `.pr_agent.toml` sets `disable_auto_feedback = true` | PR comment: `/review` (after config is on default branch) |
| **CodeRabbit** | Off — `.coderabbit.yaml` | PR comment: `@coderabbitai review` |
| **GitHub Copilot** | Must be off in GitHub UI (not repo-file configurable) | See below |

### Disable Copilot automatic PR review

Repo admin or owner:

1. **Settings → Copilot → Code review** — turn off automatic review for this repository, **or**
2. **Settings → Rules → Rulesets** — remove **Automatically request Copilot code review** from any active ruleset, **or**
3. **Profile → Copilot settings** — disable **Automatic Copilot code review** for your own PRs if enabled personally.

Copilot coding instructions (`.github/copilot-instructions.md`) are for in-IDE assistance only, not PR auto-review.