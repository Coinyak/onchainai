## Summary

<!-- What changed and why -->

## Checklist

- [ ] Scope matches the issue/task; no unrelated drive-by edits
- [ ] `cargo fmt` and `cargo clippy --features ssr -- -W clippy::all` (or note why skipped)
- [ ] `./scripts/agent-harness-check.sh` for harness/doc/gate changes

### UI / auth / routing

If this PR touches `src/pages`, `src/components`, `style`, `src/app.rs`, auth shell/nav, UI server functions, or route behavior:

- [ ] Iterated with `./scripts/dev-watch.sh` (not standalone `cargo run --features ssr`)
- [ ] `./scripts/ui-change-gate.sh --tier smoke` minimum; `--tier full` when feasible
- [ ] `./scripts/install-agent-hooks.sh` run once on the machine (pre-commit staleness guard)

## Notes

<!-- Blockers, env requirements, screenshots -->