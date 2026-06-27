import { chromium } from "playwright";
import {
  TOOL_PAGE_SIZE,
  expectedCumulativeMin,
  sleep,
  NAV_PACE_MS,
  clearSidebarStorage,
  probeLogoFallback,
  evaluateLogoFallback,
  isBenignConsoleError,
} from "./browser-test-helpers.mjs";

const base = (process.argv[2] || "http://localhost:3000").replace(/\/$/, "");
const errors = [];
const hydrationPanicRe =
  /hydration|entered unreachable code|panicked at|reactive value disposed/i;

function countRealToolCards(page) {
  return page.evaluate(
    () => document.querySelectorAll(".tool-card:not(.skeleton-card)").length,
  );
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });

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
  // Chain strip SVGs often abort during in-page navigation; not a deploy regression.
  if (url.includes("/chains/") && /ERR_ABORTED/i.test(failText)) {
    return;
  }
  errors.push(`requestfailed:${url}:${failText}`);
});
page.on("response", async (res) => {
  const url = res.url();
  if (url.includes("/api") || url.includes("/pkg/")) {
    if (res.status() >= 400) errors.push(`http:${res.status()}:${url}`);
    const contentType = res.headers()["content-type"] || "";
    const isText =
      contentType.includes("json") ||
      contentType.includes("text") ||
      contentType.includes("javascript");
    if (isText) {
      const text = await res.text().catch(() => "");
      if (/error deserializing|missing field filters/i.test(text)) {
        errors.push(`body-error:${url}:${text.slice(0, 200)}`);
      }
    }
  }
});

await page.goto(`${base}/`, { waitUntil: "domcontentloaded" });
await clearSidebarStorage(page);

await page.goto(`${base}/`, { waitUntil: "networkidle" });
const homeLayout = await page.evaluate(() => ({
  hasSidebarBrand: !!document.querySelector(".sidebar-brand"),
  hasCategoryGrid: !!document.querySelector(".category-grid"),
}));
if (!homeLayout.hasSidebarBrand) {
  errors.push("layout:home-missing-sidebar-brand");
}
if (homeLayout.hasCategoryGrid) {
  errors.push("layout:home-unexpected-category-grid");
}

for (const path of ["/", "/tools", "/tools?function=bridge&type=mcp"]) {
  await page.goto(`${base}${path}`, { waitUntil: "networkidle" });
  const text = await page.textContent("body");
  if (/error deserializing|missing field filters/i.test(text || "")) {
    errors.push(`visible-error:${path}`);
  }
}

await page.setViewportSize({ width: 1280, height: 900 });
await page.goto(`${base}/`, { waitUntil: "networkidle" });
const desktopH1Size = await page.evaluate(() => {
  const h1 = document.querySelector(".home-page h1");
  return h1 ? getComputedStyle(h1).fontSize : "";
});
await page.setViewportSize({ width: 375, height: 812 });
await page.goto(`${base}/`, { waitUntil: "networkidle" });
const mobileH1Size = await page.evaluate(() => {
  const h1 = document.querySelector(".home-page h1");
  return h1 ? getComputedStyle(h1).fontSize : "";
});
if (!desktopH1Size || !mobileH1Size) {
  errors.push("computed-style:home-h1-missing");
} else if (desktopH1Size === mobileH1Size) {
  errors.push(`computed-style:home-h1-same:${desktopH1Size}`);
}

const cssRes = await page.goto(`${base}/pkg/onchainai.css`, {
  waitUntil: "networkidle",
});
const cssText = await cssRes?.text().catch(() => "");
if (!cssText || cssText.trim().length === 0) {
  errors.push("css-empty:/pkg/onchainai.css");
}

await page.setViewportSize({ width: 1280, height: 900 });
await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
const toolsLogoStats = await page.evaluate(() => ({
  cards: document.querySelectorAll(".tool-card:not(.skeleton-card)").length,
  imgs: document.querySelectorAll("img.tool-logo-img").length,
}));
if (toolsLogoStats.cards >= 50 && toolsLogoStats.imgs === 0) {
  errors.push(`layout:tools-missing-logo-imgs:cards=${toolsLogoStats.cards}`);
}
if (toolsLogoStats.imgs > 0) {
  const brokeFallback = await probeLogoFallback(page);
  const logoEval = evaluateLogoFallback(brokeFallback);
  if (!logoEval.ok) {
    errors.push(`layout:tools-logo-fallback-missing:${logoEval.detail}`);
  }
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
  await page.goto(`${base}/tools?page=2`, { waitUntil: "networkidle" });
  const page2Text = await page.textContent("body");
  const page2Cards = await countRealToolCards(page);
  if (/error deserializing|missing field filters/i.test(page2Text || "")) {
    errors.push("visible-error:/tools?page=2");
  }
  if (page2Cards < expectedPage2) {
    errors.push(`layout:page-2-too-few-cards:${page2Cards}<${expectedPage2}`);
  }

  await sleep(NAV_PACE_MS);
  await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
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
    await page.waitForLoadState("networkidle");
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

await page.setViewportSize({ width: 375, height: 812 });
await page.goto(`${base}/tools`, { waitUntil: "domcontentloaded" });
await clearSidebarStorage(page);
await page.reload({ waitUntil: "networkidle" });
const mobileSidebarCollapsed = await page.evaluate(() => {
  const aside = document.querySelector(".tools-sidebar");
  return aside?.classList.contains("tools-sidebar-collapsed") ?? false;
});
if (!mobileSidebarCollapsed) {
  errors.push("layout:mobile-sidebar-not-collapsed");
}

await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
const chainMoreVisible = await page.evaluate(() => {
  const pill = document.querySelector(".chain-tile-more");
  if (!pill) return true;
  const style = getComputedStyle(pill);
  return style.display !== "none" && style.visibility !== "hidden";
});
if (!chainMoreVisible) {
  errors.push("computed-style:chain-strip-more-hidden-on-mobile");
}

await browser.close();

if (errors.length) {
  console.error(errors.join("\n"));
  process.exit(1);
}

console.log(`BROWSER SMOKE PASS ${base}`);