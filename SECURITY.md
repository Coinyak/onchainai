# Security Policy

## Reporting a vulnerability

Please **do not open a public issue** for security problems.

- Preferred: GitHub → Security tab → **Report a vulnerability** (private advisory)
  on this repository.
- If that is unavailable, email the maintainer listed on the GitHub profile of
  [@Coinyak](https://github.com/Coinyak) with subject
  `[onchainai security]`.

Include reproduction steps, the affected endpoint or component, and impact. You
should receive an acknowledgement within 72 hours. Please give us a reasonable
window to ship a fix before any public disclosure.

## Scope

- The `onchainai` Rust API and MCP server (`src/`), including
  `https://www.onchain-ai.xyz/mcp` and `/api/v2/*`
- The Next.js frontend (`frontend/`), served at `https://www.onchain-ai.xyz`
- Auth flows: GitHub OAuth, email magic link, SIWX wallet sign-in
- The Claude Code plugin bundle (`plugin/onchainai/`)
- Database RLS policies (`migrations/`)

Out of scope: third-party tools *listed in* the directory (report those to their
own maintainers — but if our install-risk gate mislabels a dangerous tool as
safe, that IS in scope and we want to know), volumetric DoS, and issues
requiring a compromised maintainer account.

## What we especially care about

- Bypasses of the public visibility gate (`PUBLIC_TOOL_WHERE` / RLS) that leak
  unapproved, quarantined, or critical-risk tools
- Ways to make `get_install_guide` emit a `critical`-risk or attacker-controlled
  command
- Auth/session bugs (JWT, OAuth callback, SIWX signature verification)
- Secret exposure (`SUPABASE_SERVICE_KEY`, `JWT_SECRET` must never reach clients)
- SQL injection (all queries must stay parameterized) and stored XSS

## Payment-safety guarantee

OnchainAI publishes x402 payment *metadata* only. There is intentionally **no**
custody, facilitator, gateway, or fund-moving code in this repository, and no
wallet keys in its configuration. Anything that appears to add such a path is a
bug — report it.

## Design reference

The full security design (auth model, RLS policies, headers, rate limits,
secret redaction) lives in [docs/SECURITY.md](docs/SECURITY.md).
