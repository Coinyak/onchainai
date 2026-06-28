// harness-round-11: gating click-test (sidebar-filter-click, mobile-tools)
import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "fs";
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
  visiblePageText,
  waitForToolCards,
  waitForSidebarFilterLinks,
  waitForSidebarStorageLoaded,
} from "./browser-test-helpers.mjs";

/** Steps that gate process exit (UI-only checks are informational). */
const gating = new Set([
  "home-load",
  "no-deser-error-home",
  "site-top-nav",
  "sidebar-filter-nav",
  "sidebar-filter-click",
  "tools-load",
  "tool-cards-present",
  "tool-logos-present",
  "tool-logo-fallback",
  "chain-strip-click",
  "page-2-load",
  "load-more-click",
  "load-more-api-errors",
  "load-more-present",
  "tool-preview-click",
  "tool-detail",
  "mobile-tools",
  "mobile-chain-tile-more",
  "bridge-sidebar-filter",
  "chain-strip-more-click",
  "bookmark-login-modal",
  "mobile-filter-reentry",
  "console-errors",
  "exception",
]);

export function createClickTestContext(scratch) {
  mkdirSync(scratch, { recursive: true });
  const outDir = `${scratch}/click-test-artifacts`;
  const results = [];
  const consoleErrors = [];
  const hydrationPanicRe =
    /hydration|entered unreachable code|panicked at|reactive value disposed/i;

  function log(step, ok, detail = "", gate = true) {
    results.push({ step, ok, detail, gate });
    const tag = gate ? "" : " [info]";
    console.log(`${ok ? "PASS" : "FAIL"} ${step}${tag}${detail ? ` — ${detail}` : ""}`);
  }

  return { outDir, results, consoleErrors, hydrationPanicRe, log };
}

/** Expand collapsed sidebar rail and function section so filter links are visible. */
async function ensureSidebarFiltersVisible(page) {
  const collapsed = await page.$(".tools-sidebar.tools-sidebar-collapsed");
  if (collapsed) {
    const railToggle = await page.$(".sidebar-rail-toggle");
    if (railToggle) await railToggle.click();
  }
  const closedFn = await page.$(
    '.sidebar-section .sidebar-panel.collapsed a[href*="function="]',
  );
  if (closedFn) {
    const fnToggle = await page.$(".sidebar-section button.sidebar-toggle");
    if (fnToggle) await fnToggle.click();
  }
  await page
    .waitForSelector('aside .sidebar-body a[href*="function="]:visible', {
      timeout: 8000,
    })
    .catch(() => {});
}

export function attachClickTestHandlers(page, ctx) {
  page.on("console", (msg) => {
    const text = msg.text();
    if (msg.type() === "error" && !isBenignConsoleError(text)) {
      ctx.consoleErrors.push(text);
    }
    if (ctx.hydrationPanicRe.test(text)) {
      ctx.consoleErrors.push(`hydration:${text}`);
    }
  });
}

export async function runClickTestChecks(page, base, ctx) {
  const { outDir, results, consoleErrors, hydrationPanicRe, log } = ctx;

try {
  await page.goto(`${base}/`, { waitUntil: "networkidle", timeout: 60000 });
  await clearSidebarStorage(page);
  log("home-load", true);

  const bodyText = await visiblePageText(page);
  log("no-deser-error-home", !/error deserializing|missing field filters/i.test(bodyText || ""));
  log("site-top-nav", !!(await page.$(".site-top-nav")));

  // Gating: direct navigation tests filter outcome without sidebar visibility races.
  await page.goto(`${base}/tools?function=bridge`, { waitUntil: "networkidle" });
  await waitForToolCards(page);
  const filterNavText = await visiblePageText(page);
  log(
    "sidebar-filter-nav",
    !/error deserializing|missing field/i.test(filterNavText || ""),
    page.url(),
  );

  // Gating: click a visible sidebar filter link (plan step 5 — real UI path).
  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  await waitForSidebarFilterLinks(page);
  await ensureSidebarFiltersVisible(page);
  await page
    .waitForSelector('aside .sidebar-body a[href*="function="]:visible', {
      timeout: 8000,
    })
    .catch(() => {});
  const fnLink = await page.$('aside .sidebar-body a[href*="function="]:visible');
  if (fnLink) {
    await fnLink.click();
    await page.waitForLoadState("networkidle");
    const after = await visiblePageText(page);
    log(
      "sidebar-filter-click",
      !/error deserializing|missing field/i.test(after || ""),
      page.url(),
    );
  } else {
    log("sidebar-filter-click", false, "no visible filter link");
  }

  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  await waitForSidebarFilterLinks(page);
  await ensureSidebarFiltersVisible(page);
  await page
    .waitForSelector('aside .sidebar-body a[href*="function="]:visible', {
      timeout: 8000,
    })
    .catch(() => {});
  const bridgeLink = await page.$('aside .sidebar-body a[href*="function=bridge"]:visible');
  if (bridgeLink) {
    await bridgeLink.click();
    await page.waitForLoadState("networkidle");
    const bridgeAfter = await visiblePageText(page);
    log(
      "bridge-sidebar-filter",
      page.url().includes("function=bridge") &&
        !/error deserializing|missing field/i.test(bridgeAfter || ""),
      page.url(),
    );
  } else {
    log("bridge-sidebar-filter", false, "no visible bridge filter link");
  }

  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  await waitForToolCards(page);
  log("tools-load", true);

  const toolCards = await page.$$(".tool-card:not(.skeleton-card)");
  const cardCount = toolCards.length;
  log("tool-cards-present", cardCount > 0, `count=${cardCount}`);

  const logoStats = await page.evaluate(() => ({
    monograms: document.querySelectorAll(".tool-logo-monogram").length,
    imgs: document.querySelectorAll("img.tool-logo-img").length,
  }));
  const expectLogos = cardCount >= TOOL_PAGE_SIZE;
  log(
    "tool-logos-present",
    !expectLogos || logoStats.imgs > 0,
    `monogram=${logoStats.monograms} img=${logoStats.imgs}`,
  );

  if (logoStats.imgs > 0) {
    const brokeFallback = await probeLogoFallback(page);
    const logoEval = evaluateLogoFallback(brokeFallback);
    log("tool-logo-fallback", logoEval.ok, logoEval.detail);
  }

  const chainLink = await page.$(".chain-strip a:visible, .chain-tile:visible");
  if (chainLink && (await chainLink.isVisible())) {
    await chainLink.click({ force: false });
    await page.waitForLoadState("networkidle");
    log("chain-strip-click", !/error deserializing/i.test((await visiblePageText(page)) || ""), page.url());
  } else {
    log("chain-strip-click", false, "no chain link");
  }

  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  await waitForToolCards(page);
  const chainMore = await page.$(".chain-tile-more");
  if (chainMore) {
    await chainMore.click();
    const expanded = await page.evaluate(() => {
      const pill = document.querySelector(".chain-tile-more");
      return pill?.getAttribute("aria-expanded") === "true";
    });
    log("chain-strip-more-click", expanded, expanded ? "expanded" : "not-expanded");
  } else {
    log("chain-strip-more-click", true, "no overflow pill");
  }

  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  await waitForToolCards(page);
  const bookmarkBtn = await page.$(".tool-card:not(.skeleton-card) .card-action-btn");
  if (bookmarkBtn) {
    await bookmarkBtn.click();
    let hasDialog = false;
    try {
      await page.waitForSelector('[role="dialog"]', { timeout: 5000 });
      hasDialog = true;
    } catch {
      hasDialog = false;
    }
    log("bookmark-login-modal", hasDialog, hasDialog ? "modal-open" : "no-modal");
    if (hasDialog) {
      await page.keyboard.press("Escape");
      await page.waitForSelector('[role="dialog"]', { state: "detached", timeout: 3000 }).catch(() => {});
    }
  } else {
    log("bookmark-login-modal", false, "no bookmark button");
  }

  const page1Cards = cardCount;
  const expectedPage2 = expectedCumulativeMin(page1Cards, 2);
  await sleep(NAV_PACE_MS);
  await page.goto(`${base}/tools?page=2`, { waitUntil: "networkidle" });
  await waitForToolCards(page);
  const page2Cards = (await page.$$(".tool-card:not(.skeleton-card)")).length;
  const page2Ok =
    !/error deserializing/i.test((await visiblePageText(page)) || "") &&
    page2Cards >= expectedPage2;
  log("page-2-load", page2Ok, `count=${page2Cards} expected>=${expectedPage2}`);

  await sleep(NAV_PACE_MS);
  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  await waitForToolCards(page);
  const beforeLoadMore = (await page.$$(".tool-card:not(.skeleton-card)")).length;
  const loadMore = await page.$("a.load-more-btn, .load-more-row a.load-more-btn");
  if (loadMore) {
    const loadMoreApiErrors = [];
    const onLoadMoreResponse = (res) => {
      const url = res.url();
      if (url.startsWith(base) && (url.includes("/api") || url.includes("/pkg/")) && res.status() >= 400) {
        loadMoreApiErrors.push(`${res.status()}:${url}`);
      }
    };
    page.on("response", onLoadMoreResponse);
    await loadMore.click();
    let navTimedOut = false;
    await page.waitForURL(/[?&]page=2/, { timeout: 20000 }).catch(() => {
      navTimedOut = true;
    });
    await page.waitForLoadState("networkidle");
    await waitForToolCards(page).catch(() => {});
    const expectedMin = expectedCumulativeMin(beforeLoadMore, 2);
    let waitTimedOut = false;
    await page
      .waitForFunction(
        (min) =>
          document.querySelectorAll(".tool-card:not(.skeleton-card)").length >= min,
        expectedMin,
        { timeout: 20000 },
      )
      .catch(() => {
        waitTimedOut = true;
      });
    page.off("response", onLoadMoreResponse);
    const after = (await page.$$(".tool-card:not(.skeleton-card)")).length;
    const detail = navTimedOut
      ? `nav timeout url=${page.url()}`
      : waitTimedOut
        ? `card wait timeout ${beforeLoadMore} -> ${after} (expected >= ${expectedMin})`
        : `${beforeLoadMore} -> ${after}`;
    log(
      "load-more-click",
      !navTimedOut && !waitTimedOut && after >= expectedMin,
      detail,
    );
    log(
      "load-more-api-errors",
      loadMoreApiErrors.length === 0,
      loadMoreApiErrors.slice(0, 3).join(" | ") || "none",
    );
  } else if (beforeLoadMore >= TOOL_PAGE_SIZE) {
    log("load-more-present", false, `cards=${beforeLoadMore} but no button`);
  } else {
    log("load-more-present", true, "small catalog");
  }

  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  await waitForToolCards(page);
  const firstCard = await page.$(".tool-card:not(.skeleton-card) a.tool-card-link");
  if (firstCard) {
    await firstCard.click();
    await page.waitForLoadState("networkidle");
    const hasPreview = !!(await page.$(".preview-desktop, .preview-panel, .bottom-sheet"));
    log("tool-preview-click", hasPreview || page.url().includes("selected="), page.url());
  }

  const detailHref = await page
    .$eval(".tool-card:not(.skeleton-card) a.tool-card-link", (a) => a.getAttribute("href"))
    .catch(() => null);
  if (detailHref && detailHref.startsWith("/tools/")) {
    await page.goto(`${base}${detailHref}`, { waitUntil: "networkidle" });
    log("tool-detail", (await page.textContent("h1, .tool-detail h1"))?.length > 0, detailHref);
  }

  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  let sidebarHydrationError = "";
  const sidebarStorageLoaded = await waitForSidebarStorageLoaded(page)
    .then(() => true)
    .catch((error) => {
      sidebarHydrationError = error instanceof Error ? error.message : String(error);
      return false;
    });
  const mobileText = await visiblePageText(page);
  log(
    "mobile-tools",
    sidebarStorageLoaded && !/error deserializing|missing field/i.test(mobileText || ""),
    sidebarStorageLoaded ? "" : `sidebar-hydration:${sidebarHydrationError}`,
  );

  const chainMoreVisible = await page.evaluate(() => {
    const pill = document.querySelector(".chain-tile-more");
    if (!pill) return true;
    const style = getComputedStyle(pill);
    return style.display !== "none" && style.visibility !== "hidden";
  });
  log("mobile-chain-tile-more", chainMoreVisible, chainMoreVisible ? "visible" : "hidden");

  await clearSidebarStorage(page);
  await page.reload({ waitUntil: "networkidle" });
  await waitForSidebarStorageLoaded(page).catch(() => {});
  const railToggle = await page.$(".sidebar-rail-toggle");
  let mobileFilterReentryOk = false;
  if (railToggle) {
    await railToggle.click();
    const filterLink = await page.$('aside .sidebar-body a[href*="function="]:visible');
    if (filterLink) {
      await filterLink.click();
      await page
        .waitForFunction(
          () =>
            document
              .querySelector(".tools-sidebar")
              ?.classList.contains("tools-sidebar-collapsed") === true,
          null,
          { timeout: 10000 },
        )
        .catch(() => {});
    }
    await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
    await waitForSidebarStorageLoaded(page).catch(() => {});
    mobileFilterReentryOk = await page.evaluate(() =>
      document.querySelector(".tools-sidebar")?.classList.contains("tools-sidebar-collapsed"),
    );
  }
  log(
    "mobile-filter-reentry",
    mobileFilterReentryOk,
    mobileFilterReentryOk ? "collapsed-on-reentry" : "overlay-still-open",
  );

  await page.screenshot({ path: `${outDir}-mobile-tools.png`, fullPage: false });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  await page.screenshot({ path: `${outDir}-desktop-tools.png`, fullPage: false });

  const hydrationErrors = consoleErrors.filter((e) => hydrationPanicRe.test(e));
  log(
    "console-errors",
    hydrationErrors.length === 0,
    hydrationErrors.slice(0, 3).join(" | ") || "none",
  );
} catch (e) {
  log("exception", false, String(e));
  await page.screenshot({ path: `${outDir}-error.png` }).catch(() => {});
}

  writeFileSync(`${outDir}-results.json`, JSON.stringify(results, null, 2));
  return results.filter((r) => r.gate !== false && !r.ok).length;
}

const isMain =
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (isMain) {
  const base = (process.argv[2] || "https://www.onchain-ai.xyz").replace(/\/$/, "");
  const scratch =
    process.env.ONCHAINAI_SCRATCH ||
    `${process.cwd()}/.playwright-cli/click-test-scratch`;
  const ctx = createClickTestContext(scratch);
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  attachClickTestHandlers(page, ctx);
  const failed = await runClickTestChecks(page, base, ctx);
  await browser.close();
  process.exit(failed ? 1 : 0);
}
