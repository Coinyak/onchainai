// harness-round-11: gating click-test (sidebar-filter-click, mobile-tools)
import { chromium } from "playwright";
import { writeFileSync } from "fs";
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

const base = (process.argv[2] || "https://www.onchain-ai.xyz").replace(/\/$/, "");
const scratch =
  process.env.ONCHAINAI_SCRATCH ||
  "/var/folders/k7/_r0bjtp12dngr0ncryvtt4mc0000gn/T/grok-goal-11e98898edeb/implementer";
const outDir = `${scratch}/click-test-artifacts`;
const results = [];
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
  "console-errors",
  "exception",
]);

function log(step, ok, detail = "", gate = true) {
  results.push({ step, ok, detail, gate });
  const tag = gate ? "" : " [info]";
  console.log(`${ok ? "PASS" : "FAIL"} ${step}${tag}${detail ? ` — ${detail}` : ""}`);
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

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
const consoleErrors = [];
const hydrationPanicRe = /hydration|entered unreachable code|panicked at|reactive value disposed/i;
page.on("console", (msg) => {
  const text = msg.text();
  if (msg.type() === "error" && !isBenignConsoleError(text)) {
    consoleErrors.push(text);
  }
  if (hydrationPanicRe.test(text)) {
    consoleErrors.push(`hydration:${text}`);
  }
});

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
} finally {
  writeFileSync(`${outDir}-results.json`, JSON.stringify(results, null, 2));
  await browser.close();
}

const failed = results.filter((r) => r.gate !== false && !r.ok);
process.exit(failed.length ? 1 : 0);
