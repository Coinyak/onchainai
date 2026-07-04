import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import {
  clearSidebarStorage,
  isBenignConsoleError,
  sleep,
  waitForToolCards,
} from "./browser-test-helpers.mjs";

const DEFAULT_BASE = "https://www.onchain-ai.xyz";
const args = process.argv.slice(2);

if (args[0] === "--help" || args[0] === "-h") {
  console.log(`Usage:
  node scripts/ui-audit-agent1.mjs [base-url]

Examples:
  node scripts/ui-audit-agent1.mjs
  node scripts/ui-audit-agent1.mjs http://localhost:3000
`);
  process.exit(0);
}

const BASE = (args[0] || DEFAULT_BASE).replace(/\/$/, "");
const OUT = path.join(
  ".playwright-cli",
  "ui-audit-agent1",
  new Date().toISOString().replace(/[:.]/g, "-"),
);
mkdirSync(OUT, { recursive: true });

const consoleErrors = [];
const errors = [];

let browser;
let page;

function formatError(error) {
  const message = error instanceof Error ? error.message : String(error);
  return message.replace(/\s+/g, " ").slice(0, 500);
}

async function waitForHomeContent() {
  await page.waitForSelector(".home-page .hero h1", { timeout: 20000 });
  await page
    .waitForSelector(".featured-carousel-card", { timeout: 20000 })
    .catch(() => errors.push("home-carousel-timeout"));
  await page
    .waitForSelector(".promo-card", { timeout: 20000 })
    .catch(() => errors.push("home-promo-timeout"));
}

async function waitForToolsChrome() {
  await page.waitForSelector(".site-top-nav", { timeout: 20000 });
  await waitForToolCards(page).catch(() => errors.push("tools-cards-timeout"));
}

function collectHomeAudit() {
  return page.evaluate(() => {
    const pick = (sel) => document.querySelector(sel);

    function resolvedLineHeight(el) {
      const style = getComputedStyle(el);
      const lh = style.lineHeight;
      if (lh === "normal") {
        const fs = parseFloat(style.fontSize) || 16;
        return fs * 1.2;
      }
      const parsed = parseFloat(lh);
      return Number.isFinite(parsed) ? parsed : 16;
    }

    function collectStatusElements() {
      const matches = [];
      const roots = document.querySelectorAll(
        'main, [role="status"], .tools-toolbar, .search-mode-header-row, aside.tools-sidebar',
      );
      for (const root of roots) {
        for (const el of root.querySelectorAll("*")) {
          if (el.children.length > 0) continue;
          const text = el.textContent?.trim() || "";
          if (/^Status:/i.test(text) || (/status/i.test(text) && /working/i.test(text))) {
            matches.push(el);
          }
        }
      }
      return matches.slice(0, 5);
    }

    const heroH1 = pick(".home-page .hero h1");
    const heroSub = pick(".home-page .hero h1 + p");

    const carouselCards = [...document.querySelectorAll(".featured-carousel-card")].map((card, i) => {
      const headline = card.querySelector(".featured-carousel-headline");
      const subtitle = card.querySelector(".featured-carousel-subtitle");
      const img = card.querySelector(".featured-carousel-image");
      const hRect = headline?.getBoundingClientRect();
      const sRect = subtitle?.getBoundingClientRect();
      const overlap = hRect && sRect ? sRect.top < hRect.bottom - 2 : false;
      const headlineStyle = headline ? getComputedStyle(headline) : null;
      const subtitleStyle = subtitle ? getComputedStyle(subtitle) : null;
      const bleed =
        headline && subtitle
          ? {
              headlineLines: Math.ceil((headline.scrollHeight || 0) / resolvedLineHeight(headline)),
              subtitleOverflow: getComputedStyle(subtitle).overflow,
              headlineOverflow: getComputedStyle(headline).overflow,
            }
          : null;
      return {
        index: i,
        headlineText: headline?.textContent?.trim()?.slice(0, 80),
        subtitleText: subtitle?.textContent?.trim()?.slice(0, 120),
        headlineFontSize: headlineStyle?.fontSize,
        subtitleFontSize: subtitleStyle?.fontSize,
        headlineLineHeight: headlineStyle?.lineHeight,
        subtitleLineHeight: subtitleStyle?.lineHeight,
        textOverlap: overlap,
        bleed,
        headlineScrollH: headline?.scrollHeight,
        headlineClientH: headline?.clientHeight,
        subtitleScrollH: subtitle?.scrollHeight,
        subtitleClientH: subtitle?.clientHeight,
        cardHeight: card.getBoundingClientRect().height,
        imgHeight: img?.getBoundingClientRect().height,
      };
    });

    const statusEls = collectStatusElements().map((el) => {
      const s = getComputedStyle(el);
      const r = el.getBoundingClientRect();
      return {
        text: el.textContent?.trim()?.slice(0, 100),
        tag: el.tagName,
        className: el.className,
        color: s.color,
        background: s.backgroundColor,
        fontSize: s.fontSize,
        opacity: s.opacity,
        zIndex: s.zIndex,
        rect: { x: r.x, y: r.y, w: r.width, h: r.height },
      };
    });

    const searchInput =
      pick('input[type="search"]') ||
      pick(".search-bar input") ||
      pick('[data-testid="home-search-bar"] input');
    const searchStyle = searchInput ? getComputedStyle(searchInput) : null;

    const promoCards = [...document.querySelectorAll(".promo-card")].map((card, i) => {
      const title = card.querySelector("h3, h2");
      const body = card.querySelector("p");
      const cta = card.querySelector("a, button");
      const tStyle = title ? getComputedStyle(title) : null;
      const bStyle = body ? getComputedStyle(body) : null;
      const ctaStyle = cta ? getComputedStyle(cta) : null;
      return {
        index: i,
        title: title?.textContent?.trim(),
        body: body?.textContent?.trim()?.slice(0, 100),
        cta: cta?.textContent?.trim(),
        titleSize: tStyle?.fontSize,
        bodySize: bStyle?.fontSize,
        ctaSize: ctaStyle?.fontSize,
      };
    });

    const sidebarBrand = pick('[data-testid="sidebar-brand"]') || pick(".sidebar-brand");
    const topNavLogo = pick(".site-top-nav-logo");
    const topNav = pick(".site-top-nav");
    const topNavBrands = topNav
      ? [...topNav.querySelectorAll(".site-top-nav-logo, a[href='/']")].map((el) => ({
          text: el.textContent?.trim(),
          class: el.className,
          rect: el.getBoundingClientRect(),
        }))
      : [];

    const heroStyle = heroH1 ? getComputedStyle(heroH1) : null;
    const heroSubStyle = heroSub ? getComputedStyle(heroSub) : null;

    return {
      page: location.pathname,
      hero: heroH1
        ? {
            text: heroH1.textContent?.trim(),
            fontSize: heroStyle.fontSize,
            fontWeight: heroStyle.fontWeight,
            lineHeight: heroStyle.lineHeight,
            letterSpacing: heroStyle.letterSpacing,
            color: heroStyle.color,
          }
        : null,
      heroSubtitle: heroSub
        ? {
            text: heroSub.textContent?.trim()?.slice(0, 120),
            fontSize: heroSubStyle.fontSize,
            color: heroSubStyle.color,
            lineHeight: heroSubStyle.lineHeight,
          }
        : null,
      carouselCards,
      statusEls,
      search: searchInput
        ? {
            placeholder: searchInput.getAttribute("placeholder"),
            fontSize: searchStyle.fontSize,
            color: searchStyle.color,
            width: searchInput.getBoundingClientRect().width,
            height: searchInput.getBoundingClientRect().height,
          }
        : null,
      promoCards,
      logos: {
        sidebarBrandText: sidebarBrand?.textContent?.trim(),
        sidebarBrandRect: sidebarBrand?.getBoundingClientRect(),
        topNavLogoText: topNavLogo?.textContent?.trim(),
        topNavBrands,
        duplicateLogoCheck: {
          sidebarVisible: !!(sidebarBrand && sidebarBrand.getBoundingClientRect().width > 0),
          topNavLogoVisible: !!(topNavLogo && topNavLogo.getBoundingClientRect().width > 0),
        },
      },
    };
  });
}

function collectToolsAudit() {
  return page.evaluate(() => {
    const pick = (sel) => document.querySelector(sel);

    const sidebarBrand = pick('[data-testid="sidebar-brand"]') || pick(".sidebar-brand");
    const topNav = pick(".site-top-nav");
    const topNavLogo = pick(".site-top-nav-logo");
    const searchInput =
      pick(".toolbar-search input") ||
      pick('input[type="search"]') ||
      pick(".search-input input");
    const toolCards = document.querySelectorAll(".tool-card:not(.skeleton-card)");

    const topNavBrands = topNav
      ? [...topNav.querySelectorAll(".site-top-nav-logo, a[href='/']")].map((el) => ({
          text: el.textContent?.trim(),
          class: el.className,
          rect: el.getBoundingClientRect(),
        }))
      : [];

    return {
      page: location.pathname,
      toolCardCount: toolCards.length,
      search: searchInput
        ? {
            placeholder: searchInput.getAttribute("placeholder"),
            fontSize: getComputedStyle(searchInput).fontSize,
            width: searchInput.getBoundingClientRect().width,
          }
        : null,
      logos: {
        sidebarBrandText: sidebarBrand?.textContent?.trim(),
        sidebarBrandRect: sidebarBrand?.getBoundingClientRect(),
        topNavLogoText: topNavLogo?.textContent?.trim(),
        topNavBrands,
        duplicateLogoCheck: {
          sidebarVisible: !!(sidebarBrand && sidebarBrand.getBoundingClientRect().width > 0),
          topNavLogoVisible: !!(topNavLogo && topNavLogo.getBoundingClientRect().width > 0),
        },
      },
    };
  });
}

try {
  browser = await chromium.launch({ headless: true });
  page = await browser.newPage({ viewport: { width: 1280, height: 900 } });

  page.on("console", (msg) => {
    const text = msg.text();
    if (msg.type() === "error" && !isBenignConsoleError(text)) {
      consoleErrors.push(text);
    }
  });
  page.on("requestfailed", (req) => {
    const url = req.url();
    if (!url.startsWith(BASE)) return;
    const failure = req.failure()?.errorText ?? "";
    if (url.includes("/pkg/") && /ERR_ABORTED/i.test(failure)) return;
    errors.push(`requestfailed:${url}:${failure}`);
  });

  await page.goto(`${BASE}/`, { waitUntil: "networkidle", timeout: 90000 });
  await clearSidebarStorage(page);
  await page.reload({ waitUntil: "networkidle", timeout: 90000 });
  await waitForHomeContent();

  await page.screenshot({ path: path.join(OUT, "01-home-full.png") });

  const hero = page.locator(".home-page .hero h1").first();
  if (await hero.count()) {
    await hero.screenshot({ path: path.join(OUT, "02-hero.png") });
  }

  const carousel = page.locator(".featured-carousel-shell").first();
  if (await carousel.count()) {
    await carousel.screenshot({ path: path.join(OUT, "03-featured-carousel.png") });
  }

  const statusOverlay = page.locator("text=/Status:/i").first();
  if (await statusOverlay.count()) {
    const box = await statusOverlay.boundingBox();
    if (box) {
      await page.screenshot({
        path: path.join(OUT, "04-status-overlay.png"),
        clip: {
          x: Math.max(0, box.x - 40),
          y: Math.max(0, box.y - 40),
          width: Math.min(1280, box.width + 120),
          height: Math.min(900, box.height + 120),
        },
      });
    }
  }

  const search = page
    .locator('input[type="search"], .search-bar input, [data-testid="home-search-bar"] input')
    .first();
  if (await search.count()) {
    await search.screenshot({ path: path.join(OUT, "05-search.png") });
  }

  const promo = page.locator(".promo-cards-section, .promo-cards-grid").first();
  if (await promo.count()) {
    await promo.screenshot({ path: path.join(OUT, "06-promo-cards.png") });
  }

  await page.screenshot({
    path: path.join(OUT, "07-header-sidebar-logos.png"),
    clip: { x: 0, y: 0, width: 400, height: 120 },
  });

  const homeAudit = await collectHomeAudit();

  await page.goto(`${BASE}/tools`, { waitUntil: "networkidle", timeout: 90000 });
  await sleep(500);
  await waitForToolsChrome();

  await page.screenshot({
    path: path.join(OUT, "08-tools-header-sidebar.png"),
    clip: { x: 0, y: 0, width: 1280, height: 140 },
  });

  const toolsAudit = await collectToolsAudit();

  writeFileSync(
    path.join(OUT, "audit-data.json"),
    JSON.stringify({ homePage: homeAudit, toolsPage: toolsAudit }, null, 2),
  );

  if (consoleErrors.length) {
    writeFileSync(path.join(OUT, "console-errors.json"), JSON.stringify(consoleErrors, null, 2));
    errors.push(`console-errors:${consoleErrors.length}`);
  }

  console.log(`AUDIT OUTPUT: ${OUT}`);

  if (errors.length) {
    console.error("UI audit completed with issues:");
    for (const item of errors) console.error(`  - ${item}`);
    process.exit(1);
  }
} catch (error) {
  console.error(`UI audit failed: ${formatError(error)}`);
  process.exit(1);
} finally {
  if (browser) {
    await browser.close();
  }
}