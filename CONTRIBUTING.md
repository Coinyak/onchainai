# Contributing to OnchainAI

Thanks for helping unify crypto tooling! There are three ways to contribute,
in increasing order of effort:

1. **Submit a missing tool** — no code needed: [onchain-ai.xyz/submit](https://www.onchain-ai.xyz/submit)
2. **File issues** — bugs, mislabeled tools, bad install commands, docs gaps
3. **Code** — read on

## Dev setup

Prereqs: Rust ≥ 1.85 (CI uses 1.90), Node ≥ 20, a Postgres database
(Supabase or local).

```bash
git clone https://github.com/hoyeon4315-cpu/onchainai
cd onchainai
cp .env.example .env            # fill in DB URL, keys — see comments in the file
./scripts/install-agent-hooks.sh  # once: git pre-commit hooks (UI staleness guard)

cargo run --features ssr        # API on :3000
cd frontend && npm ci && API_PROXY_TARGET=http://localhost:3000 npm run dev -- --port 3001
```

The crate's default feature set is empty — server-touching `cargo` commands
always need `--features ssr`.

## Before you open a PR

```bash
cargo test --features ssr
cargo clippy --features ssr -- -W clippy::all
cargo fmt --check
```

- **UI / auth / routing changes**: iterate with `./scripts/dev-watch.sh` and
  finish with `./scripts/ui-change-gate.sh` (the pre-commit hook blocks stale UI
  bundles). Don't end UI work with a bare `cargo build`.
- **Schema changes**: add a numbered file in `migrations/`, keep RLS policies in
  sync, then run migrations and `cargo sqlx prepare`.
- **Plugin changes** (`plugin/onchainai/`): run `claude plugin validate .` and
  bump `version` in `plugin.json`, or installed users never receive the update.
- State in the PR description which verification commands you actually ran.

## Ground rules

- Never commit `.env`, `target/`, build artifacts, or any credential. Server
  secrets (`SUPABASE_SERVICE_KEY`, `JWT_SECRET`) must never reach client code.
- All SQL goes through sqlx parameterized queries. No raw HTML injection.
- x402 stays metadata-only: no custody, facilitator, gateway, fund-moving, or
  undocumented `referrer`/`split` payment fields — PRs adding these are closed.
- Auth is required for comments, upvotes, bookmarks, and admin routes; admin
  checks live server-side.
- CI (`.github/workflows/ci.yml`) is manual-dispatch only to protect the Actions
  budget. Maintainers trigger it on PRs; use `[skip ci]` in commit messages for
  pushes that should not wake anything.

## AI coding agents

This repo is agent-native. If you're an AI agent (or pairing with one), start at
[AGENTS.md](AGENTS.md) — it routes to topic docs and the executable gates. The
same PR rules above apply to agent-authored changes.

## Security issues

Never open a public issue for a vulnerability — see [SECURITY.md](SECURITY.md).

## License

By contributing you agree your contributions are licensed under the
[MIT License](LICENSE).
