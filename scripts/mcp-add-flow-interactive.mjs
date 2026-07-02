/**
 * MCP add flow interactive verification — full transcript + screenshots.
 * Usage: node scripts/mcp-add-flow-interactive.mjs [baseUrl] [scratchDir]
 */
import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "fs";
import { join } from "path";

const base = (process.argv[2] || "http://127.0.0.1:3000").replace(/\/$/, "");
const scratch = process.argv[3] || ".playwright-cli/mcp-add-flow";
const snapDir = join(scratch, "ui-snapshots");
mkdirSync(snapDir, { recursive: true });

const transcript = [];
const log = (step, detail) => {
  const line = `[${new Date().toISOString()}] ${step}: ${detail}`;
  transcript.push(line);
  console.log(line);
};

const wait = { waitUntil: "domcontentloaded", timeout: 45000 };
const browser = await chromium.launch({ headless: true });

try {
  log("A1", `GET ${base}/`);
  const home = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await home.goto(`${base}/`, wait);
  const card = home.locator('[data-testid="connect-onchainai-mcp-card"]');
  await card.waitFor({ timeout: 15000 });
  const platformCount = await card.locator(".install-platform-btn").count();
  log("A2", `connect-card platform buttons=${platformCount}`);
  const claudePressed = await card
    .locator(".install-platform-btn", { hasText: "Claude" })
    .getAttribute("aria-pressed");
  log("A3", `connect-card default claude pressed=${claudePressed}`);
  await card.locator(".install-platform-btn", { hasText: "Cursor" }).click();
  const copyAria = await card
    .locator('button[aria-label="Copy config"], button[aria-label="Copy command"]')
    .first()
    .getAttribute("aria-label");
  log("A4", `connect-card copy aria after Cursor=${copyAria}`);
  await home.screenshot({ path: join(snapDir, "01-home-connect-desktop-1280x900.png") });
  await home.setViewportSize({ width: 375, height: 812 });
  await home.screenshot({ path: join(snapDir, "02-home-connect-mobile-375x812.png") });
  await home.close();

  log("B1", `GET ${base}/tools?type=mcp`);
  const tools = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await tools.goto(`${base}/tools?type=mcp`, wait);
  const cardCount = await tools.locator(".tool-card").count();
  log("B2", `tools list cards=${cardCount}`);

  const addUrl = `${base}/tools?type=mcp&selected=okx-agent-trade-kit&intent=add-mcp`;
  log("B3", `GET ${addUrl}`);
  await tools.goto(addUrl, wait);
  await tools.waitForSelector(".install-guide-panel", { timeout: 15000 });
  const panel = tools.locator(".install-guide-panel").first();
  log(
    "B4",
    `add-mode markers addMode=${await tools.locator(".add-mode").count()} progress=${await tools.locator(".install-progress").count()}`,
  );
  await tools.screenshot({ path: join(snapDir, "03-add-mode-desktop-1280x900.png") });

  log("B5", "click Claude platform in install guide");
  await panel.locator(".install-platform-btn", { hasText: "Claude" }).click();
  log(
    "B6",
    `claude pressed=${await panel.locator(".install-platform-btn", { hasText: "Claude" }).getAttribute("aria-pressed")}`,
  );
  log(
    "B7",
    `copy buttons=${await panel.locator('[aria-label="Copy config"], [aria-label="Copy command"]').count()}`,
  );

  await tools.setViewportSize({ width: 375, height: 812 });
  await tools.screenshot({ path: join(snapDir, "04-add-mode-mobile-375x812.png") });
  await tools.close();

  log("C1", `GET ${base}/compare?tools=okx-agent-trade-kit`);
  const compare = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await compare.goto(`${base}/compare?tools=okx-agent-trade-kit`, wait);
  const addLink = compare.locator(".add-mcp-inline-btn").first();
  const href = await addLink.getAttribute("href");
  log("C2", `compare AddMcpAction href=${href}`);
  await addLink.click();
  await compare.waitForSelector(".install-guide-panel", { timeout: 15000 });
  log("C3", `navigated to ${compare.url()}`);
  await compare.screenshot({ path: join(snapDir, "05-compare-add-mode-desktop-1280x900.png") });
  await compare.close();

  log("D1", `GET ${base}/toolkit`);
  const toolkit = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  const tkResp = await toolkit.goto(`${base}/toolkit`, wait);
  log("D2", `toolkit status=${tkResp?.status()}`);
  await toolkit.screenshot({ path: join(snapDir, "06-toolkit-desktop-1280x900.png") });
  await toolkit.close();

  if (platformCount !== 3) throw new Error(`expected 3 connect platforms, got ${platformCount}`);
  if (!href?.includes("intent=add-mcp")) throw new Error("compare href missing intent=add-mcp");
  if (!href?.includes("compare_tools=")) throw new Error("compare href missing compare_tools");
  log("RESULT", "INTERACTIVE_PASS");
} catch (e) {
  log("RESULT", `INTERACTIVE_FAIL: ${e.message}`);
  process.exitCode = 1;
} finally {
  await browser.close();
  writeFileSync(join(scratch, "mcp-add-interactive-transcript.log"), transcript.join("\n") + "\n");
}