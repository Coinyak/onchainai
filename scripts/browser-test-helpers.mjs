/** Shared helpers for Playwright smoke / click-test scripts. */

export const TOOL_PAGE_SIZE = 50;

export function expectedCumulativeMin(page1Cards, pageNum = 2) {
  if (page1Cards < TOOL_PAGE_SIZE) return page1Cards;
  return page1Cards + (pageNum - 1) * TOOL_PAGE_SIZE;
}

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Pace heavy catalog navigations to avoid tripping per-IP API rate limits. */
export const NAV_PACE_MS = 1500;

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
    return { skipped: false, imgCount, textLen: text.length };
  });
}

export function logoFallbackOk(result) {
  return !result.skipped && result.imgCount === 0 && result.textLen > 0;
}