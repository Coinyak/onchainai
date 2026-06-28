// Single Playwright session for full-tier UI gate (browser smoke + click test).
import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "fs";
import {
  attachBrowserSmokeHandlers,
  runBrowserSmokeChecks,
} from "./browser-smoke.mjs";
import {
  attachClickTestHandlers,
  createClickTestContext,
  runClickTestChecks,
} from "./click-test.mjs";

const base = (process.argv[2] || "http://localhost:3000").replace(/\/$/, "");
const scratch =
  process.env.ONCHAINAI_SCRATCH ||
  `${process.cwd()}/.playwright-cli/ui-browser-gate-scratch`;
mkdirSync(scratch, { recursive: true });

const smokeErrors = [];
const clickCtx = createClickTestContext(`${scratch}/click-test`);

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });

attachBrowserSmokeHandlers(page, base, smokeErrors);
attachClickTestHandlers(page, clickCtx);

let clickFailed = 0;
try {
  console.log(`UI browser gate: smoke checks (${base})`);
  await runBrowserSmokeChecks(page, base, smokeErrors);
  console.log(`UI browser gate: click checks (${base})`);
  clickFailed = await runClickTestChecks(page, base, clickCtx);
} finally {
  writeFileSync(
    `${scratch}/ui-browser-gate-summary.json`,
    JSON.stringify(
      {
        base,
        smokeErrorCount: smokeErrors.length,
        smokeErrors,
        clickFailed,
      },
      null,
      2,
    ),
  );
  await browser.close();
}

if (smokeErrors.length) {
  console.error(smokeErrors.join("\n"));
}

if (smokeErrors.length || clickFailed) {
  process.exit(1);
}

console.log(`UI BROWSER GATE PASS ${base}`);