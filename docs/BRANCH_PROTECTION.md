# Branch Protection

> Related docs: [[AGENT_HARNESS]] | [[../.github/workflows/ci.yml]]

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