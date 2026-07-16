# Vercel cost: MCP/API proxy and edge-routing fix

> Related: [BUILD_DEPLOY_RULES.md](BUILD_DEPLOY_RULES.md) · [CONNECT.md](CONNECT.md) · [SECURITY.md](SECURITY.md)
>
> Trigger: 2026-07-14 — Vercel usage reached ~$16 pre-OKX-approval. Root-caused to the
> `www.onchain-ai.xyz` → Railway proxy plus a `force-dynamic` 404. This doc records the
> code fixes already applied and the owner-only edge-routing migration still pending.

## Root cause

`www.onchain-ai.xyz` is a Vercel domain. `frontend/next.config.ts` rewrites `/mcp`,
`/mcp/:path*`, `/api/*`, `/auth/*`, `/onboarding/*` to the Railway API. Proven live by
response headers carrying **both** `server: Vercel` and `x-railway-edge` / `x-railway-request-id`
with `x-vercel-cache: MISS`:

- Every agent MCP call and browser API call transits Vercel first, uncached (`no-store`),
  then is proxied to Railway. Vercel bills the **edge request + bandwidth both directions**
  while Railway does the actual compute. Double-paying.
- A `force-dynamic` `not-found.tsx` that fetched Railway made every bot/scanner 404
  (`/.env`, `/wp-login.php`, random paths) a serverless invocation + backend fetch.
- `sitemap.xml` rendered per request (`x-vercel-cache: MISS`), scanning all tool slugs
  from Railway on each crawler hit.

## Will OKX approval break Vercel?

No — the paid path pays for itself. A paid `tools/call` is ~30KB through Vercel
(≈ $0.0000015 bandwidth) against **$0.1** revenue — Vercel overhead is ~0.005% of revenue,
and the edge scales fine. The cost that grows **without** offsetting revenue is
unpaid/free traffic: free `/mcp` discovery, 402 retry loops, and bots. That is what the
current ~$16 is, and it is independent of OKX.

## Applied fixes (code, shipped in this change)

| File | Change | Effect |
|------|--------|--------|
| `frontend/app/not-found.tsx` | Removed `export const dynamic = "force-dynamic"` → `revalidate` | 404 is now `○ Static`/ISR — bogus URLs are CDN hits, not function invocations |
| `frontend/app/sitemap.ts` | Added `export const revalidate = 3600` | `sitemap.xml` is now `○ Static`/ISR — crawlers no longer trigger a per-hit Railway slug-scan |

Verified via `next build` route table: `/_not-found` and `/sitemap.xml` are now `○ (Static)`.

**Not done on purpose:** `generateStaticParams` prerendering of `/tools/[slug]` was tried and
reverted. Build-time prerender of N slugs couples every deploy to N live Railway fetches;
a single slow tool-detail fetch (`/tools/bsv-mcp` timed out, HTTP 408) fails the whole build.
On-demand ISR (`revalidate = 300`) already bounds per-slug cost. Keep it on-demand.

## Rate limiting — already present, but does not shield Vercel

`src/server/rate_limit.rs` + `src/server/http_app.rs` already apply `MCP_PER_MINUTE = 100`
per IP (tower_governor) to **both** `/mcp` and `/mcp/okx`. Two caveats:

1. **It runs on Railway, after Vercel already proxied the request** — a Railway 429 does not
   refund the Vercel edge request/bandwidth. Only edge-side limiting (below) protects the
   Vercel bill.
2. **Behind the Vercel proxy the rate-limit key collapses to Vercel's egress IP.**
   `is_trusted_proxy()` trusts only private/loopback peers, so the real client IP inside
   `x-forwarded-for` is Vercel's public egress — all agents routing through Vercel share one
   100/min bucket. The edge-routing migration also fixes this (real client IP via Cloudflare).

## Pending: move `/mcp*` off Vercel (owner / DNS action)

The single biggest lever. Goal: `/mcp` and `/mcp/okx` reach Railway **directly**, so neither
free nor paid MCP traffic touches the Vercel meter — **without changing the public URL**
(`https://www.onchain-ai.xyz/mcp/okx` is committed to OKX ASP #4609, service id 33054, under
review; changing the host forces an OKX re-submit).

### Recommended: Cloudflare in front of `www`, path-routed origin

1. Put `onchain-ai.xyz` on Cloudflare DNS; proxy (orange-cloud) `www`.
2. Default: `www` → Vercel (keep the site on Vercel).
3. Add an **Origin Rule / Config Rule** matching path `^/mcp(/.*)?$` → override origin to the
   Railway host (`onchainai-production.up.railway.app`), preserving the path.
   Result: `/mcp*` bypasses Vercel entirely; everything else still hits Vercel.
4. Add a Cloudflare **rate-limiting rule** on `/mcp*` (e.g. per-IP/min) so 402 retry storms and
   bots are dropped at the edge — before any origin is billed.
5. **Railway trusted-proxy:** confirm real client IP survives. Cloudflare sets `CF-Connecting-IP`
   and prepends `x-forwarded-for`; `forwarded_client_ip()` takes the first `x-forwarded-for`
   entry, which becomes the real client IP once Railway's edge (private peer) is the trusted hop.
   Verify per-client MCP rate limiting works post-cutover (no longer pooled).

### Keep on the Vercel proxy (do NOT repoint)

`/api/*`, `/auth/*`, `/onboarding/*` stay same-origin through Vercel — auth is cookie-based and
same-origin is load-bearing. Repointing these to a cross-origin Railway host would break
session cookies/CORS. If their bandwidth becomes material, give Railway a first-party subdomain
(e.g. `api.onchain-ai.xyz`) and set cookies on the parent domain — a larger, separate change.

## Verification checklist

- [ ] `curl -sSI https://www.onchain-ai.xyz/mcp/okx` — after cutover shows Railway/Cloudflare, **no** `server: Vercel`.
- [ ] `GET /mcp/okx` still 402 x402 challenge (OKX endpoint review); unpaid `POST /mcp/okx` still 402 (OKX gate unaffected).
- [ ] Site pages still `server: Vercel` (unchanged).
- [ ] MCP rate limit keys on real client IP (hit from two IPs; only the offending one 429s).
- [ ] Vercel Usage: Edge Requests / Fast Data Transfer for `/mcp*` drop to ~0.
