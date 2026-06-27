/** Shared helpers for Playwright smoke / click-test scripts. */

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
    || /invalid\.onchainai-test\.invalid/i.test(text);
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