import { chromium } from "playwright";

const base = (process.argv[2] || "http://localhost:3000").replace(/\/$/, "");
const errors = [];
const hydrationPanicRe =
  /hydration|entered unreachable code|panicked at/i;
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });

page.on("console", (msg) => {
  const text = msg.text();
  if (hydrationPanicRe.test(text)) {
    errors.push(`hydration-panic:${text}`);
  }
  if (["error", "warning"].includes(msg.type())) {
    errors.push(`console:${msg.type()}:${text}`);
  }
});
page.on("requestfailed", (req) => {
  const url = req.url();
  // External font CDN flakes in headless CI; not an app regression signal.
  if (url.includes("fonts.gstatic.com") || url.includes("fonts.googleapis.com")) {
    return;
  }
  if (!url.startsWith(base)) {
    return;
  }
  errors.push(`requestfailed:${url}:${req.failure()?.errorText}`);
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
await page.evaluate(() => {
  localStorage.removeItem("onchain-ai-sidebar-collapsed");
  localStorage.removeItem("onchain-ai-sidebar-sections");
});

// Home layout: sidebar brand present, legacy category grid removed.
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

// Home H1 font-size should differ between desktop and mobile breakpoints.
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

// Served CSS bundle must be non-empty.
const cssRes = await page.goto(`${base}/pkg/onchainai.css`, {
  waitUntil: "networkidle",
});
const cssText = await cssRes?.text().catch(() => "");
if (!cssText || cssText.trim().length === 0) {
  errors.push("css-empty:/pkg/onchainai.css");
}

// Tools list: large catalog should expose load-more markup.
await page.setViewportSize({ width: 1280, height: 900 });
await page.goto(`${base}/tools`, { waitUntil: "networkidle" });
const toolsLoadMore = await page.evaluate(() => {
  const cards = document.querySelectorAll(".tool-card").length;
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

// Large catalog: click load-more and assert card count strictly increases.
const toolsCards = await page.evaluate(
  () => document.querySelectorAll(".tool-card").length,
);
if (toolsCards >= 50) {
  const before = toolsCards;
  const loadMore = await page.$("a.load-more-btn, .load-more-row a.load-more-btn");
  if (!loadMore) {
    errors.push("interaction:tools-missing-load-more-button");
  } else {
    await loadMore.click();
    await page.waitForLoadState("networkidle");
    let timedOut = false;
    await page
      .waitForFunction(
        (count) => document.querySelectorAll(".tool-card").length > count,
        before,
        { timeout: 15000 },
      )
      .catch(() => {
        timedOut = true;
      });
    const after = await page.evaluate(
      () => document.querySelectorAll(".tool-card").length,
    );
    if (timedOut || after <= before) {
      errors.push(`interaction:load-more-not-growing:${before}->${after}`);
    }
  }
}

// Direct ?page=2 navigation should not surface deserialization errors.
await page.goto(`${base}/tools?page=2`, { waitUntil: "networkidle" });
const page2Text = await page.textContent("body");
if (/error deserializing|missing field filters/i.test(page2Text || "")) {
  errors.push("visible-error:/tools?page=2");
}

// Mobile sidebar defaults collapsed at 375px when localStorage is cleared.
await page.setViewportSize({ width: 375, height: 812 });
await page.goto(`${base}/tools`, { waitUntil: "domcontentloaded" });
await page.evaluate(() => {
  localStorage.removeItem("onchain-ai-sidebar-collapsed");
  localStorage.removeItem("onchain-ai-sidebar-sections");
});
await page.reload({ waitUntil: "networkidle" });
const mobileSidebarCollapsed = await page.evaluate(() => {
  const aside = document.querySelector(".tools-sidebar");
  return aside?.classList.contains("tools-sidebar-collapsed") ?? false;
});
if (!mobileSidebarCollapsed) {
  errors.push("layout:mobile-sidebar-not-collapsed");
}

// Chain strip "+N" overflow control (not tool-card .chain-more — hidden on mobile by CSS).
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