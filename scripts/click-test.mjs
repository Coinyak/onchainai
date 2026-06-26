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

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
const consoleErrors = [];
page.on("console", (msg) => {
  if (msg.type() === "error" && !/fonts\.googleapis|favicon/i.test(msg.text())) {
    consoleErrors.push(msg.text());
  }
});

try {
  await page.goto(`${base}/`, { waitUntil: "networkidle", timeout: 60000 });
  log("home-load", true);

  const bodyText = await page.textContent("body");
  log("no-deser-error-home", !/error deserializing|missing field filters/i.test(bodyText || ""));
  log("sidebar-brand", !!(await page.$(".sidebar-brand")));

  // Expand sidebar if collapsed, then click visible function filter
  const toggle = await page.$(".sidebar-toggle, .sidebar-rail button, button.sidebar-expand");
  if (toggle) await toggle.click().catch(() => {});
  const fnLink = await page.$(
    'aside .sidebar-body a[href*="function="], aside.tools-sidebar:not(.tools-sidebar-collapsed) a[href*="function="]',
  );
  if (fnLink && (await fnLink.isVisible())) {
    await fnLink.click();
    await page.waitForLoadState("networkidle");
    const after = await page.textContent("body");
    log("sidebar-filter-click", !/error deserializing/i.test(after || ""), page.url());
  } else {
    log("sidebar-filter-click", false, "no filter link");
  }

  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  log("tools-load", true);

  const toolCards = await page.$$(".tool-card");
  const cardCount = toolCards.length;
  log("tool-cards-present", cardCount > 0, `count=${cardCount}`);

  const logoStats = await page.evaluate(() => ({
    monograms: document.querySelectorAll(".tool-logo").length,
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
      await new Promise((r) => setTimeout(r, 400));
      const stillImg = !!document.querySelector(".tool-logo-img");
      const logo = document.querySelector(".tool-card .tool-logo");
      const text = logo?.textContent?.trim() ?? "";
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

  // Direct page=2 navigation
  await page.goto(`${base}/tools?page=2`, { waitUntil: "networkidle" });
  const page2Cards = (await page.$$(".tool-card")).length;
  log(
    "page-2-load",
    !/error deserializing/i.test((await page.textContent("body")) || "") &&
      (page2Cards >= TOOL_PAGE_SIZE || page2Cards > 0),
    `count=${page2Cards}`,
  );

  // Load more if present
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
    await page.waitForLoadState("networkidle");
    let waitTimedOut = false;
    await page
      .waitForFunction(
        (count) => document.querySelectorAll(".tool-card").length > count,
        beforeLoadMore,
        { timeout: 15000 },
      )
      .catch(() => {
        waitTimedOut = true;
      });
    page.off("response", onLoadMoreResponse);
    const after = (await page.$$(".tool-card")).length;
    const detail = waitTimedOut
      ? `waitForFunction timeout ${beforeLoadMore} -> ${after}`
      : `${beforeLoadMore} -> ${after}`;
    log("load-more-click", !waitTimedOut && after > beforeLoadMore, detail);
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

  log("console-errors", consoleErrors.length === 0, consoleErrors.slice(0, 3).join(" | "));
} catch (e) {
  log("exception", false, String(e));
  await page.screenshot({ path: `${outDir}-error.png` }).catch(() => {});
} finally {
  writeFileSync(`${outDir}-results.json`, JSON.stringify(results, null, 2));
  await browser.close();
}

const failed = results.filter((r) => !r.ok);
process.exit(failed.length ? 1 : 0);