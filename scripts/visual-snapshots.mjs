import { chromium } from "playwright";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import {
  clearSidebarStorage,
  isBenignConsoleError,
  visiblePageText,
  waitForToolCards,
} from "./browser-test-helpers.mjs";

const args = process.argv.slice(2);

function usage() {
  console.log(`Usage:
  node scripts/visual-snapshots.mjs [base-url] [--out DIR]
  node scripts/visual-snapshots.mjs [base-url] --update-baseline DIR
  node scripts/visual-snapshots.mjs [base-url] --baseline DIR [--out DIR]

Examples:
  node scripts/visual-snapshots.mjs http://localhost:3000
  node scripts/visual-snapshots.mjs http://localhost:3000 --out .playwright-cli/ui-snapshots
  node scripts/visual-snapshots.mjs http://localhost:3000 --update-baseline .visual-baselines
  node scripts/visual-snapshots.mjs http://localhost:3000 --baseline .visual-baselines
`);
}

let base = "http://localhost:3000";
let outDir = "";
let baselineDir = "";
let updateBaselineDir = "";

for (let i = 0; i < args.length; i += 1) {
  const arg = args[i];
  if (arg === "--help" || arg === "-h") {
    usage();
    process.exit(0);
  }
  if (arg === "--out") {
    outDir = args[++i] ?? "";
    continue;
  }
  if (arg === "--baseline") {
    baselineDir = args[++i] ?? "";
    continue;
  }
  if (arg === "--update-baseline") {
    updateBaselineDir = args[++i] ?? "";
    continue;
  }
  if (arg.startsWith("--")) {
    console.error(`Unknown option: ${arg}`);
    usage();
    process.exit(2);
  }
  base = arg;
}

base = base.replace(/\/$/, "");

const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
const targetDir =
  updateBaselineDir ||
  outDir ||
  path.join(".playwright-cli", "ui-snapshots", timestamp);

mkdirSync(targetDir, { recursive: true });

const viewports = [
  { name: "desktop", width: 1280, height: 900 },
  { name: "mobile", width: 375, height: 812 },
];

const routes = [
  { name: "home", path: "/", waitForCards: false },
  { name: "tools", path: "/tools", waitForCards: true },
  {
    name: "tools-bridge-mcp",
    path: "/tools?function=bridge&type=mcp",
    waitForCards: false,
  },
];

const consoleErrors = [];
const errors = [];
const results = [];

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();

page.on("console", (msg) => {
  const text = msg.text();
  if (msg.type() === "error" && !isBenignConsoleError(text)) {
    consoleErrors.push(text);
  }
});
page.on("requestfailed", (req) => {
  const url = req.url();
  if (!url.startsWith(base)) return;
  const failure = req.failure()?.errorText ?? "";
  errors.push(`requestfailed:${url}:${failure}`);
});

async function stabilizePage() {
  await page.addStyleTag({
    content: `
      *, *::before, *::after {
        animation-duration: 0s !important;
        animation-delay: 0s !important;
        transition-duration: 0s !important;
        transition-delay: 0s !important;
        scroll-behavior: auto !important;
      }
    `,
  });
}

function hashFile(filePath) {
  return createHash("sha256").update(readFileSync(filePath)).digest("hex");
}

for (const viewport of viewports) {
  await page.setViewportSize({ width: viewport.width, height: viewport.height });

  for (const route of routes) {
    await page.goto(`${base}${route.path}`, { waitUntil: "domcontentloaded" });
    await clearSidebarStorage(page);
    await page.reload({ waitUntil: "networkidle" });
    await stabilizePage();

    if (route.waitForCards) {
      await waitForToolCards(page).catch(() => {
        errors.push(`cards-timeout:${route.path}:${viewport.name}`);
      });
    }

    const text = await visiblePageText(page);
    if (/error deserializing|missing field filters/i.test(text || "")) {
      errors.push(`visible-error:${route.path}:${viewport.name}`);
    }

    const fileName = `${route.name}-${viewport.name}.png`;
    const filePath = path.join(targetDir, fileName);
    await page.screenshot({ path: filePath, fullPage: false });

    const hash = hashFile(filePath);
    const size = statSync(filePath).size;
    const result = {
      route: route.path,
      viewport: viewport.name,
      width: viewport.width,
      height: viewport.height,
      file: filePath,
      size,
      sha256: hash,
    };

    if (baselineDir) {
      const baselinePath = path.join(baselineDir, fileName);
      if (!existsSync(baselinePath)) {
        result.baseline = "missing";
        errors.push(`baseline-missing:${fileName}`);
      } else {
        const baselineHash = hashFile(baselinePath);
        result.baseline = baselineHash === hash ? "match" : "changed";
        result.baselineFile = baselinePath;
        if (baselineHash !== hash) {
          errors.push(`baseline-changed:${fileName}`);
        }
      }
    }

    results.push(result);
  }
}

await browser.close();

if (consoleErrors.length) {
  errors.push(...consoleErrors.slice(0, 5).map((text) => `console:error:${text}`));
}

const manifest = {
  base,
  generatedAt: new Date().toISOString(),
  mode: updateBaselineDir ? "update-baseline" : baselineDir ? "compare-baseline" : "capture",
  outputDir: targetDir,
  baselineDir: baselineDir || updateBaselineDir || null,
  results,
  errors,
};

writeFileSync(path.join(targetDir, "manifest.json"), JSON.stringify(manifest, null, 2));

if (errors.length) {
  console.error(`VISUAL SNAPSHOTS FAIL (${errors.length})`);
  for (const error of errors.slice(0, 20)) console.error(`- ${error}`);
  process.exit(1);
}

console.log(`VISUAL SNAPSHOTS PASS ${targetDir}`);
