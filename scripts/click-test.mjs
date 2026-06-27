import { chromium } from "playwright";
import { writeFileSync } from "fs";

const base = (process.argv[2] || "https://www.onchain-ai.xyz").replace(/\/$/, "");
const outDir = "/tmp/onchainai-browser-test";
const TOOL_PAGE_SIZE = 50;
const results = [];

function log(step, ok, detail = "") {
  results.push({ step, ok, detail });
  console.log(`${ok ? "PASS" : "FAIL"} ${step}${detail ? ` — ${detail}` : ""}`);
}

async function clearSidebarStorage(page) {
  await page.evaluate(() => {
    localStorage.removeItem("onchain-ai-sidebar-collapsed");
    localStorage.removeItem("onchain-ai-sidebar-sections");
  });
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
  if (msg.type() === "error" && !/fonts\.googleapis|favicon/i.test(text)) {
    consoleErrors.push(text);
  }
  if (hydrationPanicRe.test(text)) {
    consoleErrors.push(`hydration:${text}`);
  }
});

try {
  await clearSidebarStorage(page);
  await page.goto(`${base}/`, { waitUntil: "networkidle", timeout: 60000 });
  log("home-load", true);

  const bodyText = await page.textContent("body");
  log("no-deser-error-home", !/error deserializing|missing field filters/i.test(bodyText || ""));
  log("sidebar-brand", !!(await page.$(".sidebar-brand")));

  // Sidebar filters: use /tools where function section defaults open (home keeps it collapsed).
  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  await ensureSidebarFiltersVisible(page);
  const fnLink = await page.$('aside .sidebar-body a[href*="function="]:visible');
  if (fnLink) {
    await fnLink.click();
    await page.waitForLoadState("networkidle");
    const after = await page.textContent("body");
    log("sidebar-filter-click", !/error deserializing/i.test(after || ""), page.url());
  } else {
    log("sidebar-filter-click", false, "no visible filter link");
  }

  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  log("tools-load", true);

  const toolCards = await page.$$(".tool-card");
  const cardCount = toolCards.length;
  log("tool-cards-present", cardCount > 0, `count=${cardCount}`);

  const logoStats = await page.evaluate(() => ({
    monograms: document.querySelectorAll(".tool-logo-monogram").length,
    imgs: document.querySelectorAll(".tool-logo img, .tool-card img.tool-logo-img").length,
  }));
  const expectLogos = cardCount >= TOOL_PAGE_SIZE;
  log(
    "tool-logos-present",
    !expectLogos || logoStats.imgs > 0,
    `monogram=${logoStats.monograms} img=${logoStats.imgs}`,
  );

  if (logoStats.imgs > 0) {
    const brokeFallback = await page.evaluate(async () => {
      const img = document.querySelector(".tool-logo-img");
      if (!img) return { skipped: true };
      img.src = "https://invalid.onchainai-test.invalid/nope.png";
      await new Promise((r) => setTimeout(r, 600));
      const stillImg = !!document.querySelector(".tool-logo-img");
      const logo = document.querySelector(".tool-card .tool-logo");
      const text = logo?.querySelector(".tool-logo-monogram")?.textContent?.trim() ?? "";
      return { skipped: false, stillImg, textLen: text.length };
    });
    if (!brokeFallback.skipped) {
      log(
        "tool-logo-fallback",
        !brokeFallback.stillImg && brokeFallback.textLen > 0,
        `stillImg=${brokeFallback.stillImg} textLen=${brokeFallback.textLen}`,
      );
    }
  }

  // Chain strip click
  const chainLink = await page.$(".chain-strip a:visible, .chain-tile:visible");
  if (chainLink && (await chainLink.isVisible())) {
    await chainLink.click({ force: false });
    await page.waitForLoadState("networkidle");
    log("chain-strip-click", !/error deserializing/i.test((await page.textContent("body")) || ""), page.url());
  } else {
    log("chain-strip-click", false, "no chain link");
  }

  // Direct page=2 navigation — cumulative list should show up to 100 cards on large catalogs.
  await page.goto(`${base}/tools?page=2`, { waitUntil: "networkidle" });
  const page2Cards = (await page.$$(".tool-card")).length;
  const page2Ok =
    !/error deserializing/i.test((await page.textContent("body")) || "") &&
    page2Cards >= TOOL_PAGE_SIZE;
  log("page-2-load", page2Ok, `count=${page2Cards}`);

  // Load more: full-page navigation to ?page=2
  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  const beforeLoadMore = (await page.$$(".tool-card")).length;
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
    const expectedMin = beforeLoadMore + TOOL_PAGE_SIZE;
    let waitTimedOut = false;
    await page
      .waitForFunction(
        (min) => document.querySelectorAll(".tool-card").length >= min,
        expectedMin,
        { timeout: 20000 },
      )
      .catch(() => {
        waitTimedOut = true;
      });
    page.off("response", onLoadMoreResponse);
    const after = (await page.$$(".tool-card")).length;
    const grew = after > beforeLoadMore;
    const detail = navTimedOut
      ? `nav timeout url=${page.url()}`
      : waitTimedOut
        ? `card wait timeout ${beforeLoadMore} -> ${after} (expected >= ${expectedMin})`
        : `${beforeLoadMore} -> ${after}`;
    log("load-more-click", !navTimedOut && !waitTimedOut && grew, detail);
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

  // Open first tool preview
  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  const firstCard = await page.$(".tool-card a.tool-card-link");
  if (firstCard) {
    await firstCard.click();
    await page.waitForLoadState("networkidle");
    const hasPreview = !!(await page.$(".preview-desktop, .preview-panel, .bottom-sheet"));
    log("tool-preview-click", hasPreview || page.url().includes("selected="), page.url());
  }

  // Tool detail page
  const detailHref = await page
    .$eval(".tool-card a.tool-card-link", (a) => a.getAttribute("href"))
    .catch(() => null);
  if (detailHref && detailHref.startsWith("/tools/")) {
    await page.goto(`${base}${detailHref}`, { waitUntil: "networkidle" });
    log("tool-detail", (await page.textContent("h1, .tool-detail h1"))?.length > 0, detailHref);
  }

  // Mobile viewport
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  log("mobile-tools", !/error deserializing/i.test((await page.textContent("body")) || ""));

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
    consoleErrors.length === 0,
    consoleErrors.slice(0, 3).join(" | ") || "none",
  );
  if (hydrationErrors.length) {
    log("hydration-panic", false, hydrationErrors.slice(0, 2).join(" | "));
  }
} catch (e) {
  log("exception", false, String(e));
  await page.screenshot({ path: `${outDir}-error.png` }).catch(() => {});
} finally {
  writeFileSync(`${outDir}-results.json`, JSON.stringify(results, null, 2));
  await browser.close();
}

const failed = results.filter((r) => !r.ok);
process.exit(failed.length ? 1 : 0);