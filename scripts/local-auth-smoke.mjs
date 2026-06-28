/**
 * Local auth smoke — Playwright checks for sign-in UI and OAuth redirect wiring.
 *
 * Usage:
 *   node scripts/local-auth-smoke.mjs
 *   node scripts/local-auth-smoke.mjs http://localhost:3000
 *
 * Prerequisites:
 *   - Dev server running (`cargo leptos watch` or similar) on port 3000
 *   - `npm i -g playwright` or project-local playwright (see browser-smoke.mjs)
 *   - For wallet button to be interactive (not link fallback): `cargo leptos build`
 *     so the WASM hydrate bundle is served at /pkg/
 *
 * Manual: new GitHub account (incognito + separate OAuth app)
 * ----------------------------------------------------------------
 * 1. Create a new GitHub account (or use one never linked to OnchainAI).
 * 2. Open an incognito/private window so no existing session cookies leak in.
 * 3. Register a separate GitHub OAuth App (Settings → Developer settings →
 *    OAuth Apps → New OAuth App):
 *      - Homepage URL: http://localhost:3000
 *      - Authorization callback URL: http://localhost:3000/auth/callback
 * 4. Set GITHUB_CLIENT_ID / GITHUB_CLIENT_SECRET in .env for this app.
 * 5. Restart the local server, click "Continue with GitHub" in the modal.
 * 6. Approve as the new GitHub user — first login should redirect to
 *    /onboarding/profile (post_auth_redirect_path for users without nickname).
 *
 * Manual: new wallet (MetaMask account switch + SIWX)
 * ---------------------------------------------------
 * 1. Ensure WASM hydrate is built (`cargo leptos build`); wallet button must
 *    show data-testid="wallet-sign-in" (button), not wallet-sign-in-link.
 * 2. Install MetaMask (or any EIP-1193 provider) and unlock it.
 * 3. Create/import a fresh account in MetaMask (Account menu → Add account).
 * 4. In the sign-in modal click "Connect Wallet (SIWX)" — MetaMask prompts
 *    eth_requestAccounts → personal_sign on /auth/siwx/challenge message.
 * 5. On success the app navigates to verify.redirect (first-time:
 *    /onboarding/profile). Switch MetaMask accounts and repeat to test another
 *    wallet identity without clearing cookies (each address = new user).
 */
import { chromium } from "playwright";
import {
  clearSidebarStorage,
  waitForSidebarStorageLoaded,
} from "./browser-test-helpers.mjs";

const base = (process.argv[2] || "http://localhost:3000").replace(/\/$/, "");
const errors = [];

function fail(code, detail = "") {
  errors.push(detail ? `${code}:${detail}` : code);
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });

// --- Home: open sign-in modal ------------------------------------------------
await page.goto(`${base}/`, { waitUntil: "networkidle" });

const signedOutNav = await page.evaluate(() => ({
  hasSignIn: !!document.querySelector('[data-testid="top-nav-sign-in"]'),
  hasProfileMenu: !!document.querySelector('[data-testid="profile-menu"]'),
  hasTopNavDashboardLink: !!document.querySelector(".site-top-nav-link-dashboard"),
  hasTopNavToolkitLink: !!document.querySelector(".site-top-nav-link-toolkit"),
}));
if (!signedOutNav.hasSignIn) {
  fail("auth-missing-sign-in-button");
}
if (signedOutNav.hasProfileMenu) {
  fail("auth-unexpected-profile-menu");
}
if (signedOutNav.hasTopNavDashboardLink) {
  fail("auth-unexpected-top-nav-dashboard");
}
if (signedOutNav.hasTopNavToolkitLink) {
  fail("auth-unexpected-top-nav-toolkit");
}

const signInBtn = await page.$('[data-testid="top-nav-sign-in"]');
if (!signInBtn) {
  fail("auth-missing-sign-in-button");
} else {
  await signInBtn.click();
  await page.waitForSelector('[role="dialog"]', { timeout: 5000 }).catch(() => {
    fail("auth-modal-not-open");
  });

  const modal = await page.evaluate(() => {
    const dialog = document.querySelector('[role="dialog"]');
    if (!dialog) {
      return { open: false };
    }
    return {
      open: true,
      hasGitHub: !!dialog.querySelector('a[href="/auth/github"]'),
      githubText: dialog.querySelector('a[href="/auth/github"]')?.textContent?.trim() ?? "",
      hasEmail: !!dialog.querySelector('input[type="email"]'),
      hasWallet: !!dialog.querySelector(
        '[data-testid="wallet-sign-in"], [data-testid="wallet-sign-in-link"]',
      ),
      walletIsButton: !!dialog.querySelector('[data-testid="wallet-sign-in"]'),
      walletIsLink: !!dialog.querySelector('[data-testid="wallet-sign-in-link"]'),
      title: document.querySelector("#login-title")?.textContent?.trim() ?? "",
    };
  });

  if (!modal.open) {
    fail("auth-modal-not-open");
  } else {
    if (!modal.hasGitHub) fail("auth-modal-missing-github");
    if (!modal.hasEmail) fail("auth-modal-missing-email");
    if (!modal.hasWallet) fail("auth-modal-missing-wallet");
    if (!modal.title.includes("Sign in")) {
      fail("auth-modal-missing-title", modal.title);
    }
    console.log(
      `modal: github=${modal.hasGitHub} email=${modal.hasEmail} wallet=${modal.hasWallet}` +
        ` (hydrated=${modal.walletIsButton}, ssr-link=${modal.walletIsLink})`,
    );
  }

  await page.keyboard.press("Escape");
}

// --- /auth/github: 307 + localhost callback (no real OAuth) ------------------
const githubRes = await page.request.get(`${base}/auth/github`, {
  maxRedirects: 0,
});
const status = githubRes.status();
const location = githubRes.headers()["location"] ?? "";

if (status !== 307) {
  fail("auth-github-status", String(status));
}
if (!location.includes("github.com/login/oauth/authorize")) {
  fail("auth-github-location", location.slice(0, 120));
}
const callbackMatch = location.match(/redirect_uri=([^&]+)/);
const decodedCallback = callbackMatch
  ? decodeURIComponent(callbackMatch[1])
  : "";
const isLocalCallback =
  decodedCallback.includes("localhost") || decodedCallback.includes("127.0.0.1");
if (!isLocalCallback || !decodedCallback.includes("/auth/callback")) {
  fail("auth-github-callback", decodedCallback || "missing redirect_uri");
} else {
  console.log(`auth/github: ${status} → callback=${decodedCallback}`);
}

// --- Sidebar toggle on /tools (if present) -----------------------------------
await page.goto(`${base}/tools`, { waitUntil: "domcontentloaded" });
await clearSidebarStorage(page);
await page.reload({ waitUntil: "networkidle" });
await waitForSidebarStorageLoaded(page).catch(() => {
  fail("sidebar-hydration-timeout");
});

const railToggle = await page.$(".sidebar-rail-toggle");
if (!railToggle) {
  console.log("sidebar: no .sidebar-rail-toggle on /tools — skipped");
} else {
  const before = await page.evaluate(() => {
    const aside = document.querySelector(".tools-sidebar");
    if (!aside) return null;
    const rect = aside.getBoundingClientRect();
    return {
      collapsed: aside.classList.contains("tools-sidebar-collapsed"),
      width: rect.width,
      expanded: aside.getAttribute("aria-expanded"),
    };
  });

  if (!before) {
    fail("sidebar-missing-aside");
  } else {
    await railToggle.click();
    await page.waitForTimeout(300);

    const after = await page.evaluate(() => {
      const aside = document.querySelector(".tools-sidebar");
      if (!aside) return null;
      const rect = aside.getBoundingClientRect();
      return {
        collapsed: aside.classList.contains("tools-sidebar-collapsed"),
        width: rect.width,
      };
    });

    const classChanged = before.collapsed !== after.collapsed;
    const widthChanged = Math.abs(before.width - after.width) > 8;
    if (!classChanged && !widthChanged) {
      fail(
        "sidebar-toggle-no-op",
        `before=${JSON.stringify(before)} after=${JSON.stringify(after)}`,
      );
    } else {
      console.log(
        `sidebar-toggle: classChanged=${classChanged} width ${before.width.toFixed(0)}→${after.width.toFixed(0)}`,
      );
    }
  }
}

await browser.close();

if (errors.length) {
  console.error("LOCAL AUTH SMOKE FAIL");
  console.error(errors.join("\n"));
  process.exit(1);
}

console.log(`LOCAL AUTH SMOKE PASS ${base}`);