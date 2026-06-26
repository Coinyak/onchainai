import { chromium } from "playwright";
import { writeFileSync } from "fs";

const base = (process.argv[2] || "https://www.onchain-ai.xyz").replace(/\/$/, "");
const outDir = "/tmp/onchainai-browser-test";
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
  const fnLink = await page.$('aside .sidebar-body a[href*="function="], aside.tools-sidebar:not(.tools-sidebar-collapsed) a[href*="function="]');
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
  log("tool-cards-present", toolCards.length > 0, `count=${toolCards.length}`);

  // Monogram vs img logos
  const logoStats = await page.evaluate(() => ({
    monograms: document.querySelectorAll(".tool-logo").length,
    imgs: document.querySelectorAll(".tool-logo img, .tool-card img.tool-logo-img").length,
  }));
  log("tool-logos-monogram", logoStats.monograms > 0, `monogram=${logoStats.monograms} img=${logoStats.imgs}`);

  // Chain strip click
  const chainLink = await page.$(".chain-strip a:visible, .chain-tile:visible");
  if (chainLink && (await chainLink.isVisible())) {
    await chainLink.click({ force: false });
    await page.waitForLoadState("networkidle");
    log("chain-strip-click", !/error deserializing/i.test((await page.textContent("body")) || ""), page.url());
  } else {
    log("chain-strip-click", false, "no chain link");
  }

  // Load more if present
  const loadMore = await page.$("a.load-more-btn");
  if (loadMore) {
    const before = (await page.$$(".tool-card")).length;
    await loadMore.click();
    await page.waitForLoadState("networkidle");
    await page
      .waitForFunction(
        (count) => document.querySelectorAll(".tool-card").length > count,
        before,
        { timeout: 15000 },
      )
      .catch(() => {});
    const after = (await page.$$(".tool-card")).length;
    log("load-more-click", after > before, `${before} -> ${after}`);
  } else {
    log("load-more-click", true, "no button (small catalog or capped)");
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
  const detailHref = await page.$eval(".tool-card a.tool-card-link", (a) => a.getAttribute("href")).catch(() => null);
  if (detailHref && detailHref.startsWith("/tools/")) {
    await page.goto(`${base}${detailHref}`, { waitUntil: "networkidle" });
    log("tool-detail", (await page.textContent("h1, .tool-detail h1"))?.length > 0, detailHref);
  }

  // Mobile viewport
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
  log("mobile-tools", !/error deserializing/i.test((await page.textContent("body")) || ""));

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