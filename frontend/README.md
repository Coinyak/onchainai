# OnchainAI Frontend

Next.js (App Router) frontend for [OnchainAI](https://www.onchain-ai.xyz),
deployed on Vercel. All data comes from the Rust API (Railway) — this app has no
database access of its own.

## Dev

```bash
npm ci
API_PROXY_TARGET=http://localhost:3000 npm run dev -- --port 3001
```

Run the Rust API first (`cargo run --features ssr` from the repo root, port 3000).
`next.config.ts` rewrites `/api/*`, `/auth/*`, `/onboarding/*`, and `/mcp` to
`API_PROXY_TARGET`, so the browser only ever talks to this app's origin.

> Repo rule: while iterating on UI/auth/routing use `../scripts/dev-watch.sh`,
> and finish with `../scripts/ui-change-gate.sh` — see the root `AGENTS.md`.

## Environment

| Var | Purpose | Default |
|---|---|---|
| `API_PROXY_TARGET` | Rust API origin the rewrites proxy to | `http://localhost:3000` |
| `NEXT_PUBLIC_API_URL` | Optional absolute API URL for client fetches | `""` (same-origin) |
| `NEXT_PUBLIC_GITHUB_REPO` | GitHub link in the top nav | repo URL |

## Styles

`npm run build` runs a `prebuild` step that copies `../style/output.css` into
`styles/site-output.css` — the shared design-token stylesheet is built at the
repo root, not here.

## Structure

- `app/` — routes: home, `/tools`, `/tools/[slug]`, `/connect` (MCP hub),
  `/categories`, `/compare`, `/blueprints`, `/dashboard`, `/toolkit`, `/submit`,
  `/login`, `/onboarding`, and the `/admin` dashboard
- `components/` — UI components (tool cards, install guide panel, connect hub)
- `lib/` — API client, MCP connect constants/deeplinks, formatting
- `hooks/` — shared React hooks

## Deploy

Vercel project with `API_PROXY_TARGET` (and optional `NEXT_PUBLIC_*`) set in the
environment. See `../docs/BUILD_DEPLOY_RULES.md` for the smoke gates that must
pass after deploys.
