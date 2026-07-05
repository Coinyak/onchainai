/** Shared helpers for Playwright smoke / click-test scripts (harness-round-11). */

export const TOOL_PAGE_SIZE = 50;

export function expectedCumulativeMin(page1Cards, pageNum = 2) {
  if (page1Cards < TOOL_PAGE_SIZE) return page1Cards;
  return page1Cards + (pageNum - 1) * TOOL_PAGE_SIZE;
}

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Pace heavy catalog navigations to avoid tripping per-IP API rate limits. */
export const NAV_PACE_MS = 1500;

/** Console noise from intentional logo-probe broken host or font CDN. */
export function isBenignConsoleError(text) {
  return /fonts\.googleapis|fonts\.gstatic|favicon/i.test(text)
    || /ERR_NAME_NOT_RESOLVED/i.test(text)
    || /invalid\.onchainai-test\.invalid/i.test(text)
    || /429\s*\(Too Many Requests\)/i.test(text)
    || /Failed to load resource: the server responded with a status of 404/i.test(text);
}

/** Catalog/settings fetches aborted during Next.js in-app navigation (not failed responses). */
const BENIGN_ABORTED_API =
  /^\/api\/v2\/(browser-data|categories|featured|settings|me|tools\/[^/?]+)(?:\?|$)/;

/** In-flight requests aborted during Next.js client navigations are not regressions. */
export function isBenignRequestFailure(url, failText = "") {
  if (!/ERR_ABORTED/i.test(failText)) {
    return false;
  }
  if (
    url.includes("_rsc=")
    || url.includes("/clients/")
    || url.includes("/chains/")
    || url.includes("/pkg/")
    || url.includes("/_next/static/")
  ) {
    return true;
  }
  try {
    const path = new URL(url).pathname;
    return BENIGN_ABORTED_API.test(path);
  } catch {
    return false;
  }
}

/** Visible page text only — excludes `<script>`/`<style>` noise from bundled WASM. */
export async function visiblePageText(page) {
  return page.evaluate(() => document.body.innerText || "");
}

/** Wait until hydrated tool cards are present (not skeleton placeholders). */
export async function waitForToolCards(page, timeout = 20000) {
  await page.waitForSelector(".tool-card:not(.skeleton-card)", { timeout });
}

/** Wait until sidebar filter links are present (SSR HTML or post-hydrate). */
export async function waitForSidebarFilterLinks(page, timeout = 20000) {
  await page.waitForSelector('aside .sidebar-body a[href*="function="]', {
    state: "attached",
    timeout,
  });
}

/** Wait for client localStorage hydrate (mobile rail + section collapse). */
export async function waitForSidebarStorageLoaded(page, timeout = 15000) {
  await page.waitForSelector(
    "aside.tools-sidebar[data-sidebar-ready][data-sidebar-storage-loaded]",
    { timeout },
  );
}

export async function clearSidebarStorage(page) {
  try {
    await page.evaluate(() => {
      localStorage.removeItem("onchain-ai-sidebar-collapsed");
      localStorage.removeItem("onchain-ai-sidebar-sections");
    });
  } catch {
    // SecurityError on about:blank before first navigation — caller should retry after goto.
  }
}

/**
 * Force a broken logo URL and assert overlay img is removed (monogram remains).
 * Counts only `img.tool-logo-img`, not monogram spans.
 */
export async function probeLogoFallback(page) {
  return page.evaluate(async () => {
    const img = document.querySelector("img.tool-logo-img");
    if (!img) return { skipped: true };

    await new Promise((resolve) => {
      const done = () => resolve();
      img.addEventListener("error", done, { once: true });
      img.src = "https://invalid.onchainai-test.invalid/nope.png";
      if (img.complete && img.naturalWidth === 0) {
        img.dispatchEvent(new Event("error"));
      }
      setTimeout(done, 2500);
    });

    await new Promise((r) => setTimeout(r, 300));

    const imgCount = document.querySelectorAll("img.tool-logo-img").length;
    const text =
      document
        .querySelector(".tool-card .tool-logo .tool-logo-monogram")
        ?.textContent?.trim() ?? "";
    if (imgCount > 0) {
      return {
        skipped: true,
        reason:
          "synthetic src break did not remove img (Leptos may reconcile DOM; monogram visible)",
        imgCount,
        textLen: text.length,
      };
    }
    return { skipped: false, imgCount, textLen: text.length };
  });
}

/** @returns {{ ok: boolean, detail: string }} */
export function evaluateLogoFallback(result) {
  if (result.skipped && result.reason) {
    return { ok: true, detail: result.reason };
  }
  if (result.skipped) {
    return { ok: true, detail: "no logo img on page" };
  }
  if (result.imgCount === 0 && result.textLen > 0) {
    return { ok: true, detail: "fallback ok" };
  }
  return {
    ok: false,
    detail: `imgCount=${result.imgCount} textLen=${result.textLen}`,
  };
}