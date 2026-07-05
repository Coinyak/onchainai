// harness-round-11: browser smoke for desktop/mobile public UI
import { chromium } from "playwright";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  TOOL_PAGE_SIZE,
  expectedCumulativeMin,
  sleep,
  NAV_PACE_MS,
  clearSidebarStorage,
  probeLogoFallback,
  evaluateLogoFallback,
  isBenignConsoleError,
  isBenignRequestFailure,
  visiblePageText,
  waitForToolCards,
  waitForSidebarStorageLoaded,
} from "./browser-test-helpers.mjs";

export const hydrationPanicRe =
  /hydration|entered unreachable code|panicked at|reactive value disposed/i;

function countRealToolCards(page) {
  return page.evaluate(
    () => document.querySelectorAll(".tool-card:not(.skeleton-card)").length,
  );
}

async function requireToolCards(page, errors, context) {
  try {
    await waitForToolCards(page);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    errors.push(`hydration:wait-for-tool-cards:${context}:${message}`);
    return 0;
  }
  const count = await countRealToolCards(page);
  if (count === 0) {
    errors.push(`layout:no-tool-cards:${context}`);
  }
  return count;
}

export function attachBrowserSmokeHandlers(page, base, errors) {
  page.on("console", (msg) => {
  const text = msg.text();
  if (hydrationPanicRe.test(text)) {
    errors.push(`hydration-panic:${text}`);
  }
  if (msg.type() === "error" && !isBenignConsoleError(text)) {
    errors.push(`console:error:${text}`);
  }
});
page.on("requestfailed", (req) => {
  const url = req.url();
  if (url.includes("fonts.gstatic.com") || url.includes("fonts.googleapis.com")) {
    return;
  }
  if (!url.startsWith(base)) {
    return;
  }
  const failText = req.failure()?.errorText ?? "";
  if (isBenignRequestFailure(url, failText)) {
    return;
  }
  errors.push(`requestfailed:${url}:${failText}`);
});
page.on("response", async (res) => {
  const url = res.url();
  if (!url.startsWith(base)) {
    return;
  }
  if (url.includes("/api")) {
    if (res.status() >= 400) {
      errors.push(`http:${res.status()}:${url}`);
    }
    const contentType = res.headers()["content-type"] || "";
    const isText =
      contentType.includes("json") ||
      contentType.includes("text") ||
      contentType.includes("javascript");
    if (isText) {
      const text = await res.text().catch(() => "");
      if (/error deserializing|missing field/i.test(text)) {
        errors.push(`body-error:${url}:${text.slice(0, 200)}`);
      }
    }
  }
});
}

export async function runBrowserSmokeChecks(page, base, errors) {
await page.goto(`${base}/`, { waitUntil: "domcontentloaded", timeout: 60000 });
await clearSidebarStorage(page);

// Wait for auth controls to hydrate before opening the sign-in modal.
await page
  .waitForSelector('[data-testid="top-nav-sign-in"]', { timeout: 30000 })
  .catch(() => {});
const homeLayout = await page.evaluate(() => ({
  hasTopNav: !!document.querySelector(".site-top-nav"),
  hasCategoryGrid: !!document.querySelector(".category-grid"),
  hasAuthSignIn: !!document.querySelector('[data-testid="auth-sign-in"]'),
  hasTopNavSignIn: !!document.querySelector('[data-testid="top-nav-sign-in"]'),
  hasProfileMenu: !!document.querySelector('[data-testid="profile-menu"]'),
  hasTopNavDashboardLink: !!document.querySelector(".site-top-nav-link-dashboard"),
  hasTopNavToolkitLink: !!document.querySelector(".site-top-nav-link-toolkit"),
}));
if (!homeLayout.hasTopNav) {
  errors.push("layout:home-missing-top-nav");
}
if (homeLayout.hasCategoryGrid) {
  errors.push("layout:home-unexpected-category-grid");
}
if (!homeLayout.hasAuthSignIn) {
  errors.push("layout:home-missing-auth-sign-in");
}
if (!homeLayout.hasTopNavSignIn) {
  errors.push("layout:home-missing-top-nav-sign-in");
}
const authSlotPresent = await page.evaluate(
  () => !!document.querySelector(".site-top-nav-auth"),
);
if (!authSlotPresent) {
  errors.push("layout:home-missing-auth-slot");
}
if (homeLayout.hasProfileMenu) {
  errors.push("layout:home-unexpected-profile-menu");
}
if (homeLayout.hasTopNavDashboardLink) {
  errors.push("layout:home-unexpected-top-nav-dashboard");
}
if (homeLayout.hasTopNavToolkitLink) {
  errors.push("layout:home-unexpected-top-nav-toolkit");
}

const hasSignInBeforeClick = await page.$('[data-testid="top-nav-sign-in"]');
if (!hasSignInBeforeClick) {
  errors.push("interaction:top-nav-sign-in-missing-before-click");
} else {
  await hasSignInBeforeClick.click();
  await page
    .waitForSelector('[role="dialog"]', { timeout: 5000 })
    .catch(() => errors.push("interaction:top-nav-sign-in-no-modal"));
}
const signInModal = await page.evaluate(() => {
  const dialog = document.querySelector('[role="dialog"]');
  if (!dialog) {
    return { open: false, hasGitHub: false, hasWallet: false };
  }
  return {
    open: true,
    hasGitHub: !!dialog.querySelector('a[href="/auth/github"]'),
    hasWallet: !!dialog.querySelector(
      '[data-testid="wallet-sign-in"], [data-testid="wallet-sign-in-link"]',
    ),
  };
});
if (!signInModal.open) {
  errors.push("interaction:sign-in-modal-missing");
} else {
  if (!signInModal.hasGitHub) {
    errors.push("interaction:sign-in-modal-missing-github");
  }
  const modalGitHubRel = await page.evaluate(() => {
    const dialog = document.querySelector('[role="dialog"]');
    const link = dialog?.querySelector('a[href="/auth/github"]');
    return link?.getAttribute("rel") ?? "";
  });
  if (!modalGitHubRel.includes("external")) {
    errors.push(`interaction:sign-in-modal-github-missing-rel-external:${modalGitHubRel}`);
  }
  if (!signInModal.hasWallet) {
    errors.push("interaction:sign-in-modal-missing-wallet");
  }
  const modalStacking = await page.evaluate(() => {
    const dialog = document.querySelector('[role="dialog"]');
    const header = document.querySelector(".site-top-nav");
    if (!dialog || !header) {
      return { ok: false, dialogZ: null, headerZ: null };
    }
    const dialogZ = Number.parseInt(getComputedStyle(dialog).zIndex, 10);
    const headerZ = Number.parseInt(getComputedStyle(header).zIndex, 10);
    return {
      ok: Number.isFinite(dialogZ) && Number.isFinite(headerZ) && dialogZ > headerZ,
      dialogZ,
      headerZ,
    };
  });
  if (!modalStacking.ok) {
    errors.push(
      `layout:modal-below-header:${modalStacking.dialogZ}:${modalStacking.headerZ}`,
    );
  }
}
await page.keyboard.press("Escape");

for (const path of ["/", "/tools", "/tools?type=mcp"]) {
  await page.goto(`${base}${path}`, { waitUntil: "domcontentloaded" });
  if (path.startsWith("/tools")) {
    await requireToolCards(page, errors, `route${path}`);
  }
  const text = await visiblePageText(page);
  if (/error deserializing|missing field/i.test(text || "")) {
    errors.push(`visible-error:${path}`);
  }
}

// Filter combo with zero results must render empty state (not hang on skeletons).
await page.goto(`${base}/tools?function=bridge&type=mcp`, {
  waitUntil: "domcontentloaded",
});
await page
  .waitForFunction(
    () => {
      const empty = !!document.querySelector(".empty-state-panel");
      const count = document.querySelector(".tool-count")?.textContent?.trim() ?? "";
      return empty || /\d+\s+tools/.test(count);
    },
    null,
    { timeout: 15000 },
  )
  .catch(() => {});
const emptyFilter = await page.evaluate(() => ({
  empty: !!document.querySelector(".empty-state-panel"),
  count: document.querySelector(".tool-count")?.textContent?.trim() ?? "",
}));
if (!emptyFilter.empty || emptyFilter.count !== "0 tools") {
  errors.push(
    `layout:empty-filter-state:empty=${emptyFilter.empty}:count=${emptyFilter.count}`,
  );
}

await page.setViewportSize({ width: 1280, height: 900 });
await page.goto(`${base}/`, { waitUntil: "domcontentloaded" });
await page.waitForSelector(".home-page h1", { timeout: 15000 }).catch(() => {});
const desktopH1Size = await page.evaluate(() => {
  const h1 = document.querySelector(".home-page h1");
  return h1 ? getComputedStyle(h1).fontSize : "";
});
await page.setViewportSize({ width: 375, height: 812 });
await page.goto(`${base}/`, { waitUntil: "domcontentloaded" });
await page.waitForSelector(".home-page h1", { timeout: 15000 }).catch(() => {});
const mobileH1Size = await page.evaluate(() => {
  const h1 = document.querySelector(".home-page h1");
  return h1 ? getComputedStyle(h1).fontSize : "";
});
if (!desktopH1Size || !mobileH1Size) {
  errors.push("computed-style:home-h1-missing");
} else if (desktopH1Size === mobileH1Size) {
  errors.push(`computed-style:home-h1-same:${desktopH1Size}`);
}

const hasHomeStyles = await page.evaluate(() => document.styleSheets.length > 0);
if (!hasHomeStyles) {
  errors.push("css-missing:home-stylesheets");
}

await page.setViewportSize({ width: 1280, height: 900 });
await page.goto(`${base}/tools`, { waitUntil: "domcontentloaded" });
await requireToolCards(page, errors, "tools-desktop");
const toolbarState = await page.evaluate(() => ({
  hasFilterRow: !!document.querySelector(".toolbar-filter-row"),
  hasVerified: [...document.querySelectorAll(".toolbar-sort-row a, .toolbar-filter-row a")]
    .some((a) => a.textContent?.trim() === "Verified"),
  hasOfficial: [...document.querySelectorAll(".toolbar-sort-row a, .toolbar-filter-row a")]
    .some((a) => a.textContent?.trim() === "Official"),
  hasMcp: [...document.querySelectorAll(".toolbar-filter-row a")]
    .some((a) => a.textContent?.trim() === "MCP"),
  hasCli: [...document.querySelectorAll(".toolbar-filter-row a")]
    .some((a) => a.textContent?.trim() === "CLI"),
}));
if (!toolbarState.hasFilterRow) errors.push("layout:tools-missing-toolbar-filter-row");
if (!toolbarState.hasVerified) errors.push("layout:tools-missing-verified-tab");
if (!toolbarState.hasOfficial) errors.push("layout:tools-missing-official-tab");
if (!toolbarState.hasMcp) errors.push("layout:tools-missing-mcp-tab");
if (!toolbarState.hasCli) errors.push("layout:tools-missing-cli-tab");
const toolsLogoStats = await page.evaluate(() => ({
  cards: document.querySelectorAll(".tool-card:not(.skeleton-card)").length,
  imgs: document.querySelectorAll("img.tool-logo-img").length,
}));
if (toolsLogoStats.cards >= 50 && toolsLogoStats.imgs === 0) {
  errors.push(`layout:tools-missing-logo-imgs:cards=${toolsLogoStats.cards}`);
}
const toolsLoadMore = await page.evaluate(() => {
  const cards = document.querySelectorAll(".tool-card:not(.skeleton-card)").length;
  const bodyLen = document.body?.innerHTML.length ?? 0;
  const hasLoadMore =
    !!document.querySelector(".load-more-btn") ||
    !!document.querySelector(".load-more-row");
  if (cards >= 50 || bodyLen > 20000) {
    return hasLoadMore;
  }
  return true;
});
if (!toolsLoadMore) {
  errors.push("layout:tools-missing-load-more");
}

const toolsCards = await countRealToolCards(page);
if (toolsCards >= 50) {
  const expectedPage2 = expectedCumulativeMin(toolsCards, 2);
  await sleep(NAV_PACE_MS);
  await page.goto(`${base}/tools?page=2`, { waitUntil: "domcontentloaded" });
  await requireToolCards(page, errors, "tools-page-2");
  const page2Text = await visiblePageText(page);
  const page2Cards = await countRealToolCards(page);
  if (/error deserializing|missing field/i.test(page2Text || "")) {
    errors.push("visible-error:/tools?page=2");
  }
  if (page2Cards < expectedPage2) {
    errors.push(`layout:page-2-too-few-cards:${page2Cards}<${expectedPage2}`);
  }

  await sleep(NAV_PACE_MS);
  await page.goto(`${base}/tools`, { waitUntil: "domcontentloaded" });
  await requireToolCards(page, errors, "tools-load-more");
  const before = await countRealToolCards(page);
  const loadMore = await page.$("a.load-more-btn, .load-more-row a.load-more-btn");
  if (!loadMore) {
    errors.push("interaction:tools-missing-load-more-button");
  } else {
    const loadMoreApiErrors = [];
    const onLoadMoreResponse = (res) => {
      const url = res.url();
      if (
        url.startsWith(base) &&
        (url.includes("/api") || url.includes("/pkg/")) &&
        res.status() >= 400
      ) {
        loadMoreApiErrors.push(`${res.status()}:${url}`);
      }
    };
    page.on("response", onLoadMoreResponse);
    await loadMore.click();
    let navTimedOut = false;
    await page.waitForURL(/[?&]page=2/, { timeout: 20000 }).catch(() => {
      navTimedOut = true;
    });
    await page.waitForLoadState("load").catch(() => {});
    // Wait for first hydrated card before counting — polling for 100 while still
    // on skeleton-only HTML can stall Leptos/WASM hydration (50->0 flake).
    await requireToolCards(page, errors, "tools-load-more-after");
    const expectedMin = expectedCumulativeMin(before, 2);
    let timedOut = false;
    await page
      .waitForFunction(
        (min) =>
          document.querySelectorAll(".tool-card:not(.skeleton-card)").length >= min,
        expectedMin,
        { timeout: 20000 },
      )
      .catch(() => {
        timedOut = true;
      });
    page.off("response", onLoadMoreResponse);
    const after = await countRealToolCards(page);
    if (navTimedOut || timedOut || after < expectedMin) {
      errors.push(`interaction:load-more-not-growing:${before}->${after}`);
    }
    if (loadMoreApiErrors.length) {
      errors.push(
        `interaction:load-more-api-errors:${loadMoreApiErrors.slice(0, 3).join("|")}`,
      );
    }
  }
}

if (toolsLogoStats.imgs > 0) {
  await page.goto(`${base}/tools`, { waitUntil: "domcontentloaded" });
  await requireToolCards(page, errors, "tools-logo-fallback");
  const brokeFallback = await probeLogoFallback(page);
  const logoEval = evaluateLogoFallback(brokeFallback);
  if (!logoEval.ok) {
    errors.push(`layout:tools-logo-fallback-missing:${logoEval.detail}`);
  }
}

await page.setViewportSize({ width: 375, height: 812 });
await page.goto(`${base}/tools`, { waitUntil: "domcontentloaded" });
await clearSidebarStorage(page);
await page.reload({ waitUntil: "domcontentloaded" });
await requireToolCards(page, errors, "mobile-tools");
await waitForSidebarStorageLoaded(page).catch(() => {
  errors.push("layout:mobile-sidebar-hydration-timeout");
});
const mobileSidebarLayout = await page.evaluate(() => {
  const header = document.querySelector(".site-top-nav");
  const aside = document.querySelector(".tools-sidebar");
  const main = document.querySelector(".tools-main");
  if (!aside || !main) {
    return { collapsed: false, asideWidth: 0, mainX: 0, mainY: 0, headerBottom: 0 };
  }
  const headerRect = header?.getBoundingClientRect();
  const asideRect = aside.getBoundingClientRect();
  const mainRect = main.getBoundingClientRect();
  return {
    collapsed: aside.classList.contains("tools-sidebar-collapsed"),
    asideWidth: asideRect.width,
    asideBottom: asideRect.bottom,
    mainX: mainRect.x,
    mainY: mainRect.y,
    headerBottom: headerRect?.bottom ?? 0,
  };
});
if (!mobileSidebarLayout.collapsed) {
  errors.push("layout:mobile-sidebar-not-collapsed");
}
if (mobileSidebarLayout.asideWidth < 320) {
  errors.push(`layout:mobile-sidebar-not-full-width:${mobileSidebarLayout.asideWidth}`);
}
if (mobileSidebarLayout.mainY < mobileSidebarLayout.asideBottom - 1) {
  errors.push(`layout:mobile-main-overlaps-sidebar:${mobileSidebarLayout.mainY}`);
}
if (mobileSidebarLayout.mainX !== 0) {
  errors.push(`layout:mobile-main-not-flush:${mobileSidebarLayout.mainX}`);
}

const bookmarkTouch = await page.evaluate(() => {
  const btn = document.querySelector(".tool-card:not(.skeleton-card) .card-action-btn");
  if (!btn) return { ok: true, missing: true };
  const rect = btn.getBoundingClientRect();
  return { ok: rect.width >= 44 && rect.height >= 44, width: rect.width, height: rect.height };
});
if (!bookmarkTouch.ok && !bookmarkTouch.missing) {
  errors.push(`computed-style:bookmark-touch-target:${bookmarkTouch.width}x${bookmarkTouch.height}`);
}

const railToggle = await page.$(".sidebar-rail-toggle");
if (railToggle) {
  await railToggle.click();
  const overlayState = await page.evaluate(() => ({
    expanded: !document.querySelector(".tools-sidebar")?.classList.contains("tools-sidebar-collapsed"),
    hasBackdrop: !!document.querySelector(".sidebar-mobile-backdrop"),
    scrollLocked: document.body.classList.contains("sidebar-scroll-locked"),
  }));
  if (!overlayState.expanded || !overlayState.hasBackdrop) {
    errors.push("interaction:mobile-filter-overlay-missing");
  }
  if (!overlayState.scrollLocked) {
    errors.push("interaction:mobile-filter-scroll-lock-missing");
  }
  const fnToggle = page.locator(".sidebar-section button.sidebar-toggle").first();
  if (await fnToggle.count()) {
    await fnToggle.click();
    await page
      .locator(".sidebar-section--open a[href*='function=']")
      .first()
      .waitFor({ state: "visible", timeout: 5000 })
      .catch(() => {});
  }
  const filterLink = page.locator(".sidebar-section--open a[href*='function=']").first();
  if (await filterLink.count()) {
    const filterHref = await filterLink.getAttribute("href");
    await filterLink.click({ force: true });
    if (filterHref) {
      await page.waitForURL((url) => url.pathname === "/tools" && url.search.includes("function="), {
        timeout: 10000,
      }).catch(() => {
        errors.push("interaction:mobile-filter-nav-timeout");
      });
    }
    let collapsedAfterFilter = false;
    try {
      await page.waitForFunction(
        () =>
          document
            .querySelector(".tools-sidebar")
            ?.classList.contains("tools-sidebar-collapsed") === true,
        null,
        { timeout: 10000 },
      );
      collapsedAfterFilter = true;
    } catch {
      collapsedAfterFilter = false;
    }
    if (!collapsedAfterFilter) {
      errors.push("interaction:mobile-filter-not-collapsed-after-click");
    }
  }
  await page.goto(`${base}/tools`, { waitUntil: "domcontentloaded" });
  await clearSidebarStorage(page);
  await page.reload({ waitUntil: "domcontentloaded" });
  await requireToolCards(page, errors, "mobile-reentry");
  const collapsedReentry = await page.evaluate(() =>
    document.querySelector(".tools-sidebar")?.classList.contains("tools-sidebar-collapsed"),
  );
  if (!collapsedReentry) {
    errors.push("layout:mobile-sidebar-not-collapsed-on-reentry");
  }
}

await page.setViewportSize({ width: 1280, height: 900 });
await page.goto(`${base}/tools`, { waitUntil: "domcontentloaded", timeout: 60000 });
await requireToolCards(page, errors, "desktop-interactions");
const chainMore = await page.$(".chain-tile-more");
if (chainMore) {
  await chainMore.click();
  await sleep(400);
  const chainExpanded = await page.evaluate(() => {
    const pill = document.querySelector(".chain-tile-more");
    return pill?.getAttribute("aria-expanded") === "true";
  });
  if (!chainExpanded) {
    errors.push("interaction:chain-more-not-expanded");
  }
}
const bookmarkBtn = await page.$(".tool-card:not(.skeleton-card) .card-action-btn");
if (bookmarkBtn) {
  await bookmarkBtn.click({ timeout: 10000 });
  await sleep(500);
  const bookmarkModal = await page.$('[role="dialog"]');
  if (!bookmarkModal) {
    errors.push("interaction:bookmark-no-login-modal");
  }
  await page.keyboard.press("Escape");
}

await page.setViewportSize({ width: 375, height: 812 });
await page.goto(`${base}/tools`, { waitUntil: "domcontentloaded" });
await requireToolCards(page, errors, "mobile-chain-strip");
const chainStripState = await page.evaluate(() => {
  const strip = document.querySelector(".chain-strip");
  if (!strip) {
    return { hasStrip: false, pillVisible: true };
  }
  const pill = strip.querySelector(".chain-tile-more");
  if (!pill) {
    // STRIP_PRIMARY_VISIBLE=20 leaves overflow when catalog > 20; no pill only if all fit.
    return { hasStrip: true, pillVisible: true, hasOverflowPill: false };
  }
  const style = getComputedStyle(pill);
  return {
    hasStrip: true,
    pillVisible: style.display !== "none" && style.visibility !== "hidden",
    hasOverflowPill: true,
  };
});
if (!chainStripState.hasStrip) {
  errors.push("layout:mobile-chain-strip-missing");
}
if (chainStripState.hasOverflowPill && !chainStripState.pillVisible) {
  errors.push("computed-style:chain-strip-more-hidden-on-mobile");
}
}

const isMain =
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (isMain) {
  const base = (process.argv[2] || "http://localhost:3000").replace(/\/$/, "");
  const errors = [];
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  attachBrowserSmokeHandlers(page, base, errors);
  await runBrowserSmokeChecks(page, base, errors);
  await browser.close();

  if (errors.length) {
    console.error(errors.join("\n"));
    process.exit(1);
  }

  console.log(`BROWSER SMOKE PASS ${base}`);
}
