# Auth, Admin, Navigation, And Preview Stability Final Spec

> Related docs: [[../../SECURITY]] | [[../../BUILD_DEPLOY_RULES]] | [[../../UI_UX_DESIGN]] | [[../../OPERATOR_GUIDE]] | [[../../../AGENTS.md]]
>
> Date: 2026-06-30
> Status: Final spec ready for implementation planning
> Scope: GitHub/email/wallet login behavior, operator authorization, profile navigation, logout visibility, production environment parity, and tool preview scroll stability

---

## 0. Session Summary

This spec consolidates the issues observed during the 2026-06-30 debugging session:

- GitHub login appears incomplete until the user refreshes.
- After login, the profile menu does not reliably show Dashboard, My Toolkit, Admin, or Logout actions.
- An operator account cannot access operator features.
- Clicking a tool such as `BOB Gateway CLI` opens the side preview but the tools page jumps back to the top as if it refreshed.
- Local development and the deployed website are not guaranteed to behave the same because code, database, OAuth callback, and environment variables can differ.
- User login and operator login share the same session mechanism; the operator difference is authorization, not a separate login flow.

No implementation is included in this document. It defines the target behavior, security constraints, acceptance criteria, and suggested implementation slices.

---

## 1. Product Goal

Authenticated users should feel signed in immediately after login, operators should gain the correct server-side permissions without manual refreshes or hidden UI states, and tool preview interactions should not disturb the user's browsing position.

The site must preserve a clear distinction:

- **Authentication** answers: who is this user?
- **Authorization** answers: is this authenticated user allowed to perform this action?
- **Navigation/UI state** answers: what actions should the authenticated user see?

Admin/operator privilege must never depend only on client-side state, URL state, cached UI, GitHub public profile fields, or user-editable metadata.

---

## 2. Non-Goals

This spec does not introduce:

- A separate operator login page.
- A separate operator account type outside the existing `profiles.is_admin` model.
- Custody, x402 payment execution, facilitator behavior, split payments, or fund movement.
- A redesign of the full application shell.
- New public visibility for pending, rejected, quarantined, or internal review data.
- Automatic CI, review bots, or deployment.

---

## 3. Current Architecture Assumptions

### 3.1 User Session Model

All login methods eventually produce the same application session shape:

- GitHub OAuth
- Email magic link
- SIWX wallet login

The server reads the signed session cookie, loads the matching profile from the database, and derives current user state from that profile. `profiles.is_admin` and `profiles.is_banned` are database-backed authorization fields.

Relevant files:

- `src/auth/session_ssr.rs`
- `src/auth/routes.rs`
- `src/auth/email.rs`
- `src/auth/siwx.rs`
- `src/auth/guard.rs`

### 3.2 Operator Model

An operator is a normal authenticated user whose profile has:

```text
profiles.is_admin = true
```

Operator-only server functions and routes must call the existing admin guard or an equivalent server-side guard. Showing or hiding an Admin link in the UI is not sufficient authorization.

### 3.3 GitHub Operator Bootstrap

GitHub login can auto-promote matching GitHub usernames through `ADMIN_GITHUB_LOGINS`.

The production Railway variable check during this session showed:

```text
ADMIN_GITHUB_LOGINS = missing
GITHUB_REDIRECT_URI = missing
SIWX_SESSION_TTL = present
SIWX_DOMAIN = present
JWT_SECRET = present
SUPABASE_SERVICE_KEY = present
ONCHAINAI_RELAX_RATE_LIMIT = missing
```

The deploy script also did not appear to sync `ADMIN_GITHUB_LOGINS`, so the variable can remain missing after deploys unless explicitly handled.

---

## 4. Required User-Facing Behavior

### 4.1 Login Completion

After successful GitHub, email, or wallet login:

- The user lands on the intended destination or onboarding route without needing a manual refresh.
- The navigation shell reflects the signed-in state on the first rendered post-login page.
- Signed-in-only actions become available without a reload.
- If onboarding is required, the user is sent to onboarding consistently.
- If the user is banned or the profile is invalid, protected mutations fail closed.

Acceptance criteria:

- A GitHub login smoke test can complete and observe signed-in navigation state without `page.reload()`.
- Email and SIWX smoke tests observe the same signed-in navigation state.
- The SSR HTML and hydrated DOM do not disagree about whether the user is signed in.

### 4.2 Profile Menu

For a signed-in normal user, the profile menu must expose:

- Dashboard
- My Toolkit
- Sign out

For a signed-in operator, the profile menu must also expose:

- Admin

The menu must:

- Be reachable by mouse and keyboard.
- Remain visible outside the nav action container.
- Not be clipped by horizontal or vertical overflow rules.
- Not rely on refresh to populate its items.
- Preserve existing `data-testid`s and route links unless a deliberate test migration is included.

Acceptance criteria:

- Clicking the profile button opens a visible menu at desktop and mobile widths.
- Dashboard, My Toolkit, and Sign out are visible for authenticated users.
- Admin is visible only when the current server-loaded session has `is_admin = true`.
- The Sign out action returns the nav to signed-out state without manual refresh.

### 4.3 Operator Access

Operator access must be granted only when the server-loaded profile has `is_admin = true`.

Acceptance criteria:

- A non-admin authenticated user receives a denied response for admin routes and admin server functions.
- An admin authenticated user can access admin pages and admin server functions.
- If `is_admin` is revoked in the database, admin access is lost on the next guarded request.
- Admin UI links are convenience only; direct URL access is still server-gated.

### 4.4 Tool Preview Selection

When a user clicks a tool card from the tools browser:

- The selected tool preview opens.
- The URL may update to include `selected=<slug>`.
- The current scroll position must not jump to the top.
- The tool list must not collapse into the skeleton state only because `selected` changed.
- Filters, sort, page, and search behavior must remain unchanged.

Acceptance criteria:

- From a scrolled tools page, selecting `bob-gateway-cli` keeps the viewport near the clicked card.
- The side preview becomes visible.
- No list-wide loading skeleton appears solely due to preview selection.
- Browser back/forward restores selection state without disorienting scroll jumps.

---

## 5. Security Requirements

### 5.1 Authorization Source Of Truth

Admin authorization must be based on server-side data that the user cannot edit directly.

Required:

- Continue to avoid `user_metadata` or other user-editable metadata for authorization.
- Continue using server-side guards for admin operations.
- Keep `profiles_public` limited to safe public profile fields.
- Keep private or sensitive fields out of public views.

### 5.2 `profiles` Update Hardening

The current self-update policy allows users to update their own profile row. This must be reviewed as a P0 security item because RLS restricts rows, not columns.

Risk:

- If the `authenticated` role has direct update access to `profiles` through the Supabase Data API, a normal user may be able to update sensitive columns on their own row, including `is_admin` or `is_banned`.

Required outcome:

- A normal user must never be able to set or clear `is_admin`.
- A normal user must never be able to set or clear `is_banned`.
- Profile self-editing, if allowed, must be limited to safe user-owned fields such as nickname, avatar, or bio.
- Sensitive profile mutations must happen only through server-side functions guarded by `require_admin` or an equivalent guard.

Verification criteria:

- Attempting to update `profiles.is_admin` as a normal authenticated user fails.
- Attempting to update `profiles.is_banned` as a normal authenticated user fails.
- Allowed self-profile fields still update successfully if the product supports them.

### 5.3 First-User-Admin Bootstrap

The first-user-admin trigger is useful for local bootstrap but dangerous for production if a database is empty or seeded in the wrong order. This risk is independent of `ADMIN_GITHUB_LOGINS`: GitHub, email, and SIWX signups can all create the first profile row, so any first signup can become admin while the trigger remains active.

Required outcome:

- Production operator bootstrap must be explicit and auditable.
- First-user-admin behavior must be disabled, gated, or documented as local-only.
- A production database must not accidentally promote the first random signup.
- The implementation plan must treat this as a standalone P0, not as a side effect of fixing `ADMIN_GITHUB_LOGINS`.

### 5.4 Logout And Session Invalidation

Current logout behavior appears to clear cookies. The security documentation expects stronger session invalidation through token/session revocation.

Required outcome:

- Logout must clear browser cookies reliably.
- For strict security, server-side session invalidation must be implemented or the docs must be corrected to reflect the actual guarantee.
- Admin revocation and ban decisions must continue to be checked against fresh server-side profile data on guarded requests.

Acceptance criteria:

- After logout, the profile menu is gone and signed-out nav appears without refresh.
- A logged-out browser cannot call authenticated server functions.
- If token blacklist or session-id validation is implemented, an old token cannot be reused after logout.

### 5.5 CSRF And Origin Defense

The security docs and several source comments mention Origin checks and CSRF token protection. Current code review found no CSRF token implementation and no explicit Origin-check middleware; CORS is not a CSRF defense. The effective current defense appears to be `SameSite=Lax` cookies, which helps for cross-site POST/subresource requests but must not be documented as if Origin checks or built-in Leptos CSRF tokens already exist.

Required outcome:

- State-changing routes and server functions must have a defined CSRF strategy.
- Admin mutations deserve the strictest protection.
- False or misleading CSRF/Origin claims in `docs/SECURITY.md` and source comments must be corrected as a P0 documentation/code-comment fix.
- If `SameSite=Lax` is the chosen v1 defense, document that precisely and verify all authenticated mutations use POST or stronger methods.
- If stronger protection is required, add explicit Origin/Referer validation for protected mutation routes and Leptos server-function POSTs before documenting it.

Acceptance criteria:

- Cross-site POST attempts to protected mutations fail.
- Same-origin protected mutations still work.
- The documented security posture matches the implemented behavior.
- Searching the repository no longer finds comments claiming non-existent CSRF tokens or Origin checks are the primary mutation defense.

### 5.6 Rate Limit Safety

Rate limiting must protect auth and public pages without causing false login failures, hydration failures, or smoke-test false positives.

The application already has separate auth, general, and MCP rate-limit configurations plus an `ONCHAINAI_RELAX_RATE_LIMIT` escape hatch. This work should tune and verify those existing layers instead of assuming rate limiting is currently one undifferentiated bucket.

Required outcome:

- Auth endpoints, callback endpoints, static assets, server functions, and public page hydration must be checked against the existing rate-limit layers.
- Static assets needed for hydration must not be throttled in a way that makes login state appear broken.
- Browser smoke tests must fail loudly on 429 responses.
- Production rate-limit configuration must be observable enough to debug.

Acceptance criteria:

- A normal login flow does not hit 429.
- Public smoke tests do not treat 429 as successful page rendering.
- Static assets needed for hydration are not blocked during normal page visits.

---

## 6. Environment And Deployment Requirements

### 6.1 Local Vs Production Truth

Local and production are not automatically synchronized.

The following may differ:

- Git checkout and built assets.
- Railway deployment revision.
- Supabase project and database contents.
- OAuth app callback URLs.
- Environment variables.
- Rate-limit settings.

Required outcome:

- Document and automate an auth/admin readiness check that compares required local and Railway variable names without printing secret values.
- Include `ADMIN_GITHUB_LOGINS` in deployment/environment synchronization if GitHub auto-promotion remains supported.
- Keep secrets out of logs and client code.

### 6.2 Canonical Domain Behavior

The production canonical domain is `https://www.onchain-ai.xyz`.

Observed during the session:

- `https://www.onchain-ai.xyz/auth/github` redirects to GitHub.
- `https://onchain-ai.xyz/` redirects to `www`.
- `https://onchain-ai.xyz/auth/github` returned 404.

The application code already contains a canonical host redirect middleware for `onchain-ai.xyz` to `www.onchain-ai.xyz`. Therefore this requirement is primarily a deployment, DNS, proxy, and smoke-test verification item, not a request to duplicate redirect logic in the app.

Required outcome:

- Confirm the existing canonical redirect middleware is present in the deployed revision.
- Confirm apex DNS/proxy routing sends all paths, including `/auth/github`, to the application before route matching.
- All generated auth links should continue to prefer the canonical `www` host.
- OAuth redirect URIs must match the canonical production callback.

Acceptance criteria:

- `https://onchain-ai.xyz/auth/github` reaches the same login flow as the `www` host or redirects to it.
- OAuth callback uses the configured canonical host.

---

## 7. Suggested Implementation Slices

### Slice A: Production Admin Bootstrap And Env Parity

Files likely involved:

- `src/config.rs`
- `scripts/deploy-railway.sh`
- `migrations/002_auth.sql`
- A new migration file
- `docs/OPERATOR_GUIDE.md`
- `docs/SECURITY.md`
- A small readiness script under `scripts/`

Deliverables:

- Required auth/admin env keys are checked without printing values.
- `ADMIN_GITHUB_LOGINS` is included in the deploy/env sync path if the feature stays.
- The first-user-admin trigger is disabled, gated, or made local-only for production.
- Operator bootstrap rules are documented.
- Production missing-env failures are actionable.

### Slice B: Profile RLS And Sensitive Column Hardening

Files likely involved:

- `migrations/002_auth.sql`
- A new migration file
- `src/server/functions.rs`
- `docs/SECURITY.md`

Deliverables:

- Normal users cannot modify sensitive profile fields.
- Safe profile edits still work.
- Admin-only profile changes remain guarded server-side.
- A local or staging verification query demonstrates the policy.

### Slice C: Auth State And Profile Menu Reliability

Files likely involved:

- `src/components/top_nav.rs`
- `style/output.css`
- `src/auth/routes.rs`
- `scripts/local-auth-smoke.mjs`
- `scripts/browser-smoke.mjs`

Deliverables:

- Signed-in navigation appears immediately after login.
- Profile dropdown is not clipped.
- Dashboard, My Toolkit, Admin, and Sign out visibility matches auth state.
- Logout updates the UI without refresh.

### Slice D: Tool Preview Without Scroll Jump

Files likely involved:

- `src/components/tools_browser.rs`
- `src/components/tool_card.rs`
- `src/server/functions.rs`
- `scripts/click-test.mjs`

Deliverables:

- Changing `selected` does not invalidate the whole browser list payload.
- Preview data can load without replacing the list with skeletons.
- Scroll position is preserved across selection changes.
- Back/forward navigation remains useful.

### Slice E: Rate Limit, Canonical Host, And Smoke Tests

Files likely involved:

- `src/lib.rs`
- `scripts/browser-smoke.mjs`
- `railway.json`
- `docs/BUILD_DEPLOY_RULES.md`

Deliverables:

- 429 responses fail smoke tests.
- Existing auth/general/MCP rate-limit layers are tuned and verified rather than duplicated.
- Auth and asset hydration are not accidentally throttled in normal browsing.
- The existing canonical host redirect is verified in the deployed revision.
- Apex DNS/proxy routing is verified for auth paths, not only `/`.
- Production smoke tests cover the login-adjacent shell behavior.

---

## 8. Test Matrix

### 8.1 Local Commands

Use the repository workflow:

```bash
./scripts/dev-watch.sh
```

Before handoff for UI/auth/routing changes:

```bash
./scripts/ui-change-gate.sh
```

For non-UI compile confidence when relevant:

```bash
cargo check --features ssr
```

### 8.2 Browser And Auth Smoke

Required scenarios:

- Signed-out home shows signed-out nav.
- GitHub login lands with signed-in nav without manual refresh.
- Email login lands with signed-in nav without manual refresh.
- SIWX login lands with signed-in nav without manual refresh.
- Profile menu opens and shows expected items.
- Logout returns to signed-out nav without manual refresh.
- Normal user cannot see or call admin features.
- Operator can see and call admin features.
- Tool preview selection preserves scroll.
- Production smoke fails on 429.

### 8.3 Security Verification

Required checks:

- Normal authenticated user cannot update `profiles.is_admin`.
- Normal authenticated user cannot update `profiles.is_banned`.
- The sensitive-column checks cover GitHub, email, and SIWX-created profiles.
- Admin user can perform intended user-management actions.
- Admin user cannot accidentally remove their own last admin access if the existing product rule forbids it.
- Cross-site mutation attempts fail according to the chosen CSRF strategy.
- Secrets are never printed in readiness checks.

---

## 9. Priority Order

### P0

1. Confirm and harden `profiles` RLS/update permissions for sensitive columns.
2. Disable, gate, or make local-only the first-user-admin trigger in production.
3. Fix production operator bootstrap by setting and syncing `ADMIN_GITHUB_LOGINS` or replacing it with an explicit admin bootstrap process.
4. Correct false CSRF/Origin claims in security docs and source comments.
5. Ensure operator access is server-side and works immediately after login.
6. Make profile menu actions visible and unclipped after login.
7. Prevent tool preview selection from resetting scroll or collapsing the list.

### P1

1. Align logout/session invalidation docs and implementation.
2. Add explicit Origin/Referer validation for protected mutations if the project chooses stronger CSRF defense than `SameSite=Lax`.
3. Tune and verify existing rate-limit layers so login and hydration do not false-fail.
4. Verify deployed apex-domain auth path behavior through the existing canonical redirect and DNS/proxy routing.
5. Add smoke coverage for login state, profile menu, admin visibility, and preview scroll.

### P2

1. Add local/prod env parity diagnostics.
2. Add operator audit logging for role, ban, and delete actions if not already covered.
3. Document the fast local server startup convention using detached `screen` plus `./scripts/dev-watch.sh`.

---

## 10. Definition Of Done

The work is complete when:

- A fresh login no longer needs refresh to show authenticated state.
- Normal users and operators share one login model but have correct server-side authorization separation.
- Operator accounts can access operator features only when `profiles.is_admin = true`.
- Profile menu reliably shows Dashboard, My Toolkit, Sign out, and Admin when appropriate.
- Logout is visible and works without manual refresh.
- Tool preview opens without jumping the page to the top.
- Production has the required operator bootstrap environment or a replacement bootstrap flow.
- Production cannot accidentally promote the first signup through first-user-admin behavior.
- Sensitive profile fields cannot be changed by normal users through direct client/API access.
- Security documentation and source comments match actual CSRF/Origin behavior.
- The UI/auth/routing gate passes, or any remaining failures are documented with exact commands and output.
