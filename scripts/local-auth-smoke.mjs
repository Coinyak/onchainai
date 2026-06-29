/**
 * Local auth smoke — Playwright checks sign-in UI, OAuth redirect wiring, and
 * local session render/logout behavior.
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
 *   - On local URLs, the smoke also performs a temporary SIWX sign-in through
 *     /auth/siwx/challenge + /auth/siwx/verify, then verifies TopNav/profile
 *     menu/logout. Set ONCHAINAI_SKIP_SESSION_AUTH_SMOKE=1 to skip that part.
 *     If SUPABASE_URL + SUPABASE_SERVICE_KEY are available in env/.env, the
 *     temporary auth user is deleted afterward.
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
import { generateKeyPairSync, sign } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import {
  clearSidebarStorage,
  isBenignConsoleError,
  waitForSidebarStorageLoaded,
} from "./browser-test-helpers.mjs";

const base = (process.argv[2] || "http://localhost:3000").replace(/\/$/, "");
const errors = [];
const consoleErrors = [];

function fail(code, detail = "") {
  errors.push(detail ? `${code}:${detail}` : code);
}

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function watchConsole(page, label) {
  page.on("console", (msg) => {
    if (msg.type() !== "error") return;
    const text = msg.text();
    if (!isBenignConsoleError(text)) {
      consoleErrors.push(`${label}:${text.slice(0, 180)}`);
    }
  });
  page.on("pageerror", (error) => {
    consoleErrors.push(`${label}:pageerror:${String(error.message || error).slice(0, 180)}`);
  });
}

function loadDotEnv() {
  const envPath = new URL("../.env", import.meta.url);
  if (!existsSync(envPath)) return;
  const text = readFileSync(envPath, "utf8");
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#") || !line.includes("=")) continue;
    const idx = line.indexOf("=");
    const key = line.slice(0, idx).trim();
    let value = line.slice(idx + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (key && process.env[key] === undefined) {
      process.env[key] = value;
    }
  }
}

function sessionSmokeConfig() {
  let parsed;
  try {
    parsed = new URL(base);
  } catch {
    return { enabled: false, reason: "invalid base URL" };
  }
  const isLocal = ["localhost", "127.0.0.1", "::1"].includes(parsed.hostname);
  if (!isLocal) {
    return { enabled: false, reason: "non-local base URL" };
  }
  if (process.env.ONCHAINAI_SKIP_SESSION_AUTH_SMOKE === "1") {
    return { enabled: false, reason: "ONCHAINAI_SKIP_SESSION_AUTH_SMOKE=1" };
  }

  loadDotEnv();
  return {
    enabled: true,
    supabaseUrl: process.env.SUPABASE_URL?.replace(/\/$/, "") ?? "",
    serviceKey: process.env.SUPABASE_SERVICE_KEY ?? "",
  };
}

const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

function base58Encode(bytes) {
  if (!bytes.length) return "";
  const digits = [0];
  for (const byte of bytes) {
    let carry = byte;
    for (let i = 0; i < digits.length; i++) {
      const value = digits[i] * 256 + carry;
      digits[i] = value % 58;
      carry = Math.floor(value / 58);
    }
    while (carry > 0) {
      digits.push(carry % 58);
      carry = Math.floor(carry / 58);
    }
  }
  let encoded = "";
  for (const byte of bytes) {
    if (byte !== 0) break;
    encoded += BASE58_ALPHABET[0];
  }
  for (let i = digits.length - 1; i >= 0; i--) {
    encoded += BASE58_ALPHABET[digits[i]];
  }
  return encoded;
}

function generateSolanaIdentity() {
  const { publicKey, privateKey } = generateKeyPairSync("ed25519");
  const publicDer = publicKey.export({ type: "spki", format: "der" });
  const publicKeyBytes = publicDer.subarray(publicDer.length - 32);
  return {
    walletAddress: base58Encode(publicKeyBytes),
    signMessage(message) {
      return base58Encode(sign(null, Buffer.from(message), privateKey));
    },
  };
}

function jwtSub(token) {
  const payload = token.split(".")[1];
  if (!payload) return "";
  const padded = payload.padEnd(payload.length + ((4 - (payload.length % 4)) % 4), "=");
  try {
    return JSON.parse(Buffer.from(padded, "base64url").toString("utf8")).sub ?? "";
  } catch {
    return "";
  }
}

async function supabaseAdminFetch(config, path, options = {}) {
  if (!config.supabaseUrl || !config.serviceKey) {
    return null;
  }
  const attempts = options.retry === false ? 1 : 4;
  let lastResponse = null;
  for (let attempt = 1; attempt <= attempts; attempt++) {
    const headers = {
      apikey: config.serviceKey,
      authorization: `Bearer ${config.serviceKey}`,
      ...(options.body ? { "content-type": "application/json" } : {}),
      ...(options.headers || {}),
    };
    const response = await fetch(`${config.supabaseUrl}${path}`, {
      ...options,
      headers,
    });
    if (response.ok) {
      return response;
    }
    lastResponse = response;
    if (![429, 500, 502, 503, 504].includes(response.status) || attempt === attempts) {
      break;
    }
    await delay(500 * attempt);
  }
  return lastResponse;
}

async function cleanupSessionUser(config, userId) {
  if (!userId) return;
  await supabaseAdminFetch(config, `/auth/v1/admin/users/${encodeURIComponent(userId)}`, {
    method: "DELETE",
  }).catch(() => {});
}

async function runSessionAuthSmoke(browser) {
  const config = sessionSmokeConfig();
  if (!config.enabled) {
    console.log(`session-auth: skipped (${config.reason})`);
    return;
  }

  let userId = "";
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  try {
    const identity = generateSolanaIdentity();
    const challengeRes = await context.request.post(`${base}/auth/siwx/challenge`, {
      data: { wallet_address: identity.walletAddress, chain_id: "solana" },
    });
    if (!challengeRes.ok()) {
      fail("session-auth-challenge", `${challengeRes.status()} ${await challengeRes.text()}`);
      return;
    }
    const challenge = await challengeRes.json();
    const signature = identity.signMessage(challenge.message);
    const verifyRes = await context.request.post(`${base}/auth/siwx/verify`, {
      data: { nonce: challenge.nonce, signature },
    });
    if (!verifyRes.ok()) {
      fail("session-auth-verify", `${verifyRes.status()} ${await verifyRes.text()}`);
      return;
    }

    const cookies = await context.cookies(base);
    const accessCookie = cookies.find((cookie) => cookie.name === "onchainai_access_token");
    const hintCookie = cookies.find((cookie) => cookie.name === "onchainai_session");
    if (!accessCookie || !hintCookie) {
      fail("session-auth-missing-cookies", cookies.map((cookie) => cookie.name).join(","));
      return;
    }
    if (accessCookie.sameSite !== "Lax" || hintCookie.sameSite !== "Lax") {
      fail(
        "session-auth-cookie-samesite",
        `access=${accessCookie.sameSite} hint=${hintCookie.sameSite}`,
      );
    }
    userId = jwtSub(accessCookie.value);

    const authPage = await context.newPage();
    watchConsole(authPage, "session-auth");
    await authPage.goto(`${base}/`, { waitUntil: "networkidle" });
    await authPage.waitForSelector('[data-testid="auth-signed-in"]', { timeout: 8000 });
    const signedInState = await authPage.evaluate(() => ({
      hasSignedIn: !!document.querySelector('[data-testid="auth-signed-in"]'),
      hasSignIn: !!document.querySelector('[data-testid="auth-sign-in"]'),
      hasProfileButton: !!document.querySelector('[data-testid="profile-menu-btn"]'),
      profileLabel:
        document.querySelector('[data-testid="profile-menu-btn"]')?.getAttribute("aria-label") ??
        "",
    }));
    if (!signedInState.hasSignedIn || signedInState.hasSignIn || !signedInState.hasProfileButton) {
      fail("session-auth-topnav", JSON.stringify(signedInState));
      return;
    }
    if (!signedInState.profileLabel.includes("Account menu")) {
      fail("session-auth-profile-label", signedInState.profileLabel);
    }

    await authPage.click('[data-testid="profile-menu-btn"]');
    await authPage.waitForSelector('[data-testid="profile-menu-dropdown"]', { timeout: 5000 });
    const menuState = await authPage.evaluate(() => ({
      hasDashboard: !!document.querySelector('[data-testid="profile-menu-dashboard"]'),
      hasToolkit: !!document.querySelector('[data-testid="profile-menu-toolkit"]'),
      hasAdmin: !!document.querySelector('[data-testid="profile-menu-admin"]'),
      hasSignOut: !!document.querySelector('[data-testid="profile-menu-sign-out"]'),
      expanded: document
        .querySelector('[data-testid="profile-menu-btn"]')
        ?.getAttribute("aria-expanded"),
    }));
    if (
      !menuState.hasDashboard ||
      !menuState.hasToolkit ||
      !menuState.hasSignOut ||
      menuState.expanded !== "true"
    ) {
      fail("session-auth-profile-menu", JSON.stringify(menuState));
    } else {
      console.log("session-auth: signed-in TopNav and profile menu ok");
    }

    await Promise.all([
      authPage.waitForNavigation({ waitUntil: "networkidle", timeout: 8000 }).catch(() => {}),
      authPage.click('[data-testid="profile-menu-sign-out"]'),
    ]);
    await authPage.waitForSelector('[data-testid="auth-sign-in"]', { timeout: 8000 });
    const signedOutState = await authPage.evaluate(() => ({
      hasSignIn: !!document.querySelector('[data-testid="auth-sign-in"]'),
      hasSignedIn: !!document.querySelector('[data-testid="auth-signed-in"]'),
      hasProfileMenu: !!document.querySelector('[data-testid="profile-menu"]'),
    }));
    if (!signedOutState.hasSignIn || signedOutState.hasSignedIn || signedOutState.hasProfileMenu) {
      fail("session-auth-logout-nav", JSON.stringify(signedOutState));
    }
    const remainingCookies = await context.cookies(base);
    const authCookies = remainingCookies.filter((cookie) =>
      ["onchainai_access_token", "onchainai_session"].includes(cookie.name),
    );
    if (authCookies.length) {
      fail("session-auth-logout-cookies", authCookies.map((cookie) => cookie.name).join(","));
    } else {
      console.log("session-auth: logout clears nav and cookies");
    }
  } catch (error) {
    fail("session-auth-error", String(error.message || error));
  } finally {
    await context.close().catch(() => {});
    await cleanupSessionUser(config, userId);
  }
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
watchConsole(page, "signed-out");

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

// --- Optional local session regression smoke --------------------------------
await runSessionAuthSmoke(browser);

await browser.close();

if (consoleErrors.length) {
  fail("console-errors", consoleErrors.slice(0, 4).join(" | "));
}

if (errors.length) {
  console.error("LOCAL AUTH SMOKE FAIL");
  console.error(errors.join("\n"));
  process.exit(1);
}

console.log(`LOCAL AUTH SMOKE PASS ${base}`);
